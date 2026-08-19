#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn production_macos_window_uses_the_proven_minimal_appkit_bridge() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let objc =
        fs::read_to_string(root.join("src/native/macos/window.m")).expect("read AppKit bridge");
    let rust =
        fs::read_to_string(root.join("src/native/macos/window.rs")).expect("read Rust adapter");
    let host = fs::read_to_string(root.join("src/bin/overlay_host.rs")).expect("read host");
    let ui = fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read bar UI");
    let build = fs::read_to_string(root.join("build.rs")).expect("read build script");

    assert!(objc.contains("NSStatusWindowLevel"));
    assert!(objc.contains("NSWindowCollectionBehaviorCanJoinAllSpaces"));
    assert!(objc.contains("1UL << 18"));
    assert!(!objc.contains("NSStatusItem"));
    assert!(!objc.contains("Gate 0A"));
    // The key/front raise reuses the shared view-to-window helper and stays
    // on plain AppKit: activate the app, then make the window key and front.
    assert!(objc.contains("int32_t suflyor_macos_raise_window_key_front(void *raw_view)"));
    assert!(objc.contains("suflyor_window_for_view(raw_view)"));
    assert!(objc.contains("makeKeyAndOrderFront"));
    assert!(objc.contains("orderFrontRegardless"));
    assert!(objc.contains("activateIgnoringOtherApps:YES"));
    assert!(objc.contains("int32_t suflyor_macos_center_window(void *raw_view)"));

    assert!(rust.contains("RawWindowHandle::AppKit"));
    assert!(rust.contains("fn suflyor_macos_raise_window_key_front(view: *mut c_void) -> c_int;"));
    assert!(rust.contains("pub fn raise_key_front(window: &slint::Window)"));
    assert!(rust.contains("pub fn center_window(window: &slint::Window)"));
    assert!(host.contains("set_bootstrap_mode(false)"));
    assert!(host.contains("native::window::configure_floating"));
    assert!(host.contains("native::window::begin_drag"));
    assert!(ui.contains("!root.compact-bar && !root.bootstrap-mode"));
    assert!(ui.contains("root.compact-bar && !root.bootstrap-mode"));
    assert!(build.contains("cargo:rustc-link-lib=framework=AppKit"));
}
