//! Source-level regression guard for the explicit lock-mode menu.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

#[test]
fn lock_menu_is_a_top_level_window_and_hides_deep_for_external_ai() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read overlay bar");
    let menu = fs::read_to_string(root.join("ui/lock_mode_menu.slint")).expect("read lock menu");
    let lock_chip = source
        .split("component LockChip inherits Rectangle {")
        .nth(1)
        .and_then(|rest| rest.split("// Active-indicator chip").next())
        .expect("find LockChip component");

    for label in ["Normal", "Listening", "Unload local AI"] {
        assert!(menu.contains(&format!("@tr(\"{label}\")")));
    }
    assert!(menu.contains("inherits Window"));
    assert!(menu.contains("if root.managed : LockModeMenuRow"));
    assert!(source.contains("callback menu-opened(length, length);"));
    assert!(!lock_chip.contains("PopupWindow"));
    assert!(source.contains("callback lock-mode-selected(int);"));
}
