//! macOS text-ask vertical slice.
//!
//! The first honest macOS product flow: configure the legacy AI bridge
//! (`ai_base_url` / `ai_bearer`) in `MacAiSetupWindow`, type a question in
//! the shared `TextAskWindow`, and show the real answer in the shared
//! `TileWindow`. Exactly one reusable instance of each window lives here;
//! the AppKit status item, the singleton guard, and the floating behavior
//! stay owned by the macOS main in `overlay_host.rs`.
//!
//! The request path is the portable live-answer API
//! `overlay_backend::ai::complete_with_usage_endpoint` on an owned Tokio
//! runtime — deliberately NOT the Windows streaming pipeline.
//!
//! Security: the bearer token is masked in the UI, never logged, never
//! populated from the stored config, and cleared on every Save attempt and
//! on Cancel. A failed request renders one fixed generic message — no URL,
//! host, token, or raw backend error ever reaches the tile or the log.

use overlay_backend::events::{TileKind, TileSpec};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::ui;

/// Non-streaming live-answer budget — the Windows `AI_MAX_TOKENS` value for
/// the same one-shot ask class (not the 4096 streaming cap).
const ANSWER_MAX_TOKENS: u32 = 600;
/// Tile title cap so a long question cannot push the chrome buttons off-screen.
const TITLE_MAX_CHARS: usize = 60;
/// Copy confirmation window — the same 1.5 s the Windows tile copy flashes.
const COPY_FEEDBACK_MS: u64 = 1500;

/// Status codes rendered by `MacAiSetupWindow` (the words stay in @tr there).
const STATUS_MISSING: i32 = 0;
const STATUS_READY: i32 = 1;
const STATUS_CHECK_FIELDS: i32 = 2;
const STATUS_SAVED: i32 = 3;
const STATUS_SAVE_FAILED: i32 = 4;

/// http(s) + a non-empty whitespace-free authority is the only URL rule this
/// slice enforces — no URL-parser dependency for one validation.
fn url_ready(url: &str) -> bool {
    let url = url.trim();
    let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    // Authority = everything before the first path/query/fragment separator.
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    !authority.is_empty() && !authority.chars().any(char::is_whitespace)
}

/// The stored config can answer a question only with a valid URL, a token, and
/// a model (the slice has no model field — an empty one would send nothing).
/// Read by the macOS main against its single startup config load.
pub(super) fn bridge_ready(cfg: &overlay_backend::config::Config) -> bool {
    url_ready(&cfg.ai_base_url)
        && !cfg.ai_bearer.trim().is_empty()
        && !cfg.ai_model.trim().is_empty()
}

/// Optional local raw-PCM STT field: empty keeps the stored STT config
/// untouched; non-empty must normalize to a usable service base URL.
fn local_stt_ready(typed: &str) -> bool {
    let trimmed = typed.trim();
    trimmed.is_empty() || overlay_backend::stt::normalize_uap_base_url(trimmed).is_some()
}

/// Write a NON-EMPTY validated local STT URL into the config and select the
/// explicit "uap" provider. Empty input changes nothing — the stored STT
/// provider/config survive exactly as they are, and no unrelated field is
/// touched.
fn apply_local_stt_url(cfg: &mut overlay_backend::config::Config, typed: &str) {
    let trimmed = typed.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Some(base) = overlay_backend::stt::normalize_uap_base_url(trimmed) {
        cfg.stt_whisper_url = base;
        cfg.stt_provider = "uap".to_string();
    }
}

/// The stored URL is shown ONLY while it is actually the active STT backend —
/// the ordinary whisper default never prefills this field.
fn local_stt_display_url(cfg: &overlay_backend::config::Config) -> String {
    if cfg.stt_provider == "uap" {
        cfg.stt_whisper_url.trim().to_string()
    } else {
        String::new()
    }
}

