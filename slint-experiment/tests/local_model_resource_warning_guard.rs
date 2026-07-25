//! Guard the Settings resource warning against disagreeing with the backend's
//! 12B fallback and optional-projector rules.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

const WITH_12B_VISION: &str = "[!] Selected 12B: the model uses about 6.3 GiB on disk; the installed vision projector adds about 167 MiB. The measured setup used about 9.5 GB of VRAM, but actual runtime memory depends on the PC.";
const TEXT_ONLY_12B: &str = "[!] Selected 12B: the model uses about 6.3 GiB on disk. Its optional 167 MiB vision projector is not installed, so the 12B runs text-only. The measured setup used about 9.5 GB of VRAM, but actual runtime memory depends on the PC.";
const FALLBACK_4B: &str = "[!] 12B is selected, but its complete model file is unavailable, so local AI falls back to 4B: 4.6 GiB on disk plus about 1.8 GiB for vision (about 6.4 GiB total). Runtime RAM/VRAM is additional and is not calibrated for every Windows GPU.";
const SELECTED_4B: &str = "[!] Selected 4B: the model uses about 4.6 GiB on disk; vision adds about 1.8 GiB (about 6.4 GiB total). Runtime RAM/VRAM is additional and is not calibrated for every Windows GPU.";

#[test]
fn resource_warning_tracks_the_effective_model_and_projector() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let slint = fs::read_to_string(root.join("ui/settings_panel.slint"))
        .expect("read ui/settings_panel.slint");
    let compact: String = slint.chars().filter(|ch| !ch.is_whitespace()).collect();
    let expected: String = format!(
        "text:root.ai-local-quality&&root.quality-model-present?root.quality-vision-present?@tr(\"{WITH_12B_VISION}\"):@tr(\"{TEXT_ONLY_12B}\"):root.ai-local-quality?@tr(\"{FALLBACK_4B}\"):@tr(\"{SELECTED_4B}\");"
    )
    .chars()
    .filter(|ch| !ch.is_whitespace())
    .collect();

    assert!(
        compact.contains(&expected),
        "the resource warning must cover 12B with vision, 12B text-only, a missing-12B fallback, and ordinary 4B"
    );
}
