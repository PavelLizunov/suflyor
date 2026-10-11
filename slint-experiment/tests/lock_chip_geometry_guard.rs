//! Keep each auto-tiles glyph centered and preserve the silent tremble.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn lock_chip_icon_has_an_explicit_vertical_center() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint");
    let source = fs::read_to_string(path).expect("read overlay bar");
    let lock_chip = source
        .split("component AutotileChip inherits Rectangle {")
        .nth(1)
        .and_then(|rest| rest.split_once("// Capsule children").map(|(chip, _)| chip))
        .expect("find AutotileChip component");

    let lock_icon = lock_chip
        .split("colorize: root.mode != 1 ? Theme.on-tint(root.accent) : Theme.text-secondary;")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("find auto-tiles image");

    assert!(lock_chip.contains("min-width: 26px;"));
    assert!(lock_icon.contains("width: 14px;"));
    assert!(lock_icon.contains("height: 14px;"));
    assert!(lock_chip.contains("root.mode == 2 ? @image-url(\"../assets/icons/flame.svg\")"));
    assert!(
        lock_chip.contains("property <color> accent: root.mode == 2 ? Theme.cat-orange : Theme.warning;")
    );
    assert!(lock_chip.contains("running: root.locked;"));
    assert!(lock_chip.contains("interval: 90ms;"));
    assert!(lock_icon.contains("duration: 85ms;"));
    assert!(lock_icon.contains("easing: ease-in-out;"));
    assert!(lock_chip.contains("property <bool> locked: root.mode == 0;"));
    assert!(lock_chip.contains("property <bool> unlocked: root.mode == 1;"));
    assert!(lock_chip.contains("? Theme.tint(accent)"));
    assert!(lock_icon.contains(
        "transform-rotation: root.locked ? (root.phase ? 7deg : -7deg) : 0deg;"
    ));

    // HorizontalLayout centers only horizontally; explicitly center the SVG.
    assert!(
        lock_icon.contains("y: (parent.height - self.height) / 2;"),
        "Auto-tiles SVG must stay vertically centered beside every label"
    );
}
