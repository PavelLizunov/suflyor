#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

const PLIST: &str = include_str!("../macos/Info.plist");
const ENTITLEMENTS: &str = include_str!("../macos/entitlements.plist");
const SCRIPT: &str = include_str!("../scripts/build-macos-app.sh");
const HOST: &str = include_str!("../src/bin/overlay_host_windows.rs");
const SETTINGS_CONTROLLER: &str = include_str!("../src/bin/overlay_host/settings_controller.rs");
const SETTINGS_UI: &str = include_str!("../ui/settings_panel.slint");

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
        "export CARGO_TARGET_DIR=\"$target_dir\"",
        "if [[ \"$target_dir\" == \"/\" ]]",
        "$(basename \"$app_dir\")\" != \"Suflyor.app\"",
        "export CARGO_INCREMENTAL=0",
        "cargo build --locked --release --bin overlay-host",
        "cargo build --locked --release --manifest-path \"$crate_root/../suflyor-tts/Cargo.toml\"",
        "cargo build --locked --release --manifest-path \"$crate_root/../suflyor-teratts/Cargo.toml\"",
        "swift build --package-path \"$mlx_root\" -c release --disable-automatic-resolution",
        "mlx_root=\"$crate_root/../suflyor-mlx\"",
        "if [[ ! -f \"$mlx_root/Package.resolved\" ]]",
        "--manifest-path \"$crate_root/Cargo.toml\"",
        "sidecar_binary=\"$target_dir/release/suflyor-tts\"",
        "tera_binary=\"$target_dir/release/suflyor-teratts\"",
        "for executable in \"$binary\" \"$sidecar_binary\" \"$tera_binary\" \"$mlx_binary\"",
        "[[ ! -x \"$executable\" ]]",
        "plutil -lint \"$crate_root/macos/Info.plist\"",
        "plutil -lint \"$crate_root/macos/entitlements.plist\"",
        "rm -rf -- \"$app_dir\"",
        "mkdir -p \"$macos_dir\" \"$resources_dir\"",
        "cp \"$crate_root/macos/Info.plist\" \"$contents_dir/Info.plist\"",
        "install -m 755 \"$binary\" \"$macos_dir/overlay-host\"",
        "install -m 755 \"$sidecar_binary\" \"$macos_dir/suflyor-tts\"",
        "install -m 755 \"$tera_binary\" \"$macos_dir/suflyor-teratts\"",
        "install -m 755 \"$mlx_binary\" \"$macos_dir/suflyor-mlx\"",
        "install_name_tool -add_rpath '@executable_path/../Frameworks'",
        "xcrun swift-stdlib-tool --copy --platform macosx",
        "find \"$mlx_bin_dir\" -maxdepth 1 -type d -name '*.bundle'",
        "otool -L \"$macos_dir/suflyor-mlx\"",
        "verify_bundle_dependency",
        "unresolved bundled dependency",
        "lipo -archs \"$mlx_binary\"",
        "[[ ! -s \"$resources_dir/AppIcon.icns\" ]]",
        "--entitlements \"$crate_root/macos/entitlements.plist\"",
        "codesign --verify --strict --verbose=2 \"$macos_dir/suflyor-tts\"",
        "codesign --verify --strict --verbose=2 \"$macos_dir/suflyor-teratts\"",
        "codesign --verify --strict --verbose=2 \"$macos_dir/suflyor-mlx\"",
        "codesign --verify --deep --strict --verbose=2 \"$app_dir\"",
        "if \"$macos_dir/suflyor-mlx\" </dev/null >/dev/null 2>&1",
        "suflyor-mlx packaged launch smoke failed",
    ] {
        assert!(SCRIPT.contains(required), "missing script step: {required}");
    }
    assert!(
        !SCRIPT.contains("$crate_root/../suflyor-tts/target/release"),
        "the package must never select a stale per-crate TTS artifact"
    );
    assert!(
        !SCRIPT.contains("if [[ -f \"$sidecar_binary\"")
            && !SCRIPT.contains("if [[ -f \"$tera_binary\""),
        "both sidecars are mandatory, not best-effort"
    );

    let tts_sign = SCRIPT
        .find("codesign --force --sign - --options runtime \"$macos_dir/suflyor-tts\"")
        .expect("missing TTS sidecar signature");
    let tera_sign = SCRIPT
        .find("codesign --force --sign - --options runtime \"$macos_dir/suflyor-teratts\"")
        .expect("missing Tera sidecar signature");
    let mlx_sign = SCRIPT
        .find("codesign --force --sign - --options runtime \"$macos_dir/suflyor-mlx\"")
        .expect("missing MLX sidecar signature");
    let app_sign = SCRIPT
        .find(
            "codesign --force --sign - --options runtime \\\n  --entitlements \"$crate_root/macos/entitlements.plist\" \\\n  \"$app_dir\"",
        )
        .expect("missing app entitlements signature");
    assert!(
        mlx_sign < tts_sign && tts_sign < tera_sign && tera_sign < app_sign,
        "nested executables must be signed before the outer app"
    );
    assert!(SCRIPT.trim_end().ends_with("echo \"$app_dir\""));
}

#[test]
fn script_stays_free_local_packaging() {
    let script = SCRIPT.to_ascii_lowercase();
    for forbidden in ["notarytool ", "hdiutil ", "productbuild ", "pkgbuild "] {
        assert!(!script.contains(forbidden), "unexpected step: {forbidden}");
    }
}

#[test]
fn windows_updater_is_absent_but_backup_remains_on_the_macos_settings_surface() {
    assert!(HOST.contains(
        "#[cfg(windows)]\n#[path = \"overlay_host/settings_updates.rs\"]\nmod settings_updates;"
    ));
    assert!(HOST.contains("#[cfg(windows)]\nuse settings_updates::*;"));
    assert!(SETTINGS_CONTROLLER.contains("#[cfg(windows)]\nuse super::wire_updates;"));
    assert!(SETTINGS_CONTROLLER.contains("#[cfg(windows)]\n    wire_updates(&win);"));
    assert!(SETTINGS_UI.contains("label: Platform.is-macos ? @tr(\"Backup\") : @tr(\"Updates\")"));
    assert!(SETTINGS_UI.contains("if root.active-tab == 4 : VerticalLayout"));
    assert!(SETTINGS_UI.contains("if !Platform.is-macos : SettingsCard {\n                            title: @tr(\"Updates\")"));
    assert!(SETTINGS_UI.contains("title: @tr(\"Backup / transfer settings\")"));
}
