//! Reachability and native clipboard guards for the canonical macOS runtime.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn canonical_runtime_is_shared_and_reaches_text_ask() {
    let wrapper = read("src/bin/overlay_host.rs");
    let host = read("src/bin/overlay_host_windows.rs");
    let windows = read("src/bin/overlay_host/aux_windows.rs");

    assert!(wrapper.contains("#[cfg(any(windows, target_os = \"macos\"))]"));
    assert!(wrapper.contains("include!(\"overlay_host_windows.rs\");"));
    assert!(host.contains("#[path = \"overlay_host/aux_windows.rs\"]"));
    assert!(host.contains("open_text_ask("));
    assert!(windows.contains("pub(crate) fn open_text_ask("));
    assert!(windows.contains("TextAskWindow::new()"));
}

#[test]
fn macos_clipboard_copy_keeps_its_native_contracts() {
    let obj_c = read("src/native/macos/clipboard.m");
    for symbol in [
        "NSPasteboard",
        "initWithBytes",
        "NSUTF8StringEncoding",
        "CGEventSourceFlagsState",
        "AXIsProcessTrusted()",
        "CGEventCreateKeyboardEvent(NULL, 8, true)",
        "CGEventPost(kCGHIDEventTap",
    ] {
        assert!(
            obj_c.contains(symbol),
            "missing native clipboard seam: {symbol}"
        );
    }
    assert!(!obj_c.contains("stringWithUTF8String"));

    let adapter = read("src/native/macos/clipboard.rs");
    for signature in [
        "pub fn set_text(text: &str) -> Result<(), String>",
        "pub fn read_text() -> Option<String>",
        "pub fn clear()",
        "pub fn copy_modifiers_released() -> bool",
        "pub fn send_command_c() -> bool",
    ] {
        assert!(
            adapter.contains(signature),
            "missing adapter API: {signature}"
        );
    }
    assert!(!adapter.contains("logging::line"));

    let routing = read("src/win32.rs");
    assert!(routing.contains("crate::native::clipboard::send_command_c()"));
    assert!(routing.contains("crate::native::clipboard::read_text()"));

    let host = read("src/bin/overlay_host_windows.rs");
    assert!(host.contains("if !slint_replay::win32::send_ctrl_c()"));
    assert!(host.contains("restore_text_clipboard(saved.as_ref())"));
    assert!(host.contains("Accessibility permission is required"));
    assert!(host.contains("ponytail: SA1 preserves only UTF-8 text"));
}
