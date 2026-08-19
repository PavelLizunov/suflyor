//! Reachability guard for global hotkeys in the canonical shared host.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn canonical_runtime_registers_and_polls_all_product_hotkeys() {
    let host = read("src/bin/overlay_host_windows.rs");
    let hotkeys = read("src/bin/overlay_host/hotkeys.rs");

    assert!(host.contains("#[path = \"overlay_host/hotkeys.rs\"]"));
    assert!(host.contains("} = register_hotkeys();"));
    assert!(host.contains("hotkey_poll.start("));
    assert!(host.contains("TimerMode::Repeated"));
    assert!(host.contains("global_hotkey::GlobalHotKeyEvent::receiver()"));
    assert!(host.contains("open_text_ask("));

    for label in [
        "F1",
        "F3",
        "F4",
        "F6",
        "F7",
        "F8",
        "Shift+F8",
        "Ctrl+F8",
        "F9",
        "Shift+F9",
        "Shift+Alt+1",
        "Shift+Alt+2",
        "Shift+Alt+3",
    ] {
        assert!(hotkeys.contains(&format!("\"{label}\"")), "missing {label}");
    }
}
