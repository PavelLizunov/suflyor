//! Explicit boundaries of the experimental Wayland port.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
fn unavailable_native_features_do_not_fabricate_success() {
    assert!(slint_replay::win32::stealth_supported());
    assert!(slint_replay::win32::set_stealth(slint_replay::win32::HWND(0), true).is_err());
    // Real monitor/capture adapters are exercised only in the live Wayland checks.
    // This headless guard keeps the unimplemented exclusion boundary explicit.
}

#[test]
fn remapped_windows_keep_the_application_id_and_first_frame_protection() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = std::fs::read_to_string(root.join("src/bin/overlay_host_windows.rs")).unwrap();
    let rules = std::fs::read_to_string(root.join("../scripts/suflyor-hyprland.lua")).unwrap();
    assert!(host.contains("slint::set_xdg_app_id(\"suflyor\")?"));
    let application = rules.split_once("name = \"suflyor-native-app\"").unwrap().1;
    let application = application.split_once("})").unwrap().0;
    assert!(application.contains("class = \"^suflyor$\""));
    assert!(application.contains("no_screen_share = true"));
    let lifecycle =
        std::fs::read_to_string(root.join("src/bin/overlay_host/window_lifecycle.rs")).unwrap();
    assert!(lifecycle.contains("realize_wayland_window(win, attempt, fallback, 50)"));
    assert!(lifecycle.contains("if !window.window().is_visible() || attempt(&window)"));
    assert!(lifecycle.contains("remaining - 1"));
    for relative in [
        "src/bin/overlay_host/tile_window.rs",
        "src/bin/overlay_host/window_lifecycle.rs",
    ] {
        let source = std::fs::read_to_string(root.join(relative)).unwrap();
        assert!(source.contains("global_stealth() || cfg!(target_os = \"linux\")"));
        assert!(source.contains("hwnd, global_stealth()"));
    }
}

#[test]
fn native_identity_does_not_keep_a_recreated_window_address_claimed() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("src/native/linux/window.rs")).unwrap();
    assert!(source.contains("native: Weak<slint::winit_030::winit::window::Window>"));
    assert!(source.contains("v.native.strong_count() > 0"));
    assert_eq!(source.matches("native: Arc::downgrade(&native)").count(), 2);
}

#[test]
fn bar_size_uses_compositor_resize_with_stale_toggle_cancellation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("src/bin/overlay_host/bar_tray.rs")).unwrap();
    assert!(source.contains("resize_wayland_bar(overlay, w as i32, h as i32, None)"));
    assert!(source.contains("if bar.get_compact_bar() != (width == 680)"));
    assert!(
        source.contains("slint_replay::native::window::resize_move(hwnd.0, x, y, width, height)")
    );
    assert!(source.contains("w == width && h == height"));
    assert!(source.contains("resize_wayland_bar(&o, width, 64, position)"));
    assert!(source.contains("super::pos_on_visible_monitor(*p, &monitors)"));
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