/// Trim the bridge answer; `None` = nothing usable came back.
fn final_answer(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Asking placeholder copy. Tile-model strings are Rust-built (not @tr), so
/// the localization lives here next to the other fixed tile copy.
fn asking_copy(is_ru: bool) -> &'static str {
    if is_ru {
        "Запрос к AI…"
    } else {
        "Asking AI…"
    }
}

/// The ONE failure message: fixed, generic, localized, secret-free.
fn ask_failure_copy(is_ru: bool) -> &'static str {
    if is_ru {
        "Запрос к AI-мосту не удался. Проверьте настройки моста и повторите попытку."
    } else {
        "The AI bridge request failed. Check the bridge settings and try again."
    }
}

fn title_from_question(question: &str) -> String {
    let trimmed = question.trim();
    let mut title: String = trimmed.chars().take(TITLE_MAX_CHARS).collect();
    if trimmed.chars().count() > TITLE_MAX_CHARS {
        title.push('…');
    }
    title
}

/// Parse a completed answer into the tile's `MarkdownBlock` rows.
fn md_blocks(source: &str) -> Vec<ui::MarkdownBlock> {
    slint_replay::markdown::parse(source)
        .into_iter()
        .map(|block| ui::MarkdownBlock {
            kind: block.kind,
            text: SharedString::from(block.text),
            display_text: SharedString::from(block.display_text),
            lang: SharedString::from(block.lang),
            marked: false,
        })
        .collect()
}

fn blocks_model(blocks: Vec<ui::MarkdownBlock>) -> ModelRc<ui::MarkdownBlock> {
    ModelRc::new(VecModel::from(blocks))
}

fn single_paragraph(text: &str) -> ModelRc<ui::MarkdownBlock> {
    blocks_model(vec![ui::MarkdownBlock {
        kind: slint_replay::markdown::kind::PARAGRAPH,
        text: SharedString::from(text),
        display_text: SharedString::from(text),
        lang: SharedString::default(),
        marked: false,
    }])
}

fn floating(window: &slint::Window) {
    if let Err(error) = slint_replay::native::window::configure_floating(window) {
        slint_replay::logging::line(&format!(
            "[macos] floating-window configuration failed: {error}"
        ));
    }
}

fn begin_native_drag(window: &slint::Window) {
    if let Err(error) = slint_replay::native::window::begin_drag(window) {
        slint_replay::logging::line(&format!("[macos] native drag failed: {error}"));
    }
}

/// Raise a shown window key and front so a reopen while another app is
/// active regains keyboard focus; the failure is diagnostic-only.
fn raise(window: &slint::Window) {
    if let Err(error) = slint_replay::native::window::raise_key_front(window) {
        slint_replay::logging::line(&format!("[macos] raise to key/front failed: {error}"));
    }
}

/// Validate + persist the setup form. A blank token on a later edit keeps the
/// stored token; on a clean install it is a validation failure. A non-empty
/// local STT URL must normalize; an empty one keeps the stored STT config.
fn save_bridge_setup(window: &ui::MacAiSetupWindow) {
    let url = window.get_bridge_url().trim().to_string();
    let token = window.get_token_input().trim().to_string();
    let local_stt = window.get_local_stt_url().trim().to_string();
    // The typed token never lingers in the field: cleared BEFORE validation
    // and persistence, on every Save attempt, whatever the outcome.
    window.set_token_input(SharedString::default());
    let mut cfg = overlay_backend::config::load();
    let has_stored_token = !cfg.ai_bearer.trim().is_empty();
    if !url_ready(&url) || (token.is_empty() && !has_stored_token) || !local_stt_ready(&local_stt) {
        window.set_status_kind(STATUS_CHECK_FIELDS);
        return;
    }
    cfg.ai_base_url = url;
    if !token.is_empty() {
        cfg.ai_bearer = token;
    }
    // The slice has no model field; an empty stored model would send an
    // empty `model` to the bridge. Restore the portable default instead.
    if cfg.ai_model.trim().is_empty() {
        cfg.ai_model = overlay_backend::config::Config::defaults().ai_model;
    }
    apply_local_stt_url(&mut cfg, &local_stt);
    if overlay_backend::config::save(&cfg).is_err() {
        window.set_status_kind(STATUS_SAVE_FAILED);
        return;
    }
    window.set_token_stored(true);
    window.set_status_kind(STATUS_SAVED);
    slint_replay::logging::line("[macos] AI bridge setup saved");
}

