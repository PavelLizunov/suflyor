//! Shared application state for the multi-window overlay-host.
//!
//! Single `Arc<Mutex<AppState>>` shared across overlay, tile, settings,
//! and replay windows (per migration plan Phase 1). Each window's
//! callbacks borrow the mutex briefly to mutate, then notify Slint
//! windows via setters from the UI thread.
//!
//! For Day 2 (skeleton) this struct only carries a few counters to
//! prove the cross-window state-sharing pipeline. Phases 2-5 will
//! add: tile registry, current session, user config, language,
//! palette state, etc.

use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct AppState {
    /// Monotonic count of tile spawns over the session. Used by the
    /// overlay bar to display "spawned N tiles" footer.
    pub tiles_spawned: u32,
    /// True when the overlay should stay on top of all other windows.
    /// Wired to overlay bar's "always on top" toggle.
    pub always_on_top: bool,
    /// True when the overlay should be hidden from screen capture.
    /// Wired to settings stealth toggle (WDA_EXCLUDEFROMCAPTURE).
    pub stealth: bool,
    /// True when microphone capture is active. Phase 3 stub — Phase 1
    /// proper integration with audio module pending.
    pub mic_active: bool,
    /// True when system audio capture is active.
    pub sys_active: bool,
    /// True when the session timer is running.
    pub timer_active: bool,
    /// Elapsed session seconds (formatted to MM:SS by overlay bar).
    pub session_secs: u64,
    /// Current AI model — cycles through "sonnet" / "haiku" / "opus".
    pub ai_model: String,
    /// Cumulative session cost in USD.
    pub cost_usd: f64,
    /// True while a record_mic_blocking probe is running. Set on
    /// click-ON, cleared in the post-result invoke_from_event_loop
    /// closure. Re-entry guard so rapid double-click doesn't spawn
    /// concurrent WASAPI captures fighting for the same device.
    /// Caught by review-agent 2026-05-27.
    pub mic_probe_in_flight: bool,
}

/// Convenience alias used by all window-spawning callbacks.
pub type SharedState = Arc<Mutex<AppState>>;

#[must_use]
pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AppState {
        always_on_top: true, // overlay defaults to topmost
        ai_model: "sonnet".to_string(),
        ..Default::default()
    }))
}

/// Cycle AI model among the full IDs accepted by both `overlay_backend::
/// ai::pricing_per_million` and the user's Claude bridge. IDs match what
/// the React/Tauri v0.1.1 app writes into `config.json`, so the two stacks
/// stay in sync.
///
/// Short names ("sonnet"/"haiku"/"opus") get mapped to the canonical
/// current-generation ID before cycling, so legacy configs still work.
/// Unknown IDs fall back to haiku (cheap default).
pub fn next_model(current: &str) -> &'static str {
    match current {
        "claude-haiku-4-5" | "haiku" => "claude-sonnet-4-6",
        "claude-sonnet-4-5" | "claude-sonnet-4-6" | "sonnet" => "claude-opus-4-7",
        "claude-opus-4-7" | "opus" => "claude-haiku-4-5",
        _ => "claude-haiku-4-5",
    }
}

/// Format session seconds as MM:SS (or H:MM:SS for >1 hour).
#[must_use]
pub fn format_timer(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All next_model outputs must be valid IDs accepted by
    /// overlay_backend::ai::pricing_per_million (otherwise we ship
    /// model strings that the bridge rejects + cost calc falls to the
    /// safe-upper-bound default). Caught by review-agent 2026-05-27.
    #[test]
    fn next_model_outputs_are_canonical_ids() {
        let canonical = ["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-7"];
        for start in canonical {
            let mut cur = start;
            for _ in 0..4 {
                cur = next_model(cur);
                assert!(
                    canonical.contains(&cur),
                    "next_model({start} ...) produced non-canonical {cur:?}"
                );
            }
        }
        assert!(canonical.contains(&next_model("haiku")));
        assert!(canonical.contains(&next_model("sonnet")));
        assert!(canonical.contains(&next_model("opus")));
        assert_eq!(next_model("garbage"), "claude-haiku-4-5");
    }
}
