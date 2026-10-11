//! Accepted bar audit contracts; runtime rendering remains a native-worker gate.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn source(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).expect(path)
}

fn section<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
    src.split_once(start)
        .expect(start)
        .1
        .split_once(end)
        .expect(end)
        .0
}

#[test]
fn stealth_fault_is_the_only_new_blink_and_both_chip_sites_share_its_phase() {
    let bar = source("ui/overlay_bar.slint");
    let fault = section(
        &bar,
        "in-out property <bool> stealth-fault:",
        "property <string> presented-status:",
    );
    for binding in [
        "interval: 420ms;",
        "running: root.stealth-fault;",
        "root.stealth-fault-phase = !root.stealth-fault-phase;",
        "changed stealth-fault => { root.stealth-fault-phase = true; }",
    ] {
        assert!(fault.contains(binding), "fault timer: {binding}");
    }
    assert_eq!(
        bar.matches("Timer {").count(),
        2,
        "only auto-tiles and fault timers"
    );
    assert!(bar.contains("interval: 90ms;\n        running: root.locked;"));
    assert!(bar.contains("root.stealth-fault ? Theme.danger : root.status-color"));
    assert!(bar.contains("border-color: root.stealth-fault ? Theme.danger : root.status-color;"));
    assert!(bar.contains("? Theme.on-tint(root.pill-color) : Theme.text-secondary;"));
    assert!(bar.contains("? Theme.danger.with-alpha(0.30) : Theme.bg-elevated;"));
    for binding in [
        "solid: root.stealth-fault && root.stealth-fault-phase;",
        "fault-outline: root.stealth-fault && !root.stealth-fault-phase;",
        "accent: root.stealth-fault ? Theme.danger : Theme.cat-pink;",
    ] {
        assert_eq!(
            bar.matches(binding).count(),
            2,
            "full/compact and mini: {binding}"
        );
    }
    let chip = section(&bar, "component BarChip", "component PinChip");
    assert!(
        chip.contains("background: root.fault-outline ? transparent : root.solid ? root.accent")
    );
    assert!(chip.contains("border-width: root.fault-outline ? Metrics.border-thin : 0px;"));
    assert!(chip.contains("duration: root.solid || root.fault-outline ? 0ms : 80ms;"));
    assert_eq!(chip.matches("root.solid ? Theme.bg-surface").count(), 2);
    assert_eq!(
        chip.matches("root.fault-outline || active ? Theme.on-tint(accent)")
            .count(),
        2
    );
    assert!(bar.contains("root.stealth-fault ? @tr(\"visible!\")"));
    // The word is not conditional on the blink phase; compact/mini remain icon-only.
    assert!(!bar.contains("root.stealth-fault-phase ? @tr"));
}

#[test]
fn small_text_uses_readable_tokens_and_the_pin_hint_is_not_dimmed() {
    let bar = source("ui/overlay_bar.slint");
    let hint = section(&bar, "if root.pins.length == 0 : PinChip {", "clicked =>");
    assert!(hint.contains("tint: Theme.text-secondary;"));
    assert!(!hint.contains("opacity:"));
    let pin = section(&bar, "component PinChip", "component MoreChip");
    assert!(pin.contains("opacity: root.action-enabled ? 1 : 0.35;"));
    let record = section(&bar, "component RecordBtn", "// ===== Main overlay bar");
    assert!(record.contains("color: root.recording ? Theme.text-primary : Theme.text-secondary;"));
    assert!(record.contains("font-weight: 600;"));
    assert!(record.contains("background: recording ? Theme.danger.with-alpha(0.2) : transparent;"));
    assert!(record.contains("tint: root.recording ? Theme.danger : Theme.text-secondary;"));
    let lock_row = section(
        &bar,
        "export component LockModeMenuRow",
        "component AutotileChip",
    );
    assert!(lock_row.contains("color: root.current ? Theme.on-tint(root.mode-color)"));
    assert!(lock_row.contains("color: Theme.on-tint(root.mode-color);"));
    let bootstrap_chip = section(&bar, "component Chip", "export component LockModeMenuRow");
    assert!(bootstrap_chip.contains("color: active ? Theme.on-tint(accent)"));
}

