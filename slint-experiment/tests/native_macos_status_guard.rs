//! Guard the macOS status-item recovery bridge.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn source(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap_or_else(|error| {
        panic!("read {relative}: {error}");
    })
}

#[test]
fn status_bridge_installs_once_and_removes_symmetrically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let objc = source(root, "src/native/macos/status.m");

    assert_eq!(objc.matches("statusItemWithLength").count(), 1);
    assert_eq!(objc.matches("removeStatusItem").count(), 1);
    assert!(objc.contains("suflyor_status_item != nil"));
    assert!(objc.contains("[NSThread isMainThread]"));
    assert!(objc.contains("[window orderOut:nil]"));
    assert!(objc.contains("suflyor_macos_configure_floating_window("));
    assert!(objc.contains("orderFrontRegardless"));
}

#[test]
fn status_menu_has_one_toggle_and_quit_with_plain_labels() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let objc = source(root, "src/native/macos/status.m");

    assert_eq!(objc.matches("initWithTitle:").count(), 2);
    assert!(objc.contains("@selector(toggleOverlay:)"));
    assert!(objc.contains("@selector(quitSuflyor:)"));
    for label in ["Hide Suflyor", "Show Suflyor", "Quit Suflyor"] {
        assert!(objc.contains(label), "missing native label {label}");
    }
}

#[test]
fn status_guard_is_synchronous_main_thread_owned_and_removes_on_drop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust = source(root, "src/native/macos/status.rs");

    assert!(rust.contains("pub struct StatusItemGuard"));
    assert!(rust.contains("impl Drop for StatusItemGuard"));
    assert!(rust.contains("suflyor_macos_status_remove"));
    assert!(rust.contains("PhantomData<Rc<()>>"));
    assert!(rust.contains("slint::quit_event_loop"));
    assert!(!rust.contains("Timer::single_shot"));
}

#[test]
fn production_host_owns_one_status_guard_through_the_event_loop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = source(root, "src/bin/overlay_host.rs");

    assert_eq!(host.matches("native::status::install").count(), 1);
    let install_at = host.find("native::status::install").unwrap();
    let run_at = host.find("window.run()").unwrap();
    assert!(install_at < run_at);
    let drop_at = host.find("drop(status_item)").unwrap();
    assert!(run_at < drop_at);

    let build = source(root, "build.rs");
    assert!(build.contains("src/native/macos/status.m"));
    assert!(build.contains("rerun-if-changed=src/native/macos/status.m"));

    let module = source(root, "src/native/mod.rs");
    assert!(module.contains("macos/status.rs"));
}

#[test]
fn status_bridge_has_no_disposable_gate_branding() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in ["src/native/macos/status.m", "src/native/macos/status.rs"] {
        let lowered = source(root, relative).to_lowercase();
        assert!(!lowered.contains("gate0a"));
        assert!(!lowered.contains("gate 0a"));
    }
}
