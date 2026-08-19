#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

const PLIST: &str = include_str!("../macos/Info.plist");
const ENTITLEMENTS: &str = include_str!("../macos/entitlements.plist");
const SCRIPT: &str = include_str!("../scripts/build-macos-app.sh");

fn has_plist_value(key: &str, value: &str) -> bool {
    let key = format!("<key>{key}</key>");
    PLIST
        .split_once(&key)
        .is_some_and(|(_, rest)| rest.trim_start().starts_with(value))
}

#[test]
fn manifest_keeps_the_production_macos_identity() {
    for (key, value) in [
        ("CFBundleName", "<string>Suflyor</string>"),
        ("CFBundleExecutable", "<string>overlay-host</string>"),
        (
            "CFBundleIdentifier",
            "<string>com.ninitux.suflyor.macos</string>",
        ),
        ("CFBundlePackageType", "<string>APPL</string>"),
        (
            "CFBundleShortVersionString",
            concat!("<string>", env!("CARGO_PKG_VERSION"), "</string>"),
        ),
        (
            "CFBundleVersion",
            concat!("<string>", env!("CARGO_PKG_VERSION"), "</string>"),
        ),
        ("LSMinimumSystemVersion", "<string>14.2</string>"),
        ("LSUIElement", "<true/>"),
        ("NSHighResolutionCapable", "<true/>"),
        (
            "NSMicrophoneUsageDescription",
            "<string>Suflyor captures your microphone to transcribe what you say",
        ),
        (
            "NSAudioCaptureUsageDescription",
            "<string>Suflyor captures system audio to transcribe meeting participants.",
        ),
    ] {
        assert!(has_plist_value(key, value), "invalid plist entry: {key}");
    }
}

#[test]
fn plist_audio_capture_purpose_string_is_non_empty() {
    let key = "<key>NSAudioCaptureUsageDescription</key>";
    let (_, rest) = PLIST
        .split_once(key)
        .expect("missing audio capture purpose string key");
    let value = rest
        .trim_start()
        .strip_prefix("<string>")
        .and_then(|rest| rest.split_once("</string>"))
        .map(|(value, _)| value.trim());
    let value = value.expect("purpose string must be a <string> value");
    assert!(
        value.len() >= 20 && value.to_ascii_lowercase().contains("system audio"),
        "purpose string must be a real sentence about system audio, got: {value:?}"
    );
}

#[test]
fn plist_microphone_purpose_string_is_non_empty() {
    // TCC shows this verbatim; an empty value makes the prompt look broken
    // and Apple rejects it in review, so guard against accidental blanking.
    let key = "<key>NSMicrophoneUsageDescription</key>";
    let (_, rest) = PLIST.split_once(key).expect("missing purpose string key");
    let value = rest
        .trim_start()
        .strip_prefix("<string>")
        .and_then(|rest| rest.split_once("</string>"))
        .map(|(value, _)| value.trim());
    let value = value.expect("purpose string must be a <string> value");
    assert!(
        value.len() >= 20 && value.to_ascii_lowercase().contains("microphone"),
        "purpose string must be a real sentence about the microphone, got: {value:?}"
    );
}

#[test]
fn entitlements_grant_microphone_access() {
    assert!(
        ENTITLEMENTS.contains("<key>com.apple.security.device.audio-input</key>"),
        "entitlements must declare the audio-input device right"
    );
    assert!(
        ENTITLEMENTS.contains("<true/>"),
        "the audio-input entitlement must be enabled"
    );
}

#[test]
fn script_builds_and_ad_hoc_signs_the_app() {
    for required in [
        "set -euo pipefail",
        "BASH_SOURCE",
        "export CARGO_INCREMENTAL=0",
        "cargo build --locked --release --bin overlay-host",
        "cargo build --locked --release --manifest-path \"$crate_root/../suflyor-tts/Cargo.toml\"",
        "--manifest-path \"$crate_root/Cargo.toml\"",
        "mkdir -p \"$macos_dir\" \"$resources_dir\"",
        "cp \"$crate_root/macos/Info.plist\" \"$contents_dir/Info.plist\"",
        "chmod 755 \"$macos_dir/overlay-host\"",
        "codesign --force --sign - --options runtime",
        "--entitlements \"$crate_root/macos/entitlements.plist\"",
        "codesign --verify --deep --strict --verbose=2 \"$app_dir\"",
    ] {
        assert!(SCRIPT.contains(required), "missing script step: {required}");
    }
    assert!(SCRIPT.trim_end().ends_with("echo \"$app_dir\""));
}

#[test]
fn script_stays_free_local_packaging() {
    let script = SCRIPT.to_ascii_lowercase();
    for forbidden in [
        "developer id",
        "notar",
        "hdiutil",
        "productbuild",
        "pkgbuild",
    ] {
        assert!(!script.contains(forbidden), "unexpected step: {forbidden}");
    }
}
