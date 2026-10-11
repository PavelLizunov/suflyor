//! Keep the deep-lock icon centered inside its fixed capsule slot.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

#[test]
fn lock_icon_is_vertically_centred_in_its_fixed_chip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui =
        fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read ui/overlay_bar.slint");
    let lock_chip = ui
        .split("component LockChip inherits Rectangle {")
        .nth(1)
        .and_then(|source| {
            source
                .split_once("// Capsule children")
                .map(|(chip, _)| chip)
        })
        .expect("find LockChip component");

    assert!(
        lock_chip.contains("y: (parent.height - self.height) / 2;"),
        "LockChip's lock image must stay vertically centred when the deep-lock label is visible"
    );
    assert!(
        lock_chip.contains("border-width: 0px;"),
        "the capsule owns the outline"
    );
    assert!(ui.contains("width: root.compact-bar ? 26px : 76px;"));
    assert!(ui.contains("tight: root.compact-bar;"));
    assert!(lock_chip.contains("min-width: 26px;"));
    assert!(!lock_chip.contains("min-width: 30px;"));
    assert!(lock_chip.contains("if root.deep && !root.tight : Text {"));
    assert!(lock_chip.contains("text: @tr(\"AI off\");"));
    assert!(!lock_chip.contains("text: root.tight ?"));
    assert!(!lock_chip.contains("text: @tr(\"Local AI unloaded\")"));
    assert!(lock_chip.contains("accessible-label: root.deep ? @tr(\"Local AI unloaded\") : a11y;"));
}
