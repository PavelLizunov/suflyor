//! Keep the deep-lock glyph centered when its text label is shown.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn lock_chip_icon_has_an_explicit_vertical_center() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint");
    let source = fs::read_to_string(path).expect("read overlay bar");
    let lock_chip = source
        .split("component LockChip inherits Rectangle {")
        .nth(1)
        .and_then(|rest| rest.split_once("// Capsule children").map(|(chip, _)| chip))
        .expect("find LockChip component");

    let lock_icon = lock_chip
        .split("source: root.unlocked ? @image-url(\"../assets/icons/unlock.svg\") : @image-url(\"../assets/icons/lock.svg\");")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("find LockChip lock/unlock image");

    assert!(lock_chip.contains("min-width: 26px;"));
    assert!(lock_icon.contains("width: 14px;"));
    assert!(lock_icon.contains("height: 14px;"));
    assert!(lock_icon.contains("colorize: root.locked ? root.accent : Theme.text-secondary;"));
    assert!(
        lock_chip.contains("in property <color> accent: root.deep ? Theme.danger : Theme.warning;")
    );
    assert!(lock_chip.contains("running: root.locked && !root.deep;"));
    assert!(lock_icon.contains(
        "transform-rotation: root.locked && !root.deep ? (root.phase ? 7deg : -7deg) : 0deg;"
    ));

    // HorizontalLayout centers on its horizontal axis only. The label added by
    // deep lock therefore must not rely on the layout to center the 14px SVG.
    assert!(
        lock_icon.contains("y: (parent.height - self.height) / 2;"),
        "LockChip's lock/unlock SVG must explicitly stay vertically centered; this catches the top-aligned deep-lock regression in MCP screenshots"
    );
}
