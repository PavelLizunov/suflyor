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
    let archive = read("src/bin/overlay_host/aux_windows/archive.rs");
    let help_palette = read("src/bin/overlay_host/aux_windows/help_palette.rs");

    assert!(host.contains("#[path = \"overlay_host/aux_windows.rs\"]"));
    assert!(windows.contains("#[path = \"aux_windows/archive.rs\"]"));
    assert!(windows.contains("#[path = \"aux_windows/help_palette.rs\"]"));
    for (open, component, implementation) in [
        ("open_archive(", "ArchiveWindow::new()", archive.as_str()),
        ("open_help(", "HelpWindow::new()", help_palette.as_str()),
        (
            "open_palette(",
            "PaletteWindow::new()",
            help_palette.as_str(),
        ),
    ] {
        assert!(host.contains(open), "canonical host does not call {open}");
        assert!(windows.contains(open.trim_end_matches('(')));
        assert!(
            implementation.contains(open),
            "shared module does not define {open}"
        );
        assert!(
            implementation.contains(component),
            "shared module lacks {component}"
        );
    }
}
