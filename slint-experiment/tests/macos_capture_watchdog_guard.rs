//! Wiring guard for the macOS capture-liveness watchdog.
//!
//! The recovery semantics live in the pure state machine and are covered
//! by the unit tests in `macos_session.rs`. This file pins only the two
//! wiring facts ordinary compilation and unit tests cannot see: the
//! watchdog runs on its own live repeated Slint timer in the macOS main,
//! and only the manual chip paths reset its episode state.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn watchdog_runs_on_its_own_repeated_slint_timer() {
    let host = read(root(), "src/bin/overlay_host.rs");

    // A distinct binding from the tile-drain timer: fusing them would let
    // tile traffic silently disable or starve the liveness fold.
    assert!(host.contains("let tile_drain_timer = slint::Timer::default();"));
    assert!(host.contains("let watchdog_timer = slint::Timer::default();"));

    // Declared before window.run() so the binding stays alive through the
    // whole event loop, and wired to the module's cadence + fold entry.
    let before_run = host
        .split_once("let result = window.run();")
        .expect("window.run() exists")
        .0;
    let watchdog_block = before_run
        .split_once("let watchdog_timer = slint::Timer::default();")
        .expect("watchdog timer exists before window.run()")
        .1;
    assert!(watchdog_block.contains("slint::TimerMode::Repeated"));
    assert!(watchdog_block.contains("macos_session::WATCHDOG_TICK_INTERVAL"));
    assert!(watchdog_block.contains("watchdog_session.watchdog_tick()"));
}

#[test]
fn only_manual_chip_paths_reset_the_watchdog() {
    let module = read(root(), "src/bin/overlay_host/macos_session.rs");

    // Exactly two reset sites — manual start and manual stop. A reset on
    // the automatic restart path would let a dying stream wipe its own
    // attempt budget and loop restarts forever.
    assert_eq!(
        module
            .matches("self.watchdog.borrow_mut().reset();")
            .count(),
        2
    );
    assert!(module.contains("fn restart_internal(&self)"));
}
