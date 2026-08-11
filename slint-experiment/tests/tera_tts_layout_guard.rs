//! Source-level regression guard for the experimental TeraTTS settings section.
//!
//! The install status can wrap while a download is in flight.  The section must
//! retain its natural height; otherwise Slint compresses it and the speed picker
//! overlays the Install model / Cancel controls.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::PathBuf;

fn settings_source() -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join("ui/settings_panel.slint"))
        .expect("settings panel source must be readable")
}

#[test]
fn tera_install_controls_keep_natural_height_before_speed_picker() {
    let source = settings_source();
    let marker = "if root.tts-engine-index == 1 : VerticalLayout {";
    let tera_start = source
        .find(marker)
        .expect("Tera settings section must exist");
    let tera = &source[tera_start..];
    let speed = tera
        .find("// Speed preset")
        .expect("speed picker must follow Tera section");
    let tera = &tera[..speed];

    assert!(
        tera.contains("vertical-stretch: 0;"),
        "Tera section must not be compressed into the speed picker"
    );
    assert!(
        tera.contains("min-height: root.tera-install-phase != 0 ? 128px : 108px;"),
        "Tera status and install controls need their own reserved vertical space"
    );
    let install = tera
        .find("@tr(\"Install model\")")
        .expect("install button must exist");
    let cancel = tera
        .find("@tr(\"Cancel\")")
        .expect("cancel button must exist");
    assert!(
        install < cancel,
        "cancel must stay beside the install control"
    );
}
