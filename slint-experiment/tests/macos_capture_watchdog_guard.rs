//! Wiring guard for the macOS capture-liveness watchdog.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn canonical_runtime_owns_the_macos_watchdog_timer() {
    let wrapper = read("src/bin/overlay_host.rs");
    let host = read("src/bin/overlay_host_windows.rs");
    let watchdog = read("src/bin/overlay_host/capture_watchdog.rs");
    let status_copy = read("src/bin/overlay_host/status_copy.rs");

    assert!(wrapper.contains("include!(\"overlay_host_windows.rs\");"));
    assert!(host.contains("#[path = \"overlay_host/capture_watchdog.rs\"]"));
    assert!(host.contains("let _capture_watchdog_timer ="));
    assert!(host.contains("TimerMode::Repeated"));
    assert!(host.contains("capture_watchdog::TICK_INTERVAL"));
    assert!(host.contains(".tick(intended, snapshot)"));
    assert!(host.contains("capture_watchdog::Decision::Stop"));

    let wiring = host
        .split_once("// macOS-only fail-safe")
        .expect("watchdog wiring starts")
        .1
        .split_once("// ===== Spawn-tile poll Timer")
        .expect("watchdog wiring ends")
        .0;
    assert!(wiring.contains("state.timer_active = false"));
    assert!(wiring.contains("stop_session_and_maybe_debrief("));
    assert!(wiring.contains("TileKind::Error"));
    assert!(!wiring.contains("slint_session::start_session"));
    assert!(wiring.contains("let weak_overlay_after_stop = weak_overlay.clone()"));
    assert!(wiring.contains("slint::invoke_from_event_loop(move ||"));
    assert!(wiring.contains("capture_stopped_copy"));
    assert!(status_copy.contains("Press Start to continue"));
    assert!(host.contains("if stop_pending_for_timer_toggle.load(Ordering::Acquire)"));
    let lifecycle_lock = wiring
        .find("lifecycle.lock().await")
        .expect("watchdog takes the lifecycle lock");
    let stale_check = wiring
        .find("generation.load(Ordering::Acquire) == stop_intent")
        .expect("watchdog rejects a stale stop intent");
    assert!(wiring.contains("let timer_still_stopped = match app_state.lock()"));
    let stop_call = wiring
        .find("stop_session_and_maybe_debrief(")
        .expect("watchdog calls the shared finalizer");
    assert!(lifecycle_lock < stale_check && stale_check < stop_call);
    let finalizer = host
        .split_once("fn stop_session_and_maybe_debrief(")
        .expect("shared stop finalizer exists")
        .1;
    assert_eq!(finalizer.matches("slint_session::stop_session").count(), 1);
    assert!(finalizer.contains("events.emit(\"session:stopped\""));
    assert!(finalizer.contains("slint_session::maybe_run_debrief("));

    // Stop policy remains a pure state machine with semantic tests.
    assert!(watchdog.contains("mod tests"));
    assert!(watchdog.contains("fn flowing_stream_then_stall_requests_one_stop()"));
    assert!(watchdog.contains("fn disappeared_flowing_capture_requests_one_stop()"));
    assert!(watchdog.contains("fn never_flowed_streams_do_not_stop()"));
    assert!(watchdog.contains("fn intentional_stop_rearms_the_next_session()"));
    for obsolete in [
        "Decision::Restart",
        "MAX_ATTEMPTS",
        "BACKOFF_TICKS",
        "finish_recovery",
    ] {
        assert!(
            !watchdog.contains(obsolete),
            "obsolete recovery state: {obsolete}"
        );
    }
}

#[test]
fn manual_session_tasks_drop_stale_macos_intents() {
    let host = read("src/bin/overlay_host_windows.rs");
    assert!(host.contains(
        "generation_for_timer_toggle\n                    .fetch_add(1, Ordering::AcqRel)\n                    .wrapping_add(1)"
    ));

    let start = host
        .split_once("if new_active {")
        .expect("manual start branch exists")
        .1
        .split_once("// Stopping")
        .expect("manual stop branch follows start")
        .0;
    let start_lock = start
        .find("lifecycle_c.lock().await")
        .expect("start takes the lifecycle lock");
    let start_stale = start
        .find("generation_c.load(Ordering::Acquire) != session_intent")
        .expect("start rejects stale intent");
    let start_call = start
        .find("slint_session::start_session")
        .expect("start calls the session owner");
    assert!(start_lock < start_stale && start_stale < start_call);

    let revert = start
        .split_once("slint::invoke_from_event_loop")
        .expect("start failure reverts through the UI loop")
        .1;
    assert!(
        revert
            .find("generation_c.load(Ordering::Acquire) != session_intent")
            .expect("failure revert rejects stale intent")
            < revert
                .find("st.timer_active = false")
                .expect("failure revert clears the current intent")
    );
    assert!(revert.contains("if !st.timer_active"));

    let stop = host
        .split_once("// Stopping")
        .expect("manual stop branch exists")
        .1;
    let stop_lock = stop
        .find("lifecycle_c.lock().await")
        .expect("stop takes the lifecycle lock");
    let stop_stale = stop
        .find("generation_c.load(Ordering::Acquire) != session_intent")
        .expect("stop rejects stale intent");
    let stop_call = stop
        .find("stop_session_and_maybe_debrief(")
        .expect("stop calls the shared finalizer");
    assert!(stop_lock < stop_stale && stop_stale < stop_call);
}
