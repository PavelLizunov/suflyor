//! Slint-binary implementation of `overlay_backend::events::RuntimeEvents`.
//!
//! This is the analog of `src-tauri/src/lib.rs::TauriEvents` — when
//! the ported overlay-backend fns (reask_last, manual_spawn_tile,
//! manual_ask_source, manual_ask_window_end, ask_stream_loop,
//! run_post_meeting_debrief) call `events.emit(...)` or
//! `events.spawn_tile_full(...)`, this adapter is what receives the
//! call from the Slint binary side and updates the UI accordingly.
//!
//! Threading: trait methods can be called from ANY thread (tokio
//! worker, Slint main, std::thread). UI mutations go through
//! `slint::invoke_from_event_loop` because Slint property setters
//! must execute on the main event-loop thread. Tile-spawn requests
//! follow the same pattern — the actual `TileWindow::new()` call
//! happens inside the invoked closure.
//!
//! Channel-name routing (in `emit`):
//! - "transcript:line"  → append to overlay's transcript model.
//! - "cost:update"      → set cost_usd property.
//! - "cost:cap-hit"     → show over-budget chip.
//! - "health:update"    → update 3 health-dot states.
//! - "speech:coach"     → update WPM + filler-density labels.
//! - "tile:error"       → eprintln + (future) toast.
//! - "tile:rate-limited" → eprintln + (future) throttle indicator.
//! - "ai:event"         → streamed F9 deltas; forwarder updates the
//!   most recent in-flight tile.
//! - "meeting:ending"   → show 🏁 chip.
//! - "session:started" / "session:stopped" → toggle UI state.
//!
//! All channel names match the React-side `app.emit_to("overlay", ...)`
//! convention from `src-tauri/src/lib.rs::TauriEvents` so the ported
//! fns are oblivious to which binary they're running in.

use overlay_backend::events::{MonitorHint, RuntimeEvents, TileKind, TileSpec};
use std::sync::Arc;

/// Trait the Slint binary's main `overlay_host.rs` implements on a
/// struct that holds the `Weak<OverlayBarWindow>` + tile registry.
/// SlintEvents calls it from inside `invoke_from_event_loop` so the
/// actual UI mutations happen on the main thread.
///
/// Keeping the surface narrow lets us swap implementations in tests
/// (e.g. a recording sink that captures events for assertions) AND
/// avoid pulling Slint-generated UI types into the SlintEvents impl
/// itself (which would prevent the impl from compiling outside the
/// binary crate).
pub trait SlintUiBridge: Send + Sync {
    /// Forward a backend event to the UI thread. Implementor uses
    /// `slint::invoke_from_event_loop` to schedule the property
    /// updates on the Slint main thread.
    fn forward_event(&self, channel: String, payload: serde_json::Value);

    /// Spawn a tile window matching the spec. Implementor schedules
    /// the `TileWindow::new()` call on the Slint main thread via
    /// `invoke_from_event_loop` and registers the resulting window
    /// in the tile registry. Returns the assigned label — the impl
    /// is expected to generate one synchronously (e.g. based on a
    /// shared atomic counter) since `spawn_tile_full` is sync from
    /// the caller's perspective.
    ///
    /// # Errors
    /// Returns Err with a human-readable diagnostic when the spawn
    /// can't be scheduled (Slint event-loop terminated etc.).
    fn schedule_spawn_tile(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String>;
}

/// `RuntimeEvents` impl that defers all UI work to a `SlintUiBridge`
/// implementor. The bridge is Arc'd because the same SlintEvents
/// instance is cloned into every spawned task / Inputs struct.
pub struct SlintEvents {
    pub bridge: Arc<dyn SlintUiBridge>,
}

impl SlintEvents {
    /// Construct from any `Arc<dyn SlintUiBridge>` impl. Typical
    /// usage from `overlay_host.rs`:
    ///
    /// ```ignore
    /// let bridge = Arc::new(OverlayBarBridge {
    ///     overlay_weak: overlay.as_weak(),
    ///     tiles: tiles.clone(),
    ///     tile_seq: Arc::new(AtomicU64::new(0)),
    /// });
    /// let events: Arc<dyn RuntimeEvents> = Arc::new(SlintEvents { bridge });
    /// ```
    #[must_use]
    pub fn new(bridge: Arc<dyn SlintUiBridge>) -> Self {
        Self { bridge }
    }
}

impl RuntimeEvents for SlintEvents {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        self.bridge.forward_event(channel.to_string(), payload);
    }

