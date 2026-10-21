//! Explicit boundaries of the experimental Wayland port.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
fn unavailable_native_features_do_not_fabricate_success() {
    assert!(!slint_replay::win32::stealth_supported());
    assert!(slint_replay::win32::set_stealth(slint_replay::win32::HWND(0), true).is_err());
    // Real monitor/capture adapters are exercised only in the live Wayland checks.
    // This headless guard keeps the unimplemented exclusion boundary explicit.
}

#[test]
#[ignore = "requires an isolated live Wayland worker clipboard"]
fn wayland_clipboard_round_trip() {
    use slint_replay::native::clipboard;
    let saved = clipboard::read_text();
    let text = "Suflyor Wayland clipboard test 123";
    let write = clipboard::set_text(text);
    let read = clipboard::read_text();
    match saved {
        Some(saved) => clipboard::write_text(&saved),
        None => clipboard::clear(),
    }
    assert!(write.is_ok());
    assert_eq!(read.as_deref(), Some(text));
}
