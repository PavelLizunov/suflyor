//! Phase 0 — `slint-replay` pilot binary.
//!
//! Day 1: scaffold + placeholder data.
//! Day 2: real backend wired via `replay_backend` module. Lists session
//!        journals from `%APPDATA%\overlay-mvp\sessions\`, auto-loads
//!        the newest, renders events with kind-coded accents, filter
//!        chips toggle visibility.
//!
//! Run: `cargo run --bin slint-replay` from `slint-experiment/`.

mod replay_backend;

// Slint's generated code uses `unwrap`/`expect`/`panic`/index-slicing
// extensively inside its VTable + property machinery. Our package-level
// `[lints.clippy]` (Tier 3 baseline) forbids those, so we wrap the
// generated code in a module with explicit allows. Our hand-written
// code outside this module still enforces the strict lints.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use ui::{FilterChip, MainWindow, ReplayEvent};

use crate::replay_backend::{
    SessionInfo, fmt_clock, list_sessions, load_session, render_event, total_cost_usd,
};

/// Mutable state shared between callbacks.
struct PilotState {
    sessions: Vec<SessionInfo>,
    events: Vec<serde_json::Value>,
    hidden_kinds: HashSet<String>,
    /// Distinct kinds in display order (sorted by count desc) — kept in
    /// state so chip-clicked(idx) knows which kind that index refers to.
    chip_kinds: Vec<String>,
}

impl PilotState {
    fn new() -> Self {
        Self {
            sessions: vec![],
            events: vec![],
            hidden_kinds: HashSet::new(),
            chip_kinds: vec![],
        }
    }
}

/// Slint's `ReplayEvent` struct can't carry Slint colors via inline
/// `slint::Color::from_rgb_u8(...)` calls inside an `impl From<...>`
/// expression — we just call it where needed. Centralized here so the
/// kind→color mapping isn't sprinkled across the conversion code.
/// Mirrors `chipAccentForKind()` in src/Replay.tsx.
fn accent_for_kind(kind: &str) -> slint::Color {
    match kind {
        "session_start" | "session_stop" | "session_summary" | "detector_decision" => {
            slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)
        }
        "transcript_line" => slint::Color::from_rgb_u8(0x6b, 0x72, 0x80),
        "ai_request" | "ai_response" => slint::Color::from_rgb_u8(0x81, 0x8c, 0xf8),
        "tile_spawn" => slint::Color::from_rgb_u8(0xf4, 0x72, 0xb6),
        "rate_limited" => slint::Color::from_rgb_u8(0xfa, 0xcc, 0x15),
        "error" => slint::Color::from_rgb_u8(0xf8, 0x71, 0x71),
        _ => slint::Color::from_rgb_u8(0x4a, 0x4c, 0x54),
    }
}

fn fmt_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.2} MB", n as f64 / 1024.0 / 1024.0)
    }
}

fn fmt_modified(unix: u64) -> String {
    let secs = unix;
    let days_since_epoch = secs / 86_400;
    // Cheap Gregorian conversion sufficient for "newer file first" display.
    // 1970-01-01 = day 0; full date math via the `time` crate in Phase 1.
    // For pilot, just emit "epoch+Nd HH:MM" so the user can distinguish files.
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    format!("epoch+{days_since_epoch}d {h:02}:{m:02}")
}

fn session_label(s: &SessionInfo) -> SharedString {
    SharedString::from(format!(
        "{} · {} · {}",
        s.filename,
        fmt_bytes(s.size_bytes),
        fmt_modified(s.modified_unix),
    ))
}

/// Build the filter-chips model from current state. Distinct kinds
/// sorted by count desc, with hidden status from `state.hidden_kinds`.
fn build_chips(state: &PilotState) -> Vec<FilterChip> {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, i32> = HashMap::new();
    for ev in &state.events {
        if let Some(kind) = ev.get("kind").and_then(serde_json::Value::as_str) {
            *counts.entry(kind).or_insert(0) += 1;
        }
    }
    let mut sorted: Vec<(&str, i32)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    sorted
        .into_iter()
        .map(|(kind, count)| FilterChip {
            kind: kind.into(),
            count,
            hidden: state.hidden_kinds.contains(kind),
            accent: accent_for_kind(kind),
        })
        .collect()
}

