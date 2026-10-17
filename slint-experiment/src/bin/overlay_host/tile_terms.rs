//! Terms under a live answer and the definitions behind them.
//!
//! A fast answer ends with a line of terms (`overlay_backend::runtime::
//! split_terms_line`). The tile shows them as buttons; a click asks the model
//! for a short definition, which ends with terms of its own, so the listener
//! walks a chain of definitions: Answer › StatefulSet › PVC. The chain is the
//! tile's `term-path`; a definition already fetched in this session is shown
//! without asking again.

use crate::ui::{TermRow, TileWindow};
use overlay_backend::ai;
use overlay_backend::config::SharedConfig;
use overlay_backend::runtime::{
    build_term_prompts, hide_terms_line_while_streaming, new_terms, split_terms_line, term_rows,
    TERM_DEFINITION_MAX_TOKENS,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Width the term buttons are laid out in: a default tile less its padding.
const TERM_ROW_BUDGET_PX: f32 = 400.0;
/// How often a definition being written is redrawn.
const TERM_STREAM_PUSH: Duration = Duration::from_millis(100);
/// Definitions kept for the session; the oldest half goes when it is full.
const TERM_CACHE_MAX: usize = 200;

/// A definition and the terms it names, by lowercase term.
static DEFINITIONS: Mutex<Vec<(String, String, Vec<String>)>> = Mutex::new(Vec::new());

fn cached(term: &str) -> Option<(String, Vec<String>)> {
    let key = term.to_lowercase();
    let cache = DEFINITIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, text, terms)| (text.clone(), terms.clone()))
}

fn remember(term: &str, text: &str, terms: &[String]) {
    let mut cache = DEFINITIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if cache.len() >= TERM_CACHE_MAX {
        cache.drain(..TERM_CACHE_MAX / 2);
    }
    let key = term.to_lowercase();
    cache.retain(|(k, _, _)| *k != key);
    cache.push((key, text.to_string(), terms.to_vec()));
}

