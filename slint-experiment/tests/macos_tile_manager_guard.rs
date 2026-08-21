//! Reachability guard for shared tile creation and presentation.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn canonical_runtime_reaches_the_shared_tile_presenter() {
    let host = read("src/bin/overlay_host_windows.rs");
    let presenter = read("src/bin/overlay_host/tile_window.rs");

    assert!(host.contains("#[path = \"overlay_host/tile_window.rs\"]"));
    assert!(host.contains("present_tile_window(&tile);"));
    assert!(presenter.contains("pub(crate) fn present_tile_window(tile: &TileWindow)"));
    assert!(presenter.contains("pub(crate) fn apply_tile_hwnd_with_monitor(tile: &TileWindow)"));
    assert!(presenter.contains("let _ = tile.show();"));
}
