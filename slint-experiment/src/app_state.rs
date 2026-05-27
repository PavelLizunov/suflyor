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
}

/// Convenience alias used by all window-spawning callbacks.
pub type SharedState = Arc<Mutex<AppState>>;

#[must_use]
pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AppState {
        always_on_top: true, // overlay defaults to topmost
        ..Default::default()
    }))
}
