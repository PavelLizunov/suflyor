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

#[test]
fn local_model_cards_reset_their_non_status_state_on_reopen() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rs = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("read settings_controller.rs");
    let body = populate_body(&rs);
    for setter in [
        "set_now_title(",
        "set_now_detail(",
        "set_now_state(",
        "set_now_notice(",
        "set_local_hardware_line(",
        "set_local_connect_open(",
        "set_local_connect_auto(",
        "set_local_download_profile(",
        "set_local_strata_job(",
        "set_local_choice_status(",
    ] {
        assert!(
            body.contains(setter),
            "missing local model card reset: {setter}"
        );
    }
    assert!(body.contains("if !win.get_quality_downloading()"));
    assert!(body.contains("if !win.get_strata_busy()"));
}

#[test]
fn tab_changes_reset_the_shared_content_scroll() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).unwrap();
    assert!(ui.contains("content-scroll := ScrollView {"));
    let tab_change = ui.split_once("changed active-tab => {").unwrap().1;
    let tab_change = tab_change.split_once("    }").unwrap().0;
    assert!(tab_change.contains("content-scroll.viewport-y = 0;"));
    assert!(tab_change.contains("if root.active-tab == 4 { root.backup-status-requested(); }"));
    for title in [
        "Profile + context",
        "Coaching",
        "STT",
        "Auto-tiles",
        "Knowledge base",
        "Hotkeys",
        "Diagnostics",
        "Components",
    ] {
        assert!(!ui.contains(&format!("title: @tr(\"{title}\");")));
    }
    // The updater has one explicit card heading; other tabs use the page title.
    assert_eq!(ui.matches("title: @tr(\"Updates\");").count(), 1);
    assert!(!ui.contains("Tip: Save writes"));
}

#[test]
fn component_actions_have_equal_width_and_hermes_has_no_primary_action() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).unwrap();
    let components = ui
        .split_once("if root.active-tab == 15 : VerticalLayout {")
        .unwrap()
        .1
        .split_once("if root.active-tab == 7 : VerticalLayout {")
        .unwrap()
        .0;
    for label in ["Open", "Install"] {
        assert!(components.contains(&format!(
            "width: 132px;\n                                        text: @tr(\"{label}\");"
        )));
    }
    let hermes = ui
        .split_once("if root.active-tab == 16 : VerticalLayout {")
        .unwrap()
        .1
        .split_once("if root.active-tab == 12 : VerticalLayout {")
        .unwrap()
        .0;
    assert!(!hermes.contains("primary: true"));
}

#[test]
fn language_changes_refresh_key_presence_without_resetting_other_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rs = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs")).unwrap();
    let language = rs
        .split_once("win.on_language_selected")
        .unwrap()
        .1
        .split_once("// Colour-scheme switch")
        .unwrap()
        .0;
    assert!(language.contains("key_presence_statuses(&snap)"));
    assert!(language.contains("set_groq_api_key_status(SharedString::from(groq))"));
    assert!(!language.contains("populate_token_status("));
    assert!(populate_body(&rs).contains("key_presence_statuses(&c)"));
}

#[test]
fn hermes_fold_and_kb_count_are_reseeded_on_every_open() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rs = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs")).unwrap();
    let body = populate_body(&rs);
    assert!(body.contains("win.set_hermes_more_open(false)"));
    assert!(body.contains("win.set_kb_entry_count(overlay_backend::kb::all().len() as i32)"));
    assert!(body.contains("populate_backup_status(win, &c)"));
}

#[test]
fn reorganised_tabs_keep_their_controls_and_shared_palette() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).unwrap();
    let rs = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs")).unwrap();
    let tab = |index: i32| {
        ui.split_once(&format!(
            "if root.active-tab == {index} : VerticalLayout {{"
        ))
        .unwrap()
        .1
        .split("if root.active-tab == ")
        .next()
        .unwrap()
    };
    assert!(!tab(0).contains("root.tile-body-opacity"));
    let interface = tab(3);
    assert_eq!(interface.matches("SettingsCard {").count(), 1);
    assert!(
        interface.find("root.tile-monitors").unwrap()
            < interface.find("root.tile-body-opacity").unwrap()
    );
    assert!(interface.contains("root.tile-body-opacity-changed(v)"));
    assert!(!interface.contains("@tr(\"Scheme:\")"));
    let audio = tab(6);
    let microphone = audio
        .split_once("@tr(\"Microphone\")")
        .unwrap()
        .1
        .split_once("@tr(\"System audio - headphones / speakers\")")
        .unwrap()
        .0;
    assert!(microphone.contains("root.audio-devices-refresh()"));
    let storage = audio.split_once("title: @tr(\"Storage\")").unwrap().1;
    for callback in [
        "record-audio-changed",
        "retention-changed",
        "open-recordings-clicked",
        "journal-retention-changed",
    ] {
        assert!(storage.contains(&format!("root.{callback}(")));
    }
    let hermes = tab(16);
    let (connect, advanced) = hermes
        .split_once("if root.hermes-more-open : WrapBlock")
        .unwrap();
    for callback in [
        "hermes-bridge-toggled",
        "hermes-plugin-install",
        "hermes-api-setup",
        "hermes-api-test",
    ] {
        assert!(connect.contains(&format!("root.{callback}(")));
    }
    let advanced = advanced
        .split_once("title: @tr(\"What you get after setup\")")
        .unwrap()
        .0;
    assert_eq!(advanced.matches("text: @tr(\"Save\")").count(), 2);
    for callback in [
        "hermes-bridge-host-save",
        "hermes-bridge-port-save",
        "hermes-bridge-regen-token",
        "hermes-api-url-save",
        "hermes-api-key-save",
    ] {
        assert!(advanced.contains(&format!("root.{callback}(")));
    }
    assert!(tab(9).contains("root.kb-open-clicked()"));
    assert!(rs.contains("super::open_palette(&palette, &tiles, &state, &overlay)"));
    let hermes_rs =
        fs::read_to_string(root.join("src/bin/overlay_host/settings_hermes.rs")).unwrap();
    assert!(hermes_rs.contains("w.invoke_backup_status_requested()"));
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

/// The Updates tab's "a check found a release" state is bool/url, NOT a
/// `*-status`/`*-result` string, so the scan above deliberately skips it. But it
/// is the SAME stale-on-reopen class (CLAUDE.md recurring bug #1): if a check
/// surfaced "⬇ Update now" and the user reopens Settings, populate_token_status
/// must clear `update-available` / `update-checking` / `update-download-url`, or
/// the reused window shows a stale offer for a release whose URL it may no longer
/// hold, or a check button stuck on "Checking…" (audit Q2). Unlike the in-flight
/// worker bools (`*_downloading`/`*_updating`, reset by their worker callback),
/// these three are check-RESULT state and must be reset by populate. This is a
/// fixed, named list (not the generic scan) because these specific props are the
/// known gap the string-convention scan can't see.
#[test]
fn updates_tab_check_result_is_reset_on_reopen() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rs = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("read settings_controller.rs");
    let body = populate_body(&rs);

    let missing: Vec<&str> = [
        "set_update_available(",
        "set_update_checking(",
        "set_update_download_url(",
    ]
    .into_iter()
    .filter(|setter| !body.contains(setter))
    .collect();

    assert!(
        missing.is_empty(),
        "Updates-tab check result NOT reset on reopen — a stale \"⬇ Update now\" \
         button / \"Checking…\" state will survive the next Settings open (same \
         class as the user's lingering \"Готово\"). Add to populate_token_status: \
         {missing:?}"
    );
}
