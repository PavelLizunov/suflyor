//! Wiring guard for the macOS global hotkey foundation.
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
fn macos_main_registers_and_polls_global_hotkeys() {
    let host = read(root(), "src/bin/overlay_host.rs");

    assert!(host.contains("mod hotkeys;"));
    assert!(host.contains("let registered_hotkeys = hotkeys::register_hotkeys();"));

    let before_run = host
        .split_once("let result = window.run();")
        .expect("window.run() exists")
        .0;
    let hotkey_block = before_run
        .split_once("let hotkey_timer = slint::Timer::default();")
        .expect("hotkey timer exists before window.run()")
        .1;

    assert!(hotkey_block.contains("slint::TimerMode::Repeated"));
    assert!(hotkey_block.contains("global_hotkey::GlobalHotKeyEvent::receiver()"));
    assert!(hotkey_block.contains("macos_text_ask::TextAskSlice::open_text_ask"));
    assert!(hotkey_block.contains("f1_id"));
    assert!(hotkey_block.contains("f3_id"));
    assert!(hotkey_block.contains("f4_id"));
    assert!(hotkey_block.contains("f6_id"));
    assert!(hotkey_block.contains("f7_id"));
    assert!(hotkey_block.contains("f8_id"));
    assert!(hotkey_block.contains("sf8_id"));
    assert!(hotkey_block.contains("cf8_id"));
    assert!(hotkey_block.contains("f9_id"));
    assert!(hotkey_block.contains("sf9_id"));
    assert!(hotkey_block.contains("sa1_id"));
    assert!(hotkey_block.contains("sa2_id"));
    assert!(hotkey_block.contains("sa3_id"));

    let after_run = host
        .split_once("let result = window.run();")
        .expect("window.run() exists")
        .1;
    assert!(after_run.contains("drop(registered_hotkeys);"));
}
