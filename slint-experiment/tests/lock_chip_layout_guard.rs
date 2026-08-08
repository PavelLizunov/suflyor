//! Keep the deep-lock icon vertically centred when its status label expands the chip.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

#[test]
fn lock_icon_is_vertically_centred_in_its_expanding_chip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui =
        fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read ui/overlay_bar.slint");
    let lock_chip = ui
        .split("component LockChip inherits Rectangle {")
        .nth(1)
        .and_then(|source| source.split("// Active-indicator chip:").next())
        .expect("find LockChip component");

    assert!(
        lock_chip.contains("y: (parent.height - self.height) / 2;"),
        "LockChip's lock image must stay vertically centred when the deep-lock label is visible"
    );
}
