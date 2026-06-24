//! Pick the TEXT source for a RETROSPECTIVE meeting summary (ТЗ3).
//!
//! The live bar Summary button summarizes the in-memory transcript; the archive
//! "↻ Summary" re-transcribes saved audio. For a session that has NEITHER a
//! transcript in hand NOR (re-)transcribable audio, these helpers recover a
//! summary input from what IS still on disk, so an old / audio-less session can
//! be summarized after the fact.
//!
//! Caller priority: saved catalog transcript → (re-STT of audio, the existing
//! `re_transcribe` path) → journal `ai_request` prompts. Every helper yields the
//! `Vec<TranscriptLine>` that `runtime::run_meeting_summary` already consumes —
//! NOTHING here touches that flow; these are additive INPUT sources only.

use crate::audio::{AudioSource, TranscriptLine};
use crate::persistence::Store;
use std::path::Path;

/// A session id is a journal file stem; reject path-escapes before we join it.
fn is_plain_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && !session_id.contains('/')
        && !session_id.contains('\\')
        && !session_id.contains("..")
}

/// The saved transcript from the catalog — the cheapest, freshest source (no
/// re-STT, no AI cost). `None` when the session has no indexed utterances.
#[must_use]
pub fn from_catalog(store: &Store, session_id: &str) -> Option<Vec<TranscriptLine>> {
    let utts = store.session_utterances(session_id).ok()?;
    if utts.is_empty() {
        return None;
    }
    Some(
        utts.into_iter()
            .map(|u| TranscriptLine {
                source: if u.source == "mic" {
                    AudioSource::Mic
                } else {
                    AudioSource::System
                },
                text: u.text,
                timestamp_ms: u.unix_ms.max(0) as u64,
            })
            .collect(),
    )
}

/// Last-resort source for an old session with no transcript AND no audio: the
/// dialogue CONTEXT sent to the AI in each `ai_request.user_prompt`. Every ask
/// carried a sliding window, so consecutive prompts overlap heavily — we dedup
/// at the LINE level (first-seen order) to reconstruct the unique text. Returns
/// ONE synthesized line (the summary model only needs the text body). `None` if
/// the journal is missing / has no usable prompts.
#[must_use]
pub fn from_jsonl_prompts(session_id: &str) -> Option<Vec<TranscriptLine>> {
    let dir = crate::journal::sessions_dir().ok()?;
    from_jsonl_prompts_in(&dir, session_id)
}

/// Pure variant (test seam): read `<dir>/<id>.jsonl` instead of the real
/// sessions dir.
fn from_jsonl_prompts_in(dir: &Path, session_id: &str) -> Option<Vec<TranscriptLine>> {
    if !is_plain_id(session_id) {
        return None;
    }
    let content = std::fs::read_to_string(dir.join(format!("{session_id}.jsonl"))).ok()?;
    let mut seen = std::collections::HashSet::new();
    let mut lines: Vec<String> = Vec::new();
    for raw in content.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
            continue; // skip a torn / corrupt journal line
        };
        if v.get("kind").and_then(serde_json::Value::as_str) != Some("ai_request") {
            continue;
        }
        let Some(prompt) = v.get("user_prompt").and_then(serde_json::Value::as_str) else {
            continue;
        };
        for l in prompt.lines() {
            let t = l.trim();
            if !t.is_empty() && seen.insert(t.to_owned()) {
                lines.push(t.to_owned());
            }
        }
    }
    if lines.is_empty() {
        return None;
    }
    Some(vec![TranscriptLine {
        source: AudioSource::System,
        text: lines.join("\n"),
        timestamp_ms: 0,
    }])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::persistence::{Session, Utterance};

    fn sess(id: &str) -> Session {
        Session {
            id: id.into(),
            journal_path: "x".into(),
            started_at_ms: Some(1000),
            finished_at_ms: Some(2000),
            status: "completed".into(),
            ai_model: None,
            transcript_lines: 0,
            ai_turns_count: 0,
            total_cost_microcents: 0,
            indexed_at_ms: 0,
        }
    }

    #[test]
    fn from_catalog_maps_utterances_else_none() {
        let mut store = Store::open_in_memory().unwrap();
        let utts = vec![
            Utterance {
                session_id: "S1".into(),
                unix_ms: 1100,
                source: "mic".into(),
                text: "привет".into(),
            },
            Utterance {
                session_id: "S1".into(),
                unix_ms: 1200,
                source: "system".into(),
                text: "здравствуйте".into(),
            },
        ];
        store.replace_session(&sess("S1"), &utts, &[]).unwrap();

        let lines = from_catalog(&store, "S1").unwrap();
        assert_eq!(lines.len(), 2);
        assert!(matches!(lines[0].source, AudioSource::Mic));
        assert_eq!(lines[0].text, "привет");
        assert!(matches!(lines[1].source, AudioSource::System));
        assert!(from_catalog(&store, "nope").is_none()); // absent → None
    }

    #[test]
    fn from_jsonl_dedups_overlapping_windows_and_guards() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let jsonl = [
            r#"{"kind":"session_start","unix_ms":1}"#,
            r#"{"kind":"ai_request","unix_ms":2,"user_prompt":"Q1 что такое хеш-таблица\nконтекст A"}"#,
            r#"{"kind":"ai_request","unix_ms":3,"user_prompt":"контекст A\nQ2 сложность поиска"}"#,
            r#"{"kind":"ai_response","unix_ms":4,"text":"ответ — не источник"}"#,
        ]
        .join("\n");
        std::fs::write(dir.join("S1.jsonl"), jsonl).unwrap();

        let lines = from_jsonl_prompts_in(dir, "S1").unwrap();
        assert_eq!(lines.len(), 1);
        let text = &lines[0].text;
        assert_eq!(text.matches("контекст A").count(), 1); // overlap deduped
        assert!(text.contains("Q1 что такое хеш-таблица"));
        assert!(text.contains("Q2 сложность поиска"));
        assert!(!text.contains("ответ — не источник")); // ai_response is NOT a source

        assert!(from_jsonl_prompts_in(dir, "../escape").is_none()); // traversal guard
        assert!(from_jsonl_prompts_in(dir, "missing").is_none()); // no journal → None
    }
}