/// The one reusable TextAskWindow + TileWindow + MacAiSetupWindow set.
pub(super) struct TextAskSlice {
    text_ask: RefCell<Option<ui::TextAskWindow>>,
    tile: RefCell<Option<ui::TileWindow>>,
    setup: RefCell<Option<ui::MacAiSetupWindow>>,
    /// True once the event loop runs (NSWindows exist); gates `floating`
    /// and the direct key/front raise.
    live: Cell<bool>,
    /// Monotonic ask counter; a stale completion never overwrites a newer ask.
    generation: Arc<AtomicU64>,
    runtime: tokio::runtime::Runtime,
}

impl TextAskSlice {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            text_ask: RefCell::new(None),
            tile: RefCell::new(None),
            setup: RefCell::new(None),
            live: Cell::new(false),
            generation: Arc::new(AtomicU64::new(0)),
            runtime,
        })
    }

    /// Called from the startup timer once NSWindows are guaranteed: applies
    /// floating behavior and raises every slice window created before `run()`.
    pub(super) fn mark_live(&self) {
        self.live.set(true);
        if let Some(window) = self.text_ask.borrow().as_ref() {
            floating(window.window());
            raise(window.window());
        }
        if let Some(window) = self.tile.borrow().as_ref() {
            floating(window.window());
            raise(window.window());
        }
        if let Some(window) = self.setup.borrow().as_ref() {
            floating(window.window());
            raise(window.window());
        }
    }

    /// Ask chip: open the typed-question window, or route to setup while the
    /// bridge is incomplete (a doomed request helps nobody). Takes the `Rc`
    /// because the submit handler needs an owned slice reference.
    pub(super) fn open_text_ask(slice: &Rc<Self>) {
        if !bridge_ready(&overlay_backend::config::load()) {
            slice.open_setup();
            return;
        }
        if slice.text_ask.borrow().is_none() {
            let window = match ui::TextAskWindow::new() {
                Ok(window) => window,
                Err(error) => {
                    slint_replay::logging::line(&format!(
                        "[macos] text-ask window creation failed: {error}"
                    ));
                    return;
                }
            };
            Self::wire_text_ask(slice, &window);
            *slice.text_ask.borrow_mut() = Some(window);
        }
        if let Some(window) = slice.text_ask.borrow().as_ref() {
            let _ = window.show();
            if slice.live.get() {
                floating(window.window());
                raise(window.window());
            }
        }
    }

    /// AI setup chip + automatic first-run surface. Reopening refreshes the
    /// URL/status from the portable config; the token field is never seeded.
    /// The optional local STT URL is shown ONLY while "uap" is the active
    /// STT provider — the ordinary whisper default never prefills it.
    pub(super) fn open_setup(&self) {
        let cfg = overlay_backend::config::load();
        if self.setup.borrow().is_none() {
            let window = match ui::MacAiSetupWindow::new() {
                Ok(window) => window,
                Err(error) => {
                    slint_replay::logging::line(&format!(
                        "[macos] setup window creation failed: {error}"
                    ));
                    return;
                }
            };
            self.wire_setup(&window);
            *self.setup.borrow_mut() = Some(window);
        }
        if let Some(window) = self.setup.borrow().as_ref() {
            window.set_bridge_url(SharedString::from(cfg.ai_base_url.trim()));
            // The stored token is NEVER copied into the field.
            window.set_token_input(SharedString::default());
            window.set_token_stored(!cfg.ai_bearer.trim().is_empty());
            window.set_local_stt_url(SharedString::from(local_stt_display_url(&cfg)));
            window.set_status_kind(if bridge_ready(&cfg) {
                STATUS_READY
            } else {
                STATUS_MISSING
            });
            let _ = window.show();
            if self.live.get() {
                floating(window.window());
                raise(window.window());
            }
        }
    }

    fn wire_text_ask(slice: &Rc<Self>, window: &ui::TextAskWindow) {
        {
            let weak = window.as_weak();
            let submit_slice = slice.clone();
            window.on_submitted(move |question| {
                if let Some(window) = weak.upgrade() {
                    let _ = window.hide();
                }
                submit_slice.start_ask(&question);
            });
        }
        {
            let weak = window.as_weak();
            window.on_cancelled(move || {
                if let Some(window) = weak.upgrade() {
                    // A cancelled draft never survives into the next open.
                    window.set_query(SharedString::default());
                    let _ = window.hide();
                }
            });
        }
        {
            let weak = window.as_weak();
            window.on_drag_start_requested(move || {
                if let Some(window) = weak.upgrade() {
                    begin_native_drag(window.window());
                }
            });
        }
    }

    fn wire_setup(&self, window: &ui::MacAiSetupWindow) {
        {
            let weak = window.as_weak();
            window.on_save_clicked(move || {
                if let Some(window) = weak.upgrade() {
                    save_bridge_setup(&window);
                }
            });
        }
        {
            let weak = window.as_weak();
            window.on_cancel_clicked(move || {
                if let Some(window) = weak.upgrade() {
                    // A typed-but-unsaved token is discarded, never kept on screen.
                    window.set_token_input(SharedString::default());
                    let _ = window.hide();
                }
            });
        }
        {
            let weak = window.as_weak();
            window.on_drag_start_requested(move || {
                if let Some(window) = weak.upgrade() {
                    begin_native_drag(window.window());
                }
            });
        }
    }

    /// The answer surface. Created once, reused for every ask, hidden on close.
    fn ensure_tile(&self) -> bool {
        if self.tile.borrow().is_some() {
            return true;
        }
        let window = match ui::TileWindow::new() {
            Ok(window) => window,
            Err(error) => {
                slint_replay::logging::line(&format!(
                    "[macos] tile window creation failed: {error}"
                ));
                return false;
            }
        };
        window.set_sequence(1);
        {
            let weak = window.as_weak();
            window.on_close_clicked(move || {
                if let Some(window) = weak.upgrade() {
                    let _ = window.hide();
                }
            });
        }
        {
            let weak = window.as_weak();
            window.on_drag_start_requested(move || {
                if let Some(window) = weak.upgrade() {
                    begin_native_drag(window.window());
                }
            });
        }
        {
            // Main copy writes EXACTLY the remembered finalized answer — the
            // button is only visible while one exists (see `start_ask`).
            let weak = window.as_weak();
            window.on_copy_clicked(move || {
                let Some(tile) = weak.upgrade() else {
                    return;
                };
                // `select-text` already holds the tile's complete joined text
                // on Windows. The Mac slice reuses it as the exact hidden copy
                // source, avoiding a second cross-thread state holder.
                let text = tile.get_select_text();
                if text.is_empty() {
                    return;
                }
                if slint_replay::native::clipboard::set_text(text.as_str()).is_err() {
                    // Category only — the payload must never reach the log.
                    slint_replay::logging::line("[macos] tile answer copy failed");
                    return;
                }
                tile.set_copied(true);
                let flash = tile.as_weak();
                slint::Timer::single_shot(Duration::from_millis(COPY_FEEDBACK_MS), move || {
                    if let Some(tile) = flash.upgrade() {
                        tile.set_copied(false);
                    }
                });
            });
        }
        {
            // Per-code-block copy writes EXACTLY the callback's clean code
            // (block.text is fence-stripped) and flashes only that block.
            let weak = window.as_weak();
            window.on_copy_block_clicked(move |idx, code| {
                if code.is_empty() {
                    return;
                }
                if slint_replay::native::clipboard::set_text(code.as_str()).is_err() {
                    slint_replay::logging::line("[macos] code-block copy failed");
                    return;
                }
                let Some(tile) = weak.upgrade() else {
                    return;
                };
                tile.set_copied_block_index(idx);
                let flash = tile.as_weak();
                slint::Timer::single_shot(Duration::from_millis(COPY_FEEDBACK_MS), move || {
                    if let Some(tile) = flash.upgrade() {
                        // Clear only while THIS block still owns the check —
                        // a newer copy of another block moved the marker.
                        if tile.get_copied_block_index() == idx {
                            tile.set_copied_block_index(-1);
                        }
                    }
                });
            });
        }
        *self.tile.borrow_mut() = Some(window);
        true
    }

    /// Placeholder tile first, then the portable non-streaming completion off
    /// the Slint thread; the result returns through the event loop.
    fn start_ask(&self, question: &str) {
        let question = question.trim().to_string();
        if question.is_empty() {
            return;
        }
        let cfg = overlay_backend::config::load();
        if !bridge_ready(&cfg) {
            self.open_setup();
            return;
        }
        if !self.ensure_tile() {
            return;
        }
        let is_ru = cfg.ui_is_ru();
        let tile_weak = {
            let borrowed = self.tile.borrow();
            let Some(tile) = borrowed.as_ref() else {
                return;
            };
            tile.set_tile_title(SharedString::from(title_from_question(&question)));
            tile.set_source_label(SharedString::from("ai"));
            // The window is reused: an earlier auto tile may have left a
            // trigger badge behind — a manual ask shows none.
            tile.set_trigger_label(SharedString::default());
            tile.set_blocks(single_paragraph(asking_copy(is_ru)));
            // A new request invalidates the previous copy target until a
            // fresh finalized answer lands; hide the affordance meanwhile.
            tile.set_select_text(SharedString::default());
            tile.set_can_copy(false);
            tile.set_copied(false);
            let _ = tile.show();
            if self.live.get() {
                floating(tile.window());
                raise(tile.window());
            }
            tile.as_weak()
        };
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let generation_guard = self.generation.clone();
        // Explicit legacy bridge endpoint built ONLY from the three portable
        // fields — no provider routing, no structuring mode, no local server.
        let endpoint = overlay_backend::ai::AiEndpoint {
            protocol: overlay_backend::ai::AiProtocol::OpenAiCompatible,
            base_url: cfg.ai_base_url.trim().to_string(),
            bearer: cfg.ai_bearer.clone(),
            model: cfg.ai_model.clone(),
            reasoning_effort: None,
            is_local: false,
        };
        self.runtime.handle().spawn(async move {
            let messages = vec![overlay_backend::ai::ChatMessage {
                role: "user".to_string(),
                content: overlay_backend::ai::MessageContent::Text(question),
            }];
            let outcome = overlay_backend::ai::complete_with_usage_endpoint(
                &endpoint,
                messages,
                ANSWER_MAX_TOKENS,
            )
            .await;
            let _ = slint::invoke_from_event_loop(move || {
                // A newer ask superseded this one while it was in flight.
                if generation_guard.load(Ordering::Relaxed) != generation {
                    return;
                }
                let Some(tile) = tile_weak.upgrade() else {
                    return;
                };
                match outcome {
                    Ok((raw, _usage)) => match final_answer(&raw) {
                        Some(answer) => {
                            // Only a non-empty finalized answer remembers a
                            // copy target and shows the main copy button.
                            tile.set_blocks(blocks_model(md_blocks(&answer)));
                            tile.set_select_text(SharedString::from(answer));
                            tile.set_can_copy(true);
                        }
                        None => {
                            slint_replay::logging::line("[macos] text-ask answer was empty");
                            tile.set_blocks(single_paragraph(ask_failure_copy(is_ru)));
                        }
                    },
                    Err(_) => {
                        // Category only — the raw error can carry URLs/hosts.
                        slint_replay::logging::line("[macos] text-ask bridge request failed");
                        tile.set_blocks(single_paragraph(ask_failure_copy(is_ru)));
                    }
                }
            });
        });
    }

    /// Present ONE session-spawned tile (STT/auto answer already final — no
    /// AI call happens here) through the same single reusable TileWindow as
    /// manual asks. Main thread only: the macOS main drains the spawner
    /// queue inside a Slint Timer.
    ///
    /// An empty answer logs category-only and leaves the tile untouched, so
    /// an in-flight manual ask keeps its placeholder and completion path.
    pub(super) fn present_auto_tile(&self, spec: TileSpec, kind: TileKind) {
        let Some(answer) = final_answer(&spec.answer) else {
            slint_replay::logging::line("[macos] auto tile carried an empty answer");
            return;
        };
        if !self.ensure_tile() {
            return;
        }
        // Bump the SAME generation counter start_ask uses, BEFORE populating:
        // a stale manual completion still in flight must not overwrite this
        // tile (its guard compares against the counter it captured).
        self.generation.fetch_add(1, Ordering::Relaxed);
        let cfg = overlay_backend::config::load();
        let borrowed = self.tile.borrow();
        let Some(tile) = borrowed.as_ref() else {
            return;
        };
        tile.set_tile_title(SharedString::from(title_from_question(&spec.question)));
        tile.set_source_label(SharedString::from(kind.as_journal_tag()));
        tile.set_blocks(blocks_model(md_blocks(&answer)));
        tile.set_select_text(SharedString::from(answer));
        tile.set_can_copy(true);
        tile.set_copied(false);
        tile.set_copied_block_index(-1);
        // The Windows trigger-badge mapping, first highlight only: keyword
        // hits orange, everything else cyan. No highlight = no badge (the
        // chrome renders it only while the label is non-empty).
        if let Some(first) = spec.highlights.first() {
            tile.set_trigger_label(SharedString::from(first.as_str()));
            tile.set_trigger_color(if first.starts_with("keyword") {
                slint::Color::from_rgb_u8(0xfb, 0x92, 0x3c)
            } else {
                slint::Color::from_rgb_u8(0x6c, 0xcf, 0xff)
            });
        } else {
            tile.set_trigger_label(SharedString::default());
            // Theme's default accent (scheme 0) — the badge is hidden while
            // the label is empty, so this only resets the remembered color.
            tile.set_trigger_color(slint::Color::from_rgb_u8(0x4c, 0x8d, 0xff));
        }
        tile.set_body_opacity(cfg.tile_body_opacity);
        let _ = tile.show();
        if self.live.get() {
            floating(tile.window());
            raise(tile.window());
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn url_ready_accepts_only_http_endpoints() {
        assert!(url_ready("http://127.0.0.1:18902/v1"));
        assert!(url_ready("https://bridge.example/v1"));
        assert!(url_ready("http://bridge.example"));
        assert!(url_ready("  http://bridge.example/v1  "));
        assert!(!url_ready(""));
        assert!(!url_ready("   "));
        assert!(!url_ready("ftp://bridge.example/v1"));
        assert!(!url_ready("bridge.example:18902/v1"));
    }

    #[test]
    fn url_ready_rejects_scheme_only_and_whitespace_hosts() {
        assert!(!url_ready("http://"));
        assert!(!url_ready("https://"));
        assert!(!url_ready("http:///v1"));
        assert!(!url_ready("http:// /v1"));
        assert!(!url_ready("http://host with space/v1"));
        assert!(!url_ready("https://bridge.example\t/v1"));
    }

    #[test]
    fn bridge_ready_needs_url_token_and_model() {
        let mut cfg = overlay_backend::config::Config::defaults();
        assert!(!bridge_ready(&cfg));
        cfg.ai_base_url = "http://127.0.0.1:18902/v1".into();
        assert!(!bridge_ready(&cfg));
        cfg.ai_bearer = "   ".into();
        assert!(!bridge_ready(&cfg));
        cfg.ai_bearer = "secret".into();
        // Config::defaults() ships a non-empty ai_model; an empty one fails.
        assert!(bridge_ready(&cfg));
        cfg.ai_model = "  ".into();
        assert!(!bridge_ready(&cfg));
    }

    #[test]
    fn final_answer_trims_and_rejects_empty() {
        assert_eq!(
            final_answer("  answer text  ").as_deref(),
            Some("answer text")
        );
        assert!(final_answer("").is_none());
        assert!(final_answer("  \n\t ").is_none());
    }

    #[test]
    fn fixed_copies_are_localized_and_leak_nothing() {
        for (en, ru) in [
            (asking_copy(false), asking_copy(true)),
            (ask_failure_copy(false), ask_failure_copy(true)),
        ] {
            assert!(!en.is_empty());
            assert!(!ru.is_empty());
            assert_ne!(en, ru);
            for copy in [en, ru] {
                let lower = copy.to_lowercase();
                assert!(!lower.contains("http://"));
                assert!(!lower.contains("https://"));
                assert!(!lower.contains("192.168."));
                assert!(!lower.contains("100."));
            }
        }
    }

    #[test]
    fn title_from_question_is_capped() {
        assert_eq!(title_from_question("  short question  "), "short question");
        let long = "a".repeat(TITLE_MAX_CHARS + 40);
        let title = title_from_question(&long);
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS + 1);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn answers_map_to_markdown_blocks() {
        let blocks = md_blocks("# Heading\n\nplain answer");
        assert!(blocks.len() >= 2);
        assert_eq!(blocks[0].kind, slint_replay::markdown::kind::H1);
        let paragraph = blocks
            .iter()
            .find(|b| b.kind == slint_replay::markdown::kind::PARAGRAPH)
            .expect("paragraph block");
        assert_eq!(paragraph.text.as_str(), "plain answer");
    }

    #[test]
    fn local_stt_ready_accepts_empty_and_valid_urls_only() {
        // Empty / whitespace = "keep the stored STT config" — always OK.
        assert!(local_stt_ready(""));
        assert!(local_stt_ready("   "));
        assert!(local_stt_ready("http://127.0.0.1:9000"));
        assert!(local_stt_ready("http://127.0.0.1:9000/v1"));
        assert!(local_stt_ready("  http://127.0.0.1:9000/v1/  "));
        assert!(!local_stt_ready("ftp://127.0.0.1:9000"));
        assert!(!local_stt_ready("127.0.0.1:9000"));
        assert!(!local_stt_ready("http://"));
        assert!(!local_stt_ready("http://host with space"));
    }

    #[test]
    fn apply_local_stt_url_normalizes_and_selects_uap() {
        let mut cfg = overlay_backend::config::Config::defaults();
        cfg.stt_provider = "cloud".into();
        apply_local_stt_url(&mut cfg, "  http://127.0.0.1:9000/v1/  ");
        assert_eq!(cfg.stt_provider, "uap");
        assert_eq!(cfg.stt_whisper_url, "http://127.0.0.1:9000");
    }

    #[test]
    fn apply_local_stt_url_empty_keeps_stored_stt_config() {
        let mut cfg = overlay_backend::config::Config::defaults();
        cfg.stt_provider = "whisper".into();
        cfg.stt_whisper_url = "http://127.0.0.1:8081/v1".into();
        apply_local_stt_url(&mut cfg, "   ");
        assert_eq!(cfg.stt_provider, "whisper");
        assert_eq!(cfg.stt_whisper_url, "http://127.0.0.1:8081/v1");
    }

    #[test]
    fn local_stt_display_url_only_for_active_uap_provider() {
        let mut cfg = overlay_backend::config::Config::defaults();
        // The whisper DEFAULT URL must never prefill the optional field.
        assert_eq!(cfg.stt_provider, "cloud");
        assert!(!cfg.stt_whisper_url.is_empty());
        assert_eq!(local_stt_display_url(&cfg), "");

        cfg.stt_provider = "whisper".into();
        assert_eq!(local_stt_display_url(&cfg), "");

        cfg.stt_provider = "uap".into();
        cfg.stt_whisper_url = " http://127.0.0.1:9000 ".into();
        assert_eq!(local_stt_display_url(&cfg), "http://127.0.0.1:9000");
    }
}
