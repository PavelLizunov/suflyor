//! Wiring guard for the macOS Archive, Help, and Palette windows and hotkeys.
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
fn macos_overlay_host_wires_archive_help_palette_windows() {
    let host = read(root(), "src/bin/overlay_host.rs");
    let archive = read(root(), "src/bin/overlay_host/macos_archive.rs");
    let help = read(root(), "src/bin/overlay_host/macos_help.rs");
    let palette = read(root(), "src/bin/overlay_host/macos_palette.rs");

    assert!(host.contains("mod macos_archive;"));
    assert!(host.contains("mod macos_help;"));
    assert!(host.contains("mod macos_palette;"));
    assert!(host.contains("archive_slice.toggle_archive()"));
    assert!(host.contains("help_slice.toggle_help()"));
    assert!(host.contains("palette_slice_for_hotkey.toggle_palette()"));

    assert!(archive.contains("ui::ArchiveWindow::new()"));
    assert!(help.contains("ui::HelpWindow::new()"));
    assert!(palette.contains("ui::PaletteWindow::new()"));
}
