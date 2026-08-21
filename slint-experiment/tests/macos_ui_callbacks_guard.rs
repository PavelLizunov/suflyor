//! Reachability guard for bar callbacks in the canonical shared host.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn canonical_runtime_wires_product_bar_callbacks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let wrapper = fs::read_to_string(root.join("src/bin/overlay_host.rs")).expect("read wrapper");
    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read canonical host");

    assert!(wrapper.contains("include!(\"overlay_host_windows.rs\");"));
    for callback in [
        "overlay.on_quit_confirm",
        "overlay.on_drag_start_requested",
        "overlay.on_text_ask_clicked",
        "overlay.on_open_settings_clicked",
        "overlay.on_mic_toggle_clicked",
        "overlay.on_capture_clicked",
        "overlay.on_spawn_tile_clicked",
        "overlay.on_pause_toggle_clicked",
        "overlay.on_help_clicked",
        "overlay.on_archive_clicked",
    ] {
        assert!(
            host.contains(callback),
            "missing callback wiring: {callback}"
        );
    }
}
