//! Reused-window reset guard (added 2026-06-14 from the regression-risk audit).
//!
//! The Settings window is REUSED, not recreated, so any transient `*-status` /
//! `*-result` string property survives the next open and describes a STALE
//! action unless `populate_token_status` clears it on every open. That is the
//! project's self-documented #1 recurring regression (CLAUDE.md "recurring UI
//! bug shapes" #1 — the user's lingering "Готово: умная модель (12B)"). The only
//! defense was a hand-maintained list of `set_*(blank())` calls; every new
//! control with a `*-result` property was one forgotten line away from bringing
//! the bug back.
//!
//! This test turns that invariant into a gate: every `*-status` / `*-result`
//! STRING property declared in `ui/settings_panel.slint` must have a matching
//! `set_<name>(` inside `populate_token_status` (which runs on every open, incl.
//! the reuse path). Pure file parse, no UI build — same style as i18n_guard.rs.
//!
//! If this fails: add `win.set_<name>(blank());` (or a fresh reseed) to
//! `populate_token_status` in `settings_controller.rs` for the named property.
//!
//! Scope notes:
//! - Only `settings_panel.slint` (the reused window). Tiles/palette are created
//!   fresh each time, so they have no stale-state invariant.
//! - Only `-status` / `-result` STRING props (the transient text-result naming
//!   convention). Worker-owned in-flight BOOLs (`*_downloading`/`*_updating`) are
//!   deliberately reset by their worker's terminal callback, NOT by populate, so
//!   they are out of scope (including them would false-positive).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts

use std::fs;
use std::path::Path;

/// Every `in-out property <string> NAME` whose NAME ends in `-status`/`-result`.
fn transient_string_props(slint_src: &str) -> Vec<String> {
    const MARKER: &str = "in-out property <string> ";
    let mut out = Vec::new();
    for line in slint_src.lines() {
        let Some(rest) = line.trim_start().strip_prefix(MARKER) else {
            continue;
        };
        // NAME runs until the first `:` (has a default), `;` (no default), or ws.
        let name: String = rest
            .chars()
            .take_while(|c| !matches!(c, ':' | ';' | ' ' | '\t'))
            .collect();
        if name.ends_with("-status") || name.ends_with("-result") {
            out.push(name);
        }
    }
    out
}

/// Extract the `populate_token_status` function body (its source slice). Runs
/// from the fn signature to the next top-level `fn`/`pub fn`/`pub(crate) fn`
/// (column-0, i.e. a sibling item) or to EOF. Good enough for a substring scan.
fn populate_body(rs_src: &str) -> &str {
    let start = rs_src
        .find("fn populate_token_status")
        .expect("populate_token_status not found in settings_controller.rs");
    let after = &rs_src[start..];
    // The first byte after the signature; search for the next top-level item.
    let body = &after["fn populate_token_status".len()..];
    let mut end = body.len();
    for marker in ["\nfn ", "\npub fn ", "\npub(crate) fn "] {
        if let Some(i) = body.find(marker) {
            end = end.min(i);
        }
    }
    &body[..end]
}

#[test]
fn every_transient_status_prop_is_reset_on_reopen() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let slint = fs::read_to_string(root.join("ui/settings_panel.slint"))
        .expect("read ui/settings_panel.slint");
    let rs = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("read settings_controller.rs");
    let body = populate_body(&rs);

    let mut missing: Vec<String> = Vec::new();
    for prop in transient_string_props(&slint) {
        // Slint hyphen-name -> Rust setter: `ai-bearer-status` -> `set_ai_bearer_status(`.
        let setter = format!("set_{}(", prop.replace('-', "_"));
        if !body.contains(&setter) {
            missing.push(format!(
                "{prop} (expected `{setter}` in populate_token_status)"
            ));
        }
    }

    assert!(
        missing.is_empty(),
        "Settings `*-status`/`*-result` props NOT reset on reopen — they will \
         show a STALE value from the previous open (the project's #1 regression \
         class). Add a reset to populate_token_status for:\n{}",
        missing
            .iter()
            .map(|m| format!("  {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Sanity: the parser actually found the known transient props (so a future
/// refactor that breaks the scan can't make the guard vacuously pass).
#[test]
fn parser_finds_the_known_transient_props() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let slint = fs::read_to_string(root.join("ui/settings_panel.slint"))
        .expect("read ui/settings_panel.slint");
    let props = transient_string_props(&slint);
    assert!(
        props.len() >= 10,
        "expected the Settings panel to declare many *-status/*-result props, \
         found {} — the parser or the .slint changed shape: {props:?}",
        props.len()
    );
    assert!(
        props.iter().any(|p| p == "engine-update-status"),
        "expected engine-update-status among the transient props; got {props:?}"
    );
}