pub(crate) fn rows_model(terms: &[String]) -> ModelRc<TermRow> {
    let rows: Vec<TermRow> = term_rows(terms, TERM_ROW_BUDGET_PX)
        .into_iter()
        .map(|row| TermRow {
            items: ModelRc::new(VecModel::from(
                row.into_iter().map(SharedString::from).collect::<Vec<_>>(),
            )),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn path_of(tile: &TileWindow) -> Vec<String> {
    tile.get_term_path().iter().map(|s| s.to_string()).collect()
}

fn error_text(is_ru: bool) -> &'static str {
    if is_ru {
        "Не удалось получить определение. Нажмите на термин ещё раз."
    } else {
        "Could not get the definition. Click the term again."
    }
}

/// Show the terms of `answer` under the tile's answer and wire the clicks.
/// `question` is what the answer replies to; definitions are asked in its
/// context.
pub(crate) fn wire_terms(
    tile: &TileWindow,
    terms: &[String],
    question: String,
    cfg: &SharedConfig,
    rt_handle: &tokio::runtime::Handle,
) {
    tile.set_answer_terms(rows_model(terms));
    tile.set_term_path(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));
    tile.set_term_definition(SharedString::new());
    tile.set_term_subterms(rows_model(&[]));
    tile.set_term_busy(false);
    if terms.is_empty() {
        return;
    }
    // Bumped by every open, back and close: a definition still being written
    // for an earlier term does not land on the one shown now.
    let generation = Arc::new(AtomicU64::new(0));
    let question = Arc::new(question);

    let open = {
        let weak = tile.as_weak();
        let cfg = cfg.clone();
        let rt_handle = rt_handle.clone();
        let generation = generation.clone();
        move |path: Vec<String>| {
            let Some(tile) = weak.upgrade() else { return };
            let Some((term, earlier)) = path.split_last() else {
                return;
            };
            let term = term.clone();
            let gen = generation.fetch_add(1, Ordering::SeqCst) + 1;
            tile.set_term_path(ModelRc::new(VecModel::from(
                path.iter().map(SharedString::from).collect::<Vec<_>>(),
            )));
            if let Some((text, terms)) = cached(&term) {
                tile.set_term_busy(false);
                tile.set_term_definition(SharedString::from(text));
                tile.set_term_subterms(rows_model(&new_terms(terms, &path)));
                return;
            }
            tile.set_term_busy(true);
            tile.set_term_definition(SharedString::new());
            tile.set_term_subterms(rows_model(&[]));
            let (endpoint, language, is_ru) = {
                let c = cfg.read();
                (
                    c.ai_endpoint(false),
                    c.response_language.clone(),
                    c.ui_is_ru(),
                )
            };
            let (system, user) = build_term_prompts(&term, earlier, &question, &language);
            let messages = vec![
                ai::ChatMessage {
                    role: "system".into(),
                    content: ai::MessageContent::Text(system),
                },
                ai::ChatMessage {
                    role: "user".into(),
                    content: ai::MessageContent::Text(user),
                },
            ];
            let weak = tile.as_weak();
            let generation = generation.clone();
            rt_handle.spawn(async move {
                let mut rx = ai::stream_chat_endpoint_plain(
                    endpoint,
                    messages,
                    TERM_DEFINITION_MAX_TOKENS,
                    true,
                );
                let mut text = String::new();
                let mut last_push: Option<Instant> = None;
                let outcome = loop {
                    match rx.recv().await {
                        Some(ai::AiEvent::Delta { text: delta }) => {
                            text.push_str(&delta);
                            if generation.load(Ordering::SeqCst) != gen {
                                return;
                            }
                            if last_push.is_none_or(|at| at.elapsed() >= TERM_STREAM_PUSH) {
                                last_push = Some(Instant::now());
                                let shown = hide_terms_line_while_streaming(&text);
                                let weak = weak.clone();
                                let generation = generation.clone();
                                let _ = slint::invoke_from_event_loop(move || {
                                    if generation.load(Ordering::SeqCst) != gen {
                                        return;
                                    }
                                    if let Some(t) = weak.upgrade() {
                                        t.set_term_definition(SharedString::from(shown));
                                    }
                                });
                            }
                        }
                        Some(ai::AiEvent::Done { .. }) => break !text.trim().is_empty(),
                        Some(ai::AiEvent::Error { message }) => {
                            log::warn!("term definition failed: {message}");
                            break false;
                        }
                        Some(_) => {}
                        None => break !text.trim().is_empty(),
                    }
                };
                let (body, terms) = split_terms_line(&text);
                if outcome {
                    remember(&term, &body, &terms);
                }
                let _ = slint::invoke_from_event_loop(move || {
                    if generation.load(Ordering::SeqCst) != gen {
                        return;
                    }
                    let Some(t) = weak.upgrade() else { return };
                    t.set_term_busy(false);
                    if outcome {
                        let path = path_of(&t);
                        t.set_term_definition(SharedString::from(body));
                        t.set_term_subterms(rows_model(&new_terms(terms, &path)));
                    } else {
                        t.set_term_definition(SharedString::from(error_text(is_ru)));
                    }
                });
            });
        }
    };
    let open = std::rc::Rc::new(open);

    {
        let weak = tile.as_weak();
        let open = open.clone();
        tile.on_term_opened(move |term, deeper| {
            let Some(tile) = weak.upgrade() else { return };
            let term = term.to_string();
            let mut path = if deeper { path_of(&tile) } else { Vec::new() };
            if path.last().is_some_and(|last| *last == term) {
                return;
            }
            path.push(term);
            open(path);
        });
    }
    {
        let weak = tile.as_weak();
        tile.on_term_crumb(move |index| {
            let Some(tile) = weak.upgrade() else { return };
            let mut path = path_of(&tile);
            let keep = usize::try_from(index).unwrap_or(0) + 1;
            if keep >= path.len() {
                return;
            }
            path.truncate(keep);
            open(path);
        });
    }
    {
        let weak = tile.as_weak();
        tile.on_term_closed(move || {
            generation.fetch_add(1, Ordering::SeqCst);
            let Some(tile) = weak.upgrade() else { return };
            tile.set_term_path(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));
            tile.set_term_definition(SharedString::new());
            tile.set_term_subterms(rows_model(&[]));
            tile.set_term_busy(false);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_is_remembered_by_its_term_in_any_case() {
        remember("Raft-test", "Алгоритм консенсуса.", &["кворум".to_string()]);
        let (text, terms) = cached("raft-TEST").unwrap();
        assert_eq!(text, "Алгоритм консенсуса.");
        assert_eq!(terms, ["кворум"]);
        remember("raft-test", "Новый текст.", &[]);
        assert_eq!(cached("Raft-test").unwrap().0, "Новый текст.");
    }
}
