//! Phase 0 — `slint-replay` pilot binary.
//!
//! Day 1: scaffold + placeholder data.
//! Day 2: real backend wired via `replay_backend` module. Lists session
//!        journals from `%APPDATA%\suflyor\sessions\`, auto-loads
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
    fmt_clock, list_sessions, load_session, render_event, total_cost_usd, SessionInfo,
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
    window.set_any_hidden(!state.hidden_kinds.is_empty());
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
        let Some(kind) = st.chip_kinds.get(i).cloned() else {
            return;
        };
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

// ============================================================================
// Phase 0 Day 3 — i-slint-backend-testing tests.
//
// The plan calls for three test SCENARIOS:
//   1. load → assert N events appear in timeline
//   2. click filter chip → assert events of that kind disappear
//   3. switch session → assert filter resets to "all"
//
// Slint's testing backend (`i_slint_backend_testing::init_no_event_loop`)
// installs a per-thread platform shim, and Rust's libtest spawns a new
// thread for each `#[test]` fn even with `--test-threads=1`. So calling
// `MainWindow::new()` from a second `#[test]` panics with "The Slint
// platform was initialized in another thread" — confirmed live with the
// earlier 3-test layout, where tests 1+2 passed but test 3 hit the
// thread-affinity check.
//
// Workaround for the pilot: ONE `#[test]` fn that runs all three
// scenarios sequentially on the same thread. Scenarios stay clearly
// labeled. A real Phase 1 test suite would invest in a custom harness
// (libtest-mimic or `serial_test` + a long-lived spawned worker
// thread) so each scenario can be a true `#[test]`. Documented in the
// pilot report as a known Slint-testing gotcha.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test brevity
mod tests {
    use super::*;
    use serde_json::json;
    use slint::Model;

    fn ev(kind: &str, unix_ms: u64) -> serde_json::Value {
        json!({"kind": kind, "unix_ms": unix_ms})
    }

    /// Phase 0.5 spike 2 — Slint MCP server smoke. Holds the test
    /// process alive for 12 s so a sibling probe can check whether
    /// `SLINT_MCP_PORT=8080` actually opens a TCP listener. Marked
    /// `#[ignore]` so normal `cargo test` doesn't sit waiting.
    ///
    /// Run: SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080 \
    ///      cargo test --bin slint-replay -- --ignored --nocapture mcp_smoke
    /// Then in another shell: Test-NetConnection localhost 8080
    #[test]
    #[ignore = "interactive MCP smoke — run with --ignored only"]
    fn mcp_smoke() {
        i_slint_backend_testing::init_no_event_loop();
        let _window = MainWindow::new().expect("create window");
        let port = std::env::var("SLINT_MCP_PORT").unwrap_or_else(|_| "(unset)".into());
        let debug = std::env::var("SLINT_EMIT_DEBUG_INFO").unwrap_or_else(|_| "(unset)".into());
        eprintln!("[mcp-smoke] SLINT_MCP_PORT={port} SLINT_EMIT_DEBUG_INFO={debug}");
        eprintln!("[mcp-smoke] sleeping 12 s — probe with `Test-NetConnection localhost {port}`");
        std::thread::sleep(std::time::Duration::from_secs(12));
        eprintln!("[mcp-smoke] exiting cleanly");
    }

