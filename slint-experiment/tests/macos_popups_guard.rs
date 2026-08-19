//! Reachability guard for shared Archive, Help, and Palette windows.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn canonical_runtime_reaches_shared_popup_implementations() {
    let host = read("src/bin/overlay_host_windows.rs");
    let windows = read("src/bin/overlay_host/aux_windows.rs");

    assert!(host.contains("#[path = \"overlay_host/aux_windows.rs\"]"));
    for (open, component) in [
        ("open_archive(", "ArchiveWindow::new()"),
        ("open_help(", "HelpWindow::new()"),
        ("open_palette(", "PaletteWindow::new()"),
    ] {
        assert!(host.contains(open), "canonical host does not call {open}");
        assert!(
            windows.contains(open),
            "shared module does not define {open}"
        );
        assert!(
            windows.contains(component),
            "shared module lacks {component}"
        );
    }
}
