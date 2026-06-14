//! Version-drift guard (added 2026-06-14 from the regression-risk audit).
//!
//! The product version lives in TWO hand-synced places: `Cargo.toml` (the
//! binary's `env!("CARGO_PKG_VERSION")`, used by the in-app updater + logs) and
//! `scripts/slint-installer.nsi` (`!define PRODUCT_VERSION`, the installer +
//! Add/Remove-Programs version). CLAUDE.md only says "keep them in sync" — a
//! manual instruction. On a forgotten bump they disagree: the updater compares
//! the WRONG current version against GitHub's latest tag, and Add/Remove-Programs
//! shows a version that doesn't match the running binary.
//!
//! This test makes the drift a hard failure. If it fails: bump BOTH
//! `slint-experiment/Cargo.toml` `version` and `scripts/slint-installer.nsi`
//! `!define PRODUCT_VERSION` to the same value.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts

use std::fs;
use std::path::Path;

/// Pull the value of `!define PRODUCT_VERSION "X"` out of the NSI script.
fn nsi_product_version(nsi: &str) -> Option<String> {
    for line in nsi.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("!define PRODUCT_VERSION") else {
            continue;
        };
        // rest is like `  "0.18.2"` — take the first double-quoted token.
        let start = rest.find('"')? + 1;
        let end = start + rest[start..].find('"')?;
        return Some(rest[start..end].to_string());
    }
    None
}

#[test]
fn cargo_toml_version_matches_nsi_product_version() {
    // The crate's own compiled-in version IS the Cargo.toml `version` — no parse.
    let cargo_version = env!("CARGO_PKG_VERSION");

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let nsi_path = root.join("../scripts/slint-installer.nsi");
    let nsi = fs::read_to_string(&nsi_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", nsi_path.display()));
    let nsi_version =
        nsi_product_version(&nsi).expect("PRODUCT_VERSION not found in slint-installer.nsi");

    assert_eq!(
        cargo_version, nsi_version,
        "version drift: Cargo.toml = {cargo_version}, slint-installer.nsi \
         PRODUCT_VERSION = {nsi_version}. Bump BOTH to the same value."
    );
}
