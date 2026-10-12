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
    let compat = fs::read_to_string(root.join("src/win32.rs")).expect("read compatibility layer");
    let lifecycle = fs::read_to_string(root.join("src/bin/overlay_host/window_lifecycle.rs"))
        .expect("read window lifecycle");
    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read canonical host");
    let bar_tray = fs::read_to_string(root.join("src/bin/overlay_host/bar_tray.rs"))
        .expect("read bar and tray helpers");
    let tiles = fs::read_to_string(root.join("src/bin/overlay_host/tile_window.rs"))
        .expect("read tile placement");
    let lock_menu = host
        .split_once("overlay.on_lock_menu_opened")
        .expect("lock menu open handler")
        .1
        .split_once("overlay.on_lock_mode_selected")
        .expect("lock menu selection handler follows open handler")
        .0;
    let maximize = tiles
        .split_once("pub(crate) fn toggle_tile_maximize")
        .expect("tile maximize helper")
        .1
        .split_once("pub(crate) fn wire_tile_drag")
        .expect("tile drag helper follows maximize")
        .0;
    let ui = fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read bar UI");
    let build = fs::read_to_string(root.join("build.rs")).expect("read build script");

    assert!(objc.contains("NSPopUpMenuWindowLevel"));
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
    assert!(objc.contains("int32_t suflyor_macos_get_window_rect("));
    assert!(objc.contains("CGDisplayBounds(CGMainDisplayID())"));
    assert!(objc.contains("CGRectGetMaxY(primary_bounds) - NSMaxY(frame)"));

    assert!(rust.contains("RawWindowHandle::AppKit"));
    assert!(rust.contains("pub fn view_id(window: &slint::Window)"));
    assert!(rust.contains("pub fn configure_floating_by_id(view_id: isize)"));
    assert!(rust.contains("pub fn raise_key_front_by_id(view_id: isize)"));
    assert!(rust.contains("fn suflyor_macos_raise_window_key_front(view: *mut c_void) -> c_int;"));
    assert!(rust.contains("pub fn raise_key_front(window: &slint::Window)"));
    assert!(rust.contains("pub fn window_rect_by_id("));
    assert!(compat.contains("crate::native::window::view_id(window).map(HWND)"));
    assert!(compat.contains("crate::native::window::configure_floating_by_id(hwnd.0)"));
    assert!(compat.contains("crate::native::window::raise_key_front_by_id(hwnd.0)"));
    assert!(compat.contains("crate::native::window::window_rect_by_id(hwnd.0)"));
    assert!(compat.contains("std::io::ErrorKind::Unsupported"));
    assert!(lifecycle.contains("native::window::configure_floating(w.window())"));
    assert!(lifecycle.contains("native::window::raise_key_front(w.window())"));
    assert!(lifecycle.contains("pub(crate) fn set_platform_window_position("));
    assert!(lifecycle.contains("slint::PhysicalPosition::new(x, y)"));
    assert!(lifecycle.contains("slint::LogicalPosition::new(x as f32, y as f32)"));
    assert!(lifecycle.contains("set_platform_window_position(w.window(), sx, sy)"));
    assert!(lifecycle.contains("set_platform_window_position(w.window(), cx, cy)"));
    assert!(bar_tray.contains("set_platform_window_position(o.window(), x, y)"));
    assert!(bar_tray.contains("let target_width = target_w_logical.round() as i32"));
    assert!(bar_tray.contains("move_window_pos_only(hwnd, x, y)"));
    assert!(lock_menu.contains("let (bar_left, bar_top) = match get_window_rect(bar_hwnd)"));
    assert!(
        lock_menu.contains("#[cfg(target_os = \"macos\")]\n                let scale = 1.0_f32;")
    );
    assert!(lock_menu.contains("set_platform_window_position(window.window(), x, y)"));
    assert!(lock_menu.contains("set_platform_window_position(menu.window(), -32000, -32000)"));
    assert!(!lock_menu.contains("set_position(slint::PhysicalPosition"));
    assert!(maximize.contains("#[cfg(target_os = \"macos\")]\n    let scale = 1.0_f32;"));
    assert!(maximize.contains("set_platform_window_position(tile.window(), nx, ny)"));
    assert!(tiles.contains("set_platform_window_position(t.window(), x_clamped, y_clamped)"));
    assert!(tiles.contains("set_platform_window_position(t.window(), 100, 100)"));
    assert!(!lifecycle.contains("center_window"));
    assert!(!host.contains("center_window"));
    assert!(!bar_tray.contains("center_window"));
    assert!(!rust.contains("center_window"));
    assert!(!objc.contains("center_window"));
    assert!(ui.contains("!root.compact-bar && !root.bootstrap-mode"));
    assert!(ui.contains("root.compact-bar && !root.bootstrap-mode"));
    assert!(build.contains("cargo:rustc-link-lib=framework=AppKit"));
}
