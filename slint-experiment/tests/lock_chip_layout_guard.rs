//! Keep the auto-tiles switch fixed and the independent AI-off item bare.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn auto_tiles_and_ai_off_have_distinct_fixed_slots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read bar");
    let chip = ui
        .split_once("component AutotileChip inherits Rectangle {")
        .expect("chip")
        .1
        .split_once("// Capsule children")
        .expect("chip boundary")
        .0;
    assert!(chip.contains("y: (parent.height - self.height) / 2;"));
    assert!(chip.contains("border-width: 0px;"), "capsule owns outline");
    assert!(ui.contains("width: root.compact-bar ? 26px : 96px;"));
    assert!(ui.contains("tight: root.compact-bar;"));
    assert!(chip.contains("min-width: 26px;"));
    assert!(!chip.contains("min-width: 30px;"));
    assert!(chip.contains("if !root.tight : Text {"));
    assert!(chip.contains("text: root.label;"));
    assert!(
        !chip.contains("root.deep"),
        "unloaded AI must not replace the switch"
    );
    for word in ["silent", "questions", "everything"] {
        assert!(chip.contains(&format!("@tr(\"{word}\")")));
    }
    let indicator = ui
        .split_once("if root.deep-lock : PinChip {")
        .expect("AI off")
        .1
        .split_once("if root.timer-active && root.session-name")
        .expect("next footer item")
        .0;
    assert!(indicator.contains("width: 70px;"));
    assert!(indicator.contains("@image-url(\"../assets/icons/lock.svg\")"));
    assert!(indicator.contains("label: @tr(\"AI off\"); a11y: root.lock-a11y;"));
    assert!(indicator.contains("tint: Theme.on-tint(Theme.danger); weight: 600;"));
    assert!(indicator.contains("root.local-ai-mode-selected(0)"));
    assert!(!indicator.contains("compact-bar"), "present in both forms");
    let bare = ui
        .split_once("component PinChip")
        .expect("bare chip")
        .1
        .split_once("component MoreChip")
        .expect("bare boundary")
        .0;
    assert!(!bare.contains("border-width"));
    assert!(!bare.contains("background:"));
}
