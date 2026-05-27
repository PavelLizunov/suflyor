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
    ///
    /// **Soft-deprecated** — use `spawn_tile_full` for any new caller
    /// that needs monitor / stealth / kind. Kept for the current
    /// overlay-host callsites that don't yet pass those fields.
    fn spawn_tile(&self, spec: TileSpec) -> String;

    /// Phase B2 trait extension — spawn a tile with the full set of
    /// fields the Tauri-side `tile::spawn_tile_with_stealth(...)` call
    /// receives today. Required for all 8 runtime.rs tile-spawn sites
    /// when they port to take `Arc<dyn RuntimeEvents>` instead of
    /// `(AppHandle, SharedTiles)`.
    ///
    /// - `monitor`: which display to land on. Default impl ignores
    ///   and the Tauri side uses pick_monitor's portrait-aware logic.
    /// - `stealth`: when true, apply WDA_EXCLUDEFROMCAPTURE to the
    ///   spawned tile window (carries the session-wide stealth flag).
    /// - `kind`: discriminates Ai / Kb / Snippet / Translate / Reload
    ///   / Followup / Bookmark / Debrief — drives chrome glyph +
    ///   journal categorization.
    ///
    /// Returns Ok(tile_label) on success or Err(diagnostic) on
    /// failure (e.g. window creation failed). Default trait
    /// implementation forwards to `spawn_tile` ignoring the new
    /// fields — lets existing callers migrate incrementally.
    ///
    /// # Errors
    /// Implementations return Err with a human-readable diagnostic
    /// when tile window creation fails or the registry rejects.
    fn spawn_tile_full(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        // Default — preserves source label + question + answer; drops
        // the new fields. Real impls (TauriEvents / SlintEvents) will
        // override to honor monitor + stealth + kind.
        let _ = (monitor, stealth, kind);
        Ok(self.spawn_tile(spec))
    }
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

/// Tile kind — discriminates the chrome glyph, journal
/// categorization, and (in Phase 4 polish) the per-kind accent
/// color. Replaces today's stringly-typed source field for the new
/// `spawn_tile_full` trait method; the old `spawn_tile(spec)` keeps
/// using `spec.source: String` for incremental migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    /// AI-generated answer (default).
    Ai,
    /// Knowledge-base entry spawned from F4 palette or /key snippet.
    Kb,
    /// User snippet expansion (`/k8s` body → tile).
    Snippet,
    /// Translation of an existing tile body to RU↔EN.
    Translate,
    /// Re-ask the same question (F9) → fresh answer overwrites
    /// existing tile or spawns sibling tile.
    Reload,
    /// Follow-up suggestions tile (💡 chip after an answer).
    Followup,
    /// Bookmark tile (one-off, prints a confirmation rather than
    /// answer content; could be removed if bookmark stays
    /// statusbar-only).
    Bookmark,
    /// Post-meeting debrief tile (one per stop_session when opt-in).
    Debrief,
    /// User-forced manual tile (F6 / manual chip / PTT result).
    /// Visually distinct (gray chrome) from auto-detector Ai tiles
    /// so the user can tell which suggestions they explicitly
    /// asked for vs which the detector spawned. Added Phase B2
    /// port #3 — without this variant, manual_spawn_tile would
    /// have to use `Ai` and silently become a blue tile when the
    /// adapter gains per-kind branches.
    Manual,
    /// Auto-detector spawn — question/keyword caught in transcript
    /// without F6/F9 user action. Maps to `tile::TileKind::Auto`
    /// on the Tauri side (yellow chrome). Added Phase B2 port #5
    /// alongside System/Mic so port #7 (maybe_spawn_tile +
    /// start_session) doesn't have to re-litigate the variant set.
    Auto,
    /// System-side ask (🔊 chip / PTT on interviewer audio).
    /// Maps to `tile::TileKind::System` (purple chrome). Added
    /// Phase B2 port #5 so source-color affordance survives the
    /// adapter polish.
    System,
    /// Mic-side ask (🎤 chip / PTT on user mic). Maps to
    /// `tile::TileKind::Mic` (teal chrome). Added Phase B2 port #5.
    Mic,
}

impl TileKind {
    /// Stable string tag for journal serialization.
    #[must_use]
    pub fn as_journal_tag(&self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Kb => "kb",
            Self::Snippet => "snippet",
            Self::Translate => "translate",
            Self::Reload => "reload",
            Self::Followup => "followup",
            Self::Bookmark => "bookmark",
            Self::Debrief => "debrief",
            Self::Manual => "manual",
            Self::Auto => "auto",
            Self::System => "system",
            Self::Mic => "mic",
        }
    }

    /// Chrome glyph for the tile's source-label area. Some kinds
    /// share glyphs (Reload + Followup both use 💡 by convention).
    #[must_use]
    pub fn chrome_glyph(&self) -> &'static str {
        match self {
            Self::Ai => "",
            Self::Kb => "📚",
            Self::Snippet => "✂",
            Self::Translate => "🌐",
            Self::Reload => "🔄",
            Self::Followup => "💡",
            Self::Bookmark => "⭐",
            Self::Debrief => "🎯",
            Self::Manual => "✋",
            Self::Auto => "",
            Self::System => "🔊",
            Self::Mic => "🎤",
        }
    }
}

