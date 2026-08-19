//! Wiring guard for the macOS MacTileManager.
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
fn macos_overlay_host_wires_tile_manager() {
    let host = read(root(), "src/bin/overlay_host.rs");
    let manager = read(root(), "src/bin/overlay_host/macos_tile_manager.rs");

    assert!(host.contains("mod macos_tile_manager;"));
    assert!(host.contains("MacTileManager::new()"));
    assert!(host.contains("tile_manager.present_tile"));

    assert!(manager.contains("ui::TileWindow::new()"));
    assert!(manager.contains("slint_replay::native::window::configure_floating"));
    assert!(manager.contains("slint_replay::native::window::raise_key_front"));
}
