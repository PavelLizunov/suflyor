//! Pins the compact Horizon-style read-aloud transport hierarchy.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn active_player_keeps_status_transport_and_secondary_controls_separate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tile = fs::read_to_string(root.join("ui/tile.slint")).expect("read tile.slint");
    let start = tile
        .find("if root.speak-active : Rectangle")
        .expect("active player section");
    let end = tile[start..]
        .find("// ===== Follow-up input")
        .map(|offset| start + offset)
        .expect("section after active player");
    let player = &tile[start..end];

    assert!(player.contains("height: 60px;"));
    assert!(player.contains("padding-top: 8px;"));
    assert!(player.contains("padding-bottom: 8px;"));
    let status = player.find("Preparing voice...").expect("status row");
    let transport = player
        .find("transport := HorizontalLayout")
        .expect("transport");
    let secondary = player
        .find("secondary := HorizontalLayout")
        .expect("secondary controls");
    assert!(status < transport && transport < secondary);
    assert!(player.contains("label: \"-10\";"));
    assert!(player.contains("primary: true;"));
    assert!(player.contains("label: \"+15\";"));
    assert!(player.contains("border-radius: 15px;"));
}

#[test]
fn tile_conversation_fixes_stay_wired() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tile = fs::read_to_string(root.join("ui/tile.slint")).expect("read tile.slint");
    let overlay =
        fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read overlay_bar.slint");
    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read overlay_host_windows.rs");

    assert!(tile.contains("if root.select-mode && !root.capture-pending : TextEdit"));
    assert!(tile.contains("block.kind == 5 && block.lang == \"user\""));
    assert!(overlay.contains("callback restore-tile-clicked();"));
    assert!(host.contains("tile.on_followup_submitted"));
    assert!(host.contains("LAST_CLOSED_READ_TILE"));
}