/// Build the visible-events model from current state (events filtered
/// by hidden_kinds). Each row gets a Slint-friendly ReplayEvent.
fn build_visible_events(state: &PilotState) -> Vec<ReplayEvent> {
    state
        .events
        .iter()
        .filter(|ev| {
            let kind = ev
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            !state.hidden_kinds.contains(kind)
        })
        .map(|ev| {
            let kind = ev
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            let time = fmt_clock(ev.get("unix_ms").and_then(serde_json::Value::as_u64));
            let (label, body) = render_event(ev);
            ReplayEvent {
                kind: kind.clone().into(),
                time: time.into(),
                label: label.into(),
                body: body.into(),
                accent: accent_for_kind(&kind),
            }
        })
        .collect()
}

/// Push state into the Slint window: session labels, filter chips,
/// visible events, and footer stats. Also stores the chip kinds in
/// `state.chip_kinds` so chip-clicked(idx) can look up the kind.
fn sync_window(window: &MainWindow, state: &mut PilotState) {
    let chips = build_chips(state);
    state.chip_kinds = chips.iter().map(|c| c.kind.to_string()).collect();

    let session_labels: Vec<SharedString> = state.sessions.iter().map(session_label).collect();
    window.set_session_labels(ModelRc::new(VecModel::from(session_labels)));
    window.set_filter_chips(ModelRc::new(VecModel::from(chips)));

    let visible = build_visible_events(state);
    window.set_events(ModelRc::new(VecModel::from(visible)));

    let (cost, count) = total_cost_usd(&state.events);
    // Saturate at i32::MAX so a future huge journal can't wrap to negative
    // (10 MB cap makes this practically unreachable, but cheap to guard).
    let events_i32 = i32::try_from(state.events.len()).unwrap_or(i32::MAX);
    let count_i32 = i32::try_from(count).unwrap_or(i32::MAX);
    window.set_total_events(events_i32);
    window.set_ai_response_count(count_i32);
    window.set_total_cost_display(format!("{cost:.4}").into());
}

/// Load a session from disk into state. Resets filter chips. Errors
/// log to stderr and clear events so the UI shows the empty state.
fn load_session_into_state(state: &mut PilotState, path: &str) {
    state.hidden_kinds.clear();
    match load_session(std::path::Path::new(path)) {
        Ok(events) => {
            eprintln!("[slint-replay] loaded {} events from {path}", events.len());
            state.events = events;
        }
        Err(e) => {
            eprintln!("[slint-replay] load_session failed for {path}: {e:#}");
            state.events = vec![];
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let state = Rc::new(RefCell::new(PilotState::new()));

    // ----- Initial load: list sessions, auto-select newest -----
    {
        let mut s = state.borrow_mut();
        match list_sessions() {
            Ok(sessions) => {
                eprintln!("[slint-replay] found {} session(s)", sessions.len());
                s.sessions = sessions;
            }
            Err(e) => {
                eprintln!("[slint-replay] list_sessions failed: {e:#}");
            }
        }
        // Auto-load newest if any.
        if let Some(first) = s.sessions.first() {
            let path = first.path.clone();
            load_session_into_state(&mut s, &path);
        }
        sync_window(&window, &mut s);
    }
    window.set_selected_session_index(0);

    // ----- Callbacks -----

    let s = state.clone();
    let w = window.as_weak();
    window.on_session_changed(move |idx| {
        let Some(w) = w.upgrade() else { return };
        let mut st = s.borrow_mut();
        let i = idx as usize;
        if let Some(info) = st.sessions.get(i).cloned() {
            eprintln!("[slint-replay] session-changed: idx={i} path={}", info.path);
            load_session_into_state(&mut st, &info.path);
            sync_window(&w, &mut st);
        }
    });

    let s = state.clone();
    let w = window.as_weak();
    window.on_chip_clicked(move |idx| {
        let Some(w) = w.upgrade() else { return };
        let mut st = s.borrow_mut();
        let i = idx as usize;
        let Some(kind) = st.chip_kinds.get(i).cloned() else { return };
        if st.hidden_kinds.contains(&kind) {
            st.hidden_kinds.remove(&kind);
        } else {
            st.hidden_kinds.insert(kind.clone());
        }
        eprintln!(
            "[slint-replay] chip-clicked: idx={i} kind={kind} hidden_count={}",
            st.hidden_kinds.len()
        );
        sync_window(&w, &mut st);
    });

    let s = state.clone();
    let w = window.as_weak();
    window.on_reset_filter(move || {
        let Some(w) = w.upgrade() else { return };
        let mut st = s.borrow_mut();
        st.hidden_kinds.clear();
        eprintln!("[slint-replay] reset-filter");
        sync_window(&w, &mut st);
    });

    let w = window.as_weak();
    window.on_back_clicked(move || {
        if let Some(w) = w.upgrade() {
            // Pilot: just hide. Real Replay (Phase 1+) returns to overlay.
            let _ = w.hide();
        }
    });

    window.run()
}