/// Monitor placement hint for new tile windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorHint {
    /// Use the `pick_monitor` heuristic (default to primary unless a
    /// non-primary monitor is landscape AND at least as wide as primary).
    Auto,
    /// Force the primary display.
    Primary,
    /// Index into `EnumDisplayMonitors` output (0-based). Falls back
    /// to Primary if out of range.
    Index(usize),
    /// Match a specific OS-side monitor name (Windows
    /// `EnumDisplayDevices` `DeviceString`). Falls back to `Auto` if
    /// no such monitor exists. Carries `cfg.tile_monitor_name` —
    /// added Phase B2 port #1 so the ported `run_post_meeting_debrief`
    /// doesn't silently drop the user's monitor pin.
    Named(String),
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
    fn spawn_tile_full(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        log::debug!(
            "[runtime-events:noop] spawn_tile_full kind={} monitor={monitor:?} stealth={stealth} q.len={}",
            kind.as_journal_tag(),
            spec.question.len()
        );
        // Deterministic id: encode kind tag for verifiable test output.
        Ok(format!(
            "noop-tile-{}-{}",
            kind.as_journal_tag(),
            spec.question.len()
        ))
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

    #[test]
    fn tile_kind_journal_tags_are_unique() {
        use std::collections::HashSet;
        let all = [
            TileKind::Ai,
            TileKind::Kb,
            TileKind::Snippet,
            TileKind::Translate,
            TileKind::Reload,
            TileKind::Followup,
            TileKind::Bookmark,
            TileKind::Debrief,
            TileKind::Manual,
            TileKind::Auto,
            TileKind::System,
            TileKind::Mic,
        ];
        let tags: HashSet<_> = all.iter().map(|k| k.as_journal_tag()).collect();
        assert_eq!(tags.len(), all.len(), "duplicate journal tag in TileKind");
        for k in &all {
            assert!(!k.as_journal_tag().is_empty());
        }
    }

    #[test]
    fn noop_spawn_tile_full_encodes_kind_in_id() {
        let sink: Arc<dyn RuntimeEvents> = noop();
        let spec = TileSpec {
            question: "hello".into(),
            answer: "world".into(),
            source: "ai".into(),
            is_translation: false,
        };
        let id_ai = sink
            .spawn_tile_full(spec.clone(), MonitorHint::Auto, false, TileKind::Ai)
            .expect("noop never fails");
        let id_kb = sink
            .spawn_tile_full(spec, MonitorHint::Primary, true, TileKind::Kb)
            .expect("noop never fails");
        assert_eq!(id_ai, "noop-tile-ai-5");
        assert_eq!(id_kb, "noop-tile-kb-5");
        assert_ne!(id_ai, id_kb, "different kinds must produce different ids");
    }

    #[test]
    fn monitor_hint_named_carries_string_through_clone_and_debug() {
        // Named must accept non-empty + empty names, clone cheaply, and
        // render its content in Debug output so log lines stay useful.
        let h = MonitorHint::Named("\\\\.\\DISPLAY2".into());
        let cloned = h.clone();
        assert_eq!(h, cloned);
        let dbg = format!("{h:?}");
        assert!(
            dbg.contains("DISPLAY2"),
            "Named monitor name must appear in Debug output, got: {dbg}"
        );
        // Empty-name Named is allowed at the type level; consumers
        // (TauriEvents adapter) translate it to None. Sanity-check the
        // round-trip:
        let empty = MonitorHint::Named(String::new());
        assert_ne!(empty, MonitorHint::Auto);
    }

    #[test]
    fn noop_spawn_tile_full_does_not_panic_on_named_hint() {
        let sink: Arc<dyn RuntimeEvents> = noop();
        let id = sink
            .spawn_tile_full(
                TileSpec {
                    question: "q".into(),
                    answer: "a".into(),
                    source: "debrief".into(),
                    is_translation: false,
                },
                MonitorHint::Named("\\\\.\\DISPLAY2".into()),
                true,
                TileKind::Debrief,
            )
            .unwrap();
        assert!(id.starts_with("noop-tile-debrief-"));
    }

    #[test]
    fn spawn_tile_full_default_forwards_to_spawn_tile() {
        // A custom impl that overrides only spawn_tile (not _full).
        struct MinimalImpl;
        impl RuntimeEvents for MinimalImpl {
            fn emit(&self, _channel: &str, _payload: serde_json::Value) {}
            fn spawn_tile(&self, _spec: TileSpec) -> String {
                "minimal-impl-tile".into()
            }
            // Note: spawn_tile_full is NOT overridden — default fwd.
        }
        let sink: Arc<dyn RuntimeEvents> = Arc::new(MinimalImpl);
        let id = sink
            .spawn_tile_full(
                TileSpec {
                    question: "q".into(),
                    answer: "a".into(),
                    source: "ai".into(),
                    is_translation: false,
                },
                MonitorHint::Auto,
                false,
                TileKind::Ai,
            )
            .unwrap();
        assert_eq!(id, "minimal-impl-tile");
    }
}
