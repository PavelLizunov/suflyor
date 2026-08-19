//! Wiring guard for the macOS UI bar callbacks.
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
fn macos_overlay_host_wires_bar_callbacks() {
    let host = read(root(), "src/bin/overlay_host.rs");

    for required_callback in [
        "window.on_quit_confirm",
        "window.on_drag_start_requested",
        "window.on_text_ask_clicked",
        "window.on_open_settings_clicked",
        "window.on_mic_toggle_clicked",
        "window.on_capture_clicked",
        "window.on_spawn_tile_clicked",
        "window.on_pause_toggle_clicked",
        "window.on_help_clicked",
        "window.on_archive_clicked",
    ] {
        assert!(
            host.contains(required_callback),
            "missing callback wiring: {required_callback}"
        );
    }
}
