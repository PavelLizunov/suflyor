//! Structural guard for explicit read-aloud requests and visible failures.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let tail = source.split_once(start).expect(start).1;
    tail.split_once(end).expect(end).0
}

#[test]
fn explicit_speech_replaces_the_current_utterance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/bin/overlay_host/tile_copy.rs"))
        .expect("read tile_copy.rs");

    let explicit = between(
        &source,
        "pub(crate) fn speak_explicit",
        "pub(crate) fn stop_if_speaking",
    );
    assert!(explicit.contains("overlay_backend::tts::speak(text)"));
    assert!(!explicit.contains("is_speaking"));
    assert!(explicit.contains("reset_pause()"));
    assert!(explicit.contains("mark_speaking(convo_id)"));

    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read overlay_host_windows.rs");
    let read_aloud = fs::read_to_string(root.join("src/bin/overlay_host/read_aloud.rs"))
        .expect("read read_aloud.rs");
    assert!(host.contains("#[path = \"overlay_host/read_aloud.rs\"]"));
    assert!(read_aloud.contains("speak_explicit(text, convo_id);"));
    assert!(read_aloud.contains("speak_explicit(trimmed, convo_id);"));
    assert!(!source.contains("auto_speak_if_idle"));
}

#[test]
fn settings_voice_test_reports_and_resets_a_generic_failure() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let voice = fs::read_to_string(root.join("src/bin/overlay_host/settings_voice.rs"))
        .expect("read settings_voice.rs");
    let settings = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("read settings_controller.rs");
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint"))
        .expect("read settings_panel.slint");

    assert!(voice.contains("w.set_tts_test_status(SharedString::from(status))"));
    assert!(settings.contains("win.set_tts_test_status(SharedString::from(\"\"))"));
    assert!(ui.contains("in-out property <string> tts-test-status;"));
    assert!(ui.contains("if root.tts-test-status != \"\" : Text"));
}

#[test]
fn tile_speaker_tracks_backend_availability_and_rejection() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/bin/overlay_host/tile_copy.rs"))
        .expect("read tile_copy.rs");
    let wire = source
        .split_once("pub(crate) fn wire_speak")
        .expect("wire_speak")
        .1;

    assert!(wire.contains("set_can_speak(overlay_backend::tts::is_available())"));
    assert!(wire.contains("if !speak_explicit(&text, convo_id)"));

    let explicit_state = between(
        &source,
        "fn set_speak_error",
        "pub(crate) fn speak_explicit",
    );
    assert!(explicit_state.contains("tile.set_speak_error(failed)"));
    assert!(explicit_state.contains("set_can_speak(overlay_backend::tts::is_available())"));
    let explicit = between(
        &source,
        "pub(crate) fn speak_explicit",
        "pub(crate) fn stop_if_speaking",
    );
    assert!(explicit.contains("set_speak_error(convo_id, true)"));
    assert!(explicit.contains("set_speak_error(convo_id, false)"));

    let tile = fs::read_to_string(root.join("ui/tile.slint")).expect("read tile.slint");
    assert!(tile.contains("in-out property <bool> speak-error: false;"));
    assert!(tile.contains("if root.speak-error : Text"));
}