    fn spawn_tile(&self, spec: TileSpec) -> String {
        // Soft-deprecated path. Maps to spawn_tile_full with Auto +
        // stealth=false + Ai defaults — same shape as TauriEvents.
        self.spawn_tile_full(spec, MonitorHint::Auto, false, TileKind::Ai)
            .unwrap_or_else(|e| {
                // slint-experiment doesn't pull in the `log` crate;
                // surface failures via stderr to match the binary's
                // existing pattern (`eprintln!("[overlay-host] ...")`).
                eprintln!("[slint-events] spawn_tile failed: {e}");
                String::new()
            })
    }

    fn spawn_tile_full(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        self.bridge
            .schedule_spawn_tile(spec, monitor, stealth, kind)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests need brevity for unwrap_or_else on Result values; runtime code stays strict"
)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Recording bridge for unit tests — captures every forwarded
    /// event + spawn call without touching Slint.
    #[derive(Default)]
    struct RecordingBridge {
        events: Mutex<Vec<(String, serde_json::Value)>>,
        spawns: Mutex<Vec<(TileSpec, TileKind)>>,
        next_label: std::sync::atomic::AtomicU64,
    }

    impl SlintUiBridge for RecordingBridge {
        fn forward_event(&self, channel: String, payload: serde_json::Value) {
            let mut g = match self.events.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            g.push((channel, payload));
        }
        fn schedule_spawn_tile(
            &self,
            spec: TileSpec,
            _monitor: MonitorHint,
            _stealth: bool,
            kind: TileKind,
        ) -> Result<String, String> {
            let n = self
                .next_label
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut g = match self.spawns.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            g.push((spec, kind));
            Ok(format!("slint-tile-{n}"))
        }
    }

    fn lock_evs(
        bridge: &RecordingBridge,
    ) -> std::sync::MutexGuard<'_, Vec<(String, serde_json::Value)>> {
        match bridge.events.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    fn lock_spawns(
        bridge: &RecordingBridge,
    ) -> std::sync::MutexGuard<'_, Vec<(TileSpec, TileKind)>> {
        match bridge.spawns.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    #[test]
    fn emit_forwards_channel_and_payload_to_bridge() {
        let bridge = Arc::new(RecordingBridge::default());
        let events = SlintEvents::new(bridge.clone());
        events.emit(
            "transcript:line",
            serde_json::json!({"text": "hello", "source": "mic"}),
        );
        events.emit("cost:update", serde_json::json!({"session_usd": 0.0012}));

        let evs = lock_evs(&bridge);
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].0, "transcript:line");
        assert_eq!(
            evs[0].1,
            serde_json::json!({"text": "hello", "source": "mic"})
        );
        assert_eq!(evs[1].0, "cost:update");
    }

    #[test]
    fn spawn_tile_full_returns_label_and_records_spec() {
        let bridge = Arc::new(RecordingBridge::default());
        let events = SlintEvents::new(bridge.clone());
        let label1 = match events.spawn_tile_full(
            TileSpec {
                question: "q1".into(),
                answer: "a1".into(),
                source: "ai".into(),
                is_translation: false,
                highlights: vec!["k8s".into()],
            },
            MonitorHint::Auto,
            false,
            TileKind::Ai,
        ) {
            Ok(l) => l,
            Err(e) => panic!("first spawn_tile_full failed: {e}"),
        };
        let label2 = match events.spawn_tile_full(
            TileSpec {
                question: "q2".into(),
                answer: "a2".into(),
                source: "kb".into(),
                is_translation: false,
                highlights: vec![],
            },
            MonitorHint::Named("\\\\.\\DISPLAY2".into()),
            true,
            TileKind::Kb,
        ) {
            Ok(l) => l,
            Err(e) => panic!("second spawn_tile_full failed: {e}"),
        };
        assert_ne!(label1, label2);
        assert!(label1.starts_with("slint-tile-"));

        let spawns = lock_spawns(&bridge);
        assert_eq!(spawns.len(), 2);
        assert_eq!(spawns[0].1, TileKind::Ai);
        assert_eq!(spawns[0].0.highlights, vec!["k8s".to_string()]);
        assert_eq!(spawns[1].1, TileKind::Kb);
    }

    #[test]
    fn slint_events_is_a_runtime_events_trait_object() {
        // Type-check: confirms the trait impl signature matches
        // RuntimeEvents — caught if anyone changes the trait shape.
        let bridge = Arc::new(RecordingBridge::default());
        let events: Arc<dyn RuntimeEvents> = Arc::new(SlintEvents::new(bridge));
        // If this compiles + runs, the impl is correctly object-safe.
        events.emit("test", serde_json::Value::Null);
    }
}