#[test]
fn menu_feedback_targets_and_confirmation_keep_the_grid() {
    let menu = source("ui/bar_menu.slint");
    for binding in [
        "row-touch.pressed ? Theme.tint(Theme.accent)",
        "row-touch.has-hover ? Theme.border-strong.with-alpha(0.22)",
        "border-color: pin-touch.has-hover ? Theme.accent : Theme.border-strong;",
        "pin-touch.pressed || pin-touch.has-hover ? Theme.tint(Theme.accent)",
        "pin-touch.pressed || pin-touch.has-hover ? Theme.on-tint(Theme.accent)",
        "x: 0px; y: 0px; height: parent.height;",
    ] {
        assert!(menu.contains(binding), "menu feedback: {binding}");
    }
    assert_eq!(
        menu.matches("height: 24px; x: 0px; y: (parent.height - self.height) / 2;")
            .count(),
        3
    );
    assert_eq!(
        menu.matches("width: parent.width; border-radius: Metrics.radius-field;")
            .count(),
        3
    );
    assert!(source("ui/metrics.slint").contains("radius-field: 8px;"));
    let yes = section(
        &menu,
        "if root.confirmation == r.id : Rectangle {",
        "if root.confirmation != r.id",
    );
    assert!(yes.contains("background: Theme.danger;"));
    assert!(yes.contains("text: @tr(\"Yes\"); color: Theme.bg-surface;"));
    let no = section(&menu, "width: root.col-pin;", "if r.pinnable");
    assert!(no.contains("background: transparent;"));
    assert!(no.contains("border-width: Metrics.border-thin; border-color: Theme.border-strong;"));
    let bar = source("ui/overlay_bar.slint");
    let pin = section(&bar, "component PinChip", "component MoreChip");
    assert!(pin.contains("pin-touch.pressed ? Theme.tint(Theme.accent)"));
    assert!(pin.contains("pin-touch.has-hover ? Theme.border-strong.with-alpha(0.22)"));
}

#[test]
fn save_failure_keeps_rollback_and_shows_a_timed_translated_footer() {
    let menu = source("ui/bar_menu.slint");
    for binding in [
        "root.pin-error-ticks > 0 ? 24px : 0px",
        "pin-save-failed => { root.pin-error-ticks = 40; }",
        "running: root.pin-error-ticks > 0;",
        "triggered => { root.pin-error-ticks -= 1; }",
        "if root.pin-error-ticks > 0 : Text",
        "height: root.footer-height;",
        "text: @tr(\"Could not save. Try again.\");",
        "color: Theme.on-tint(Theme.danger); font-size: Metrics.font-label;",
        "+ root.footer-height;",
    ] {
        assert!(menu.contains(binding), "save error footer: {binding}");
    }
    let timer = section(&menu, "pin-save-failed =>", "callback action");
    assert!(
        timer.contains("interval: 100ms;"),
        "40 ticks = about four seconds"
    );
    let host = source("src/bin/overlay_host/bar_menu.rs");
    let failure = section(
        &host,
        "if let Err(e) = overlay_backend::config::save(&c)",
        "pins_model(&c.bar_pins)",
    );
    assert!(
        failure.find("c.bar_pins = previous;").unwrap()
            < failure.find("menu.invoke_pin_save_failed();").unwrap()
    );
    assert!(failure.contains("return;"));
    assert!(host.contains("menu.on_footer_resized"));
    assert!(host.contains("slint_replay::bar_form::clamp_menu_origin("));
    assert!(host.contains("+ menu.get_footer_height()"));
    assert!(host.contains("let logical_height = menu_logical_height(&menu);"));
}