    /// Combined Phase 0 test — three scenarios, single thread.
    #[test]
    fn slint_pilot_scenarios() {
        i_slint_backend_testing::init_no_event_loop();
        let window = MainWindow::new().expect("create window");

        // ===================================================================
        // Scenario 1 — load: given populated state, sync_window pushes
        // the events model into Slint at the right row count + chip count.
        // ===================================================================
        {
            let mut state = PilotState::new();
            state.events = vec![
                ev("transcript_line", 1000),
                ev("transcript_line", 2000),
                ev("ai_response", 3000),
            ];
            sync_window(&window, &mut state);

            assert_eq!(window.get_total_events(), 3, "scenario 1: total_events");
            assert_eq!(window.get_ai_response_count(), 1, "scenario 1: ai count");
            assert_eq!(
                window.get_filter_chips().row_count(),
                2,
                "scenario 1: 2 distinct kinds → 2 chips"
            );
            assert_eq!(
                window.get_events().row_count(),
                3,
                "scenario 1: no hidden kinds → all visible"
            );
            // chip_kinds order is by count desc, then kind asc as tie-breaker.
            assert_eq!(state.chip_kinds.len(), 2);
            assert_eq!(state.chip_kinds[0], "transcript_line"); // count=2
            assert_eq!(state.chip_kinds[1], "ai_response"); // count=1
        }

        // ===================================================================
        // Scenario 2 — chip toggle: hiding a kind drops only that kind's
        // events from the visible model, totals stay unfiltered.
        // ===================================================================
        {
            let mut state = PilotState::new();
            state.events = vec![
                ev("transcript_line", 1000),
                ev("transcript_line", 2000),
                ev("ai_response", 3000),
            ];
            sync_window(&window, &mut state);

            let idx = state
                .chip_kinds
                .iter()
                .position(|k| k == "ai_response")
                .expect("ai_response chip should exist");
            let kind = state.chip_kinds[idx].clone();

            // Mirror what on_chip_clicked does — toggle into hidden_kinds,
            // re-sync to push the filtered events to the window.
            state.hidden_kinds.insert(kind.clone());
            sync_window(&window, &mut state);

            assert_eq!(window.get_events().row_count(), 2, "scenario 2: 2 visible");
            assert_eq!(window.get_total_events(), 3, "scenario 2: total unchanged");
            assert_eq!(
                window.get_ai_response_count(),
                1,
                "scenario 2: ai unchanged"
            );

            let chips = window.get_filter_chips();
            let ai_chip = chips
                .iter()
                .find(|c| c.kind.as_str() == "ai_response")
                .expect("ai_response chip");
            assert!(
                ai_chip.hidden,
                "scenario 2: ai_response chip flagged hidden"
            );

            // Toggle back on — all 3 visible.
            state.hidden_kinds.remove(&kind);
            sync_window(&window, &mut state);
            assert_eq!(
                window.get_events().row_count(),
                3,
                "scenario 2: untoggle restores"
            );
        }

        // ===================================================================
        // Scenario 3 — session switch: load_session_into_state's reset
        // semantics (clear hidden_kinds, replace events) leave the new
        // session showing all its events.
        // ===================================================================
        {
            let mut state = PilotState::new();
            state.events = vec![ev("transcript_line", 1000), ev("transcript_line", 2000)];
            state.hidden_kinds.insert("transcript_line".to_string());
            sync_window(&window, &mut state);
            // Pre-condition: 2 total, 0 visible.
            assert_eq!(window.get_total_events(), 2);
            assert_eq!(window.get_events().row_count(), 0);

            // Switching to session B: same mutation load_session_into_state
            // does (clear filter, replace events). The disk-reading branch is
            // exercised by the binary at runtime; here we test the post-load
            // state transition.
            state.hidden_kinds.clear();
            state.events = vec![
                ev("ai_response", 3000),
                ev("ai_response", 4000),
                ev("ai_response", 5000),
            ];
            sync_window(&window, &mut state);

            assert!(state.hidden_kinds.is_empty(), "scenario 3: filter reset");
            assert_eq!(
                window.get_total_events(),
                3,
                "scenario 3: new session total"
            );
            assert_eq!(
                window.get_events().row_count(),
                3,
                "scenario 3: all events of new session visible"
            );
            assert_eq!(window.get_ai_response_count(), 3, "scenario 3: ai count");

            // Scenario 3b — call the REAL load_session_into_state to
            // exercise the disk path and confirm hidden_kinds is cleared
            // even when the load itself errors out. Catches the regression
            // class "load_session_into_state forgot to clear hidden_kinds
            // in the Err branch" — the inlined mutation above would NOT.
            state.hidden_kinds.insert("ai_response".to_string());
            assert!(!state.hidden_kinds.is_empty(), "pre: hidden seeded");
            load_session_into_state(&mut state, "C:/nonexistent/slint-pilot-test.jsonl");
            assert!(
                state.hidden_kinds.is_empty(),
                "scenario 3b: load_session_into_state must clear hidden_kinds even on error"
            );
            assert!(
                state.events.is_empty(),
                "scenario 3b: events cleared on error path"
            );
        }
    }
}
