//! Runtime event sink trait — Phase B2 scaffold.
//!
//! `RuntimeEvents` decouples runtime.rs / pipeline code from any
//! specific UI framework. Today it has two implementations expected:
//!
//! - `TauriEvents` in `src-tauri/src/lib.rs` — wraps an `AppHandle`,
//!   forwards `emit` to `app.emit(channel, payload)`.
//! - `SlintEvents` in `slint-experiment/src/bin/overlay_host.rs` —
//!   wraps a channel + the Slint Weak<MainWindow>, forwards via
//!   `slint::invoke_from_event_loop` to update UI properties.
//!
//! Phase B2 proper (not yet shipped) ports runtime.rs's ~9
//! AppHandle-taking public fns to take `Arc<dyn RuntimeEvents>`
//! instead, removing the last Tauri coupling from the backend.
//!
//! Today the trait is only consumed by `Noop` (default impl) — real
//! consumers land with each runtime fn that gets ported.

use std::sync::Arc;

/// Sink for events the backend pipeline emits during a session.
/// Channel names match the existing Tauri event channels so the
/// Tauri-side implementation is a 1:1 forward; the Slint-side
/// implementation maps channel names to property setters / Rust
/// callbacks.
pub trait RuntimeEvents: Send + Sync {
    /// Emit an event with a JSON payload to the channel.
    /// Implementations should NOT block. Channel names are stable
    /// identifiers like "transcript:line" / "ai:answer-chunk" /
    /// "health:update" / "session:started" / "session:stopped".
    fn emit(&self, channel: &str, payload: serde_json::Value);

    /// Spawn a tile window with the given spec. Returns the assigned
    /// tile label/identifier (e.g. "tile-42") for downstream
    /// `pin_tile` / `close_tile` calls. For headless impls returns
    /// a synthetic id.
    fn spawn_tile(&self, spec: TileSpec) -> String;
}

/// Description of a tile to spawn — replaces the React-side
/// `tile::spawn_tile_with_stealth(question, answer)` Tauri call.
#[derive(Debug, Clone)]
pub struct TileSpec {
    pub question: String,
    pub answer: String,
    /// "ai" | "kb" | "snippet" | "translate" | "reload" — for the
    /// chrome source-label + journal categorization.
    pub source: String,
    /// True when the tile carries a translation rather than an
    /// AI-generated answer (chrome adds the 🌐 glyph).
    pub is_translation: bool,
}

/// Headless no-op events sink — for backend tests + situations
/// where the caller doesn't care about emit (e.g. one-shot AI
/// completion via `ai::complete_with_usage` doesn't need the sink
/// at all). Logs emits to `log::debug!` for diagnostic.
#[derive(Default, Debug, Clone, Copy)]
pub struct Noop;

impl RuntimeEvents for Noop {
    fn emit(&self, channel: &str, _payload: serde_json::Value) {
        log::debug!("[runtime-events:noop] {channel}");
    }
    fn spawn_tile(&self, spec: TileSpec) -> String {
        log::debug!(
            "[runtime-events:noop] spawn_tile source={} q.len={} a.len={}",
            spec.source,
            spec.question.len(),
            spec.answer.len()
        );
        format!("noop-tile-{}", spec.question.len())
    }
}

/// Convenience: `Arc<Noop>` ready to pass into APIs that take
/// `Arc<dyn RuntimeEvents>`.
#[must_use]
pub fn noop() -> Arc<dyn RuntimeEvents> {
    Arc::new(Noop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_emit_does_not_panic_on_arbitrary_channels() {
        let sink: Arc<dyn RuntimeEvents> = noop();
        sink.emit("transcript:line", serde_json::json!({"text": "hello"}));
        sink.emit("ai:answer-chunk", serde_json::json!({"delta": ""}));
        sink.emit("health:update", serde_json::Value::Null);
    }

    #[test]
    fn noop_spawn_tile_returns_stable_id_per_question_len() {
        let sink: Arc<dyn RuntimeEvents> = noop();
        let id1 = sink.spawn_tile(TileSpec {
            question: "abc".into(),
            answer: String::new(),
            source: "ai".into(),
            is_translation: false,
        });
        let id2 = sink.spawn_tile(TileSpec {
            question: "abc".into(),
            answer: String::new(),
            source: "kb".into(),
            is_translation: false,
        });
        // Stable-per-question-len: both have len=3 → same id (noop is
        // deterministic by design; real impls assign unique ids).
        assert_eq!(id1, id2);
        assert!(id1.starts_with("noop-tile-"));
    }
}
