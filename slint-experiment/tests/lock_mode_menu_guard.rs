//! Source-level regression guard for the explicit lock-mode menu.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

#[test]
fn lock_chip_exposes_explicit_modes_and_hides_deep_for_external_ai() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint");
    let source = fs::read_to_string(path).expect("read overlay bar");
    let lock_chip = source
        .split("component LockChip inherits Rectangle {")
        .nth(1)
        .and_then(|rest| rest.split("// Active-indicator chip").next())
        .expect("find LockChip component");

    for label in ["Normal", "Listening", "Unload local AI"] {
        assert!(lock_chip.contains(&format!("@tr(\"{label}\")")));
    }
    assert!(lock_chip.contains("if root.managed : LockModeMenuRow"));
    assert!(lock_chip.contains("callback mode-selected(int);"));
    assert!(source.contains("callback lock-mode-selected(int);"));
}