#[test]
fn info_progress_is_visible_and_restores_the_prior_status_on_all_completions() {
    let bar = source("ui/overlay_bar.slint");
    let status = section(&bar, "// Bottom row:", "if root.deep-lock");
    assert!(status.contains("root.status-tone == StatusTone.info"));
    assert!(status.contains("? Theme.on-tint(Theme.accent)"));
    assert!(status.contains("font-weight: 600;"));
    assert!(status.contains("root.status-tone != StatusTone.info"));
    let copy = source("src/bin/overlay_host/status_copy.rs");
    let begin = section(
        &copy,
        "pub(super) fn begin_bar_operation",
        "pub(super) fn finish",
    );
    assert!(begin.contains("(bar.get_status_text(), bar.get_status_tone())"));
    assert!(begin.contains("bar.set_status_tone(crate::ui::StatusTone::Info)"));
    let finish = section(
        &copy,
        "pub(super) fn finish_bar_operation",
        "/// Refresh auto-tiles",
    );
    assert!(finish.contains("bar.set_status_text(previous.0)"));
    assert!(finish.contains("bar.set_status_tone(previous.1)"));
    let host = source("src/bin/overlay_host_windows.rs");
    let lifecycle = section(&host, "LockAction::EnterDeepLock =>", "// ===== Bar drag");
    assert_eq!(lifecycle.matches("begin_bar_operation(").count(), 2);
    assert_eq!(lifecycle.matches("finish_bar_operation(").count(), 3);
    assert!(lifecycle.contains("LockStatus::Unlocking"));
    assert!(lifecycle.contains("LockStatus::Unloading"));
    assert!(lifecycle.contains("spawn_mlx_runtime_error"));
    assert!(lifecycle.contains("overlay_backend::deep_lock::unlock_failed_notice(ru)"));
    let bridge = source("src/bin/overlay_host/tile_controller.rs");
    assert!(bridge.contains("if o.get_operation_active()"));
}

#[test]
fn listening_and_accessible_copy_are_bilingual_and_restore_has_one_name() {
    let bar = source("ui/overlay_bar.slint");
    let cue = section(
        &bar,
        "if (root.bar-form == 0 || root.bar-form == 1) && root.timer-active",
        "if root.deep-lock",
    );
    for binding in [
        "!root.session-paused",
        "root.last-transcript-line == \"\"",
        "!root.stealth-fault",
        "root.status-tone != StatusTone.warning",
        "root.status-tone != StatusTone.danger",
        "@tr(\"listening…\")",
        "font-size: Metrics.font-label; color: Theme.text-muted;",
    ] {
        assert!(cue.contains(binding), "listening condition: {binding}");
    }
    let po = source("translations/ru/LC_MESSAGES/slint-replay.po");
    for (en, ru) in [
        (
            "Hold to ask with your voice",
            "Удерживайте: вопрос своим голосом",
        ),
        (
            "Hold to ask about what they said",
            "Удерживайте: вопрос по словам собеседника",
        ),
        ("Start the session", "Начать сессию"),
        ("Stop the session", "Остановить сессию"),
        ("listening…", "слушаю…"),
        (
            "Could not save. Try again.",
            "Не удалось сохранить. Повторите.",
        ),
        ("Restore closed tile", "Вернуть закрытый тайл"),
        ("Expand the bar", "Развернуть панель"),
    ] {
        assert!(
            po.contains(&format!("msgid \"{en}\"\nmsgstr \"{ru}\"")),
            "translation: {en}"
        );
    }
    assert!(bar.contains(
        "a11y: root.timer-active ? @tr(\"Stop the session\") : @tr(\"Start the session\");"
    ));
    assert!(
        bar.contains("label: root.timer-active ? root.timer-label : @tr(\"start\"); mono: true;")
    );
    assert!(bar.contains("label: root.bar-form == 2 ? \"\" : @tr(\"ask\")"));
    assert!(bar.contains("label: root.bar-form == 2 ? \"\" : @tr(\"grab\")"));
    for old in [
        "Развернуть бар",
        "двигать бар",
        "на баре",
        "Иконки бара",
        "превращает бар",
    ] {
        assert!(!po.contains(old), "bar terminology: {old}");
    }
    assert!(!po.contains("msgid \"Restore last tile\""));
    assert!(!bar.contains("@tr(\"Restore last tile\")"));
    assert!(source("ui/bar_menu.slint").contains("@tr(\"Restore closed tile\")"));
}
