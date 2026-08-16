#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

const PLIST: &str = include_str!("../macos/Info.plist");
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
    ] {
        assert!(has_plist_value(key, value), "invalid plist entry: {key}");
    }
}

#[test]
fn script_builds_and_ad_hoc_signs_the_app() {
    for required in [
        "set -euo pipefail",
        "BASH_SOURCE",
        "export CARGO_INCREMENTAL=0",
        "cargo build --locked --release --bin overlay-host",
        "--manifest-path \"$crate_root/Cargo.toml\"",
        "mkdir -p \"$macos_dir\" \"$resources_dir\"",
        "cp \"$crate_root/macos/Info.plist\" \"$contents_dir/Info.plist\"",
        "chmod 755 \"$macos_dir/overlay-host\"",
        "codesign --force --sign - --options runtime",
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
