//! Text contracts for the native More menu, its shared grid, routes and pins.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn source(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).expect(path)
}

#[test]
fn menu_is_exported_and_every_action_uses_one_fixed_column_grid() {
    let menu = source("ui/bar_menu.slint");
    let index = source("ui/index.slint");
    assert!(menu.contains("export component BarMenuWindow inherits Window"));
    assert!(index.contains("import { BarMenuWindow } from \"bar_menu.slint\";"));
    assert!(index
        .split_once("export {")
        .unwrap()
        .1
        .contains("BarMenuWindow"));
    assert!(menu.contains("no-frame: true;"));
    assert!(menu.contains("always-on-top: true;"));
    assert!(menu.contains("background: transparent;"));
    assert!(menu.contains("preferred-width: 408px;"));
    for declaration in [
        "property <length> col-key: 96px;",
        "property <length> col-pin: 78px;",
    ] {
        assert_eq!(
            menu.matches(declaration).count(),
            1,
            "declare each fixed column once"
        );
    }
    let grid = menu.split_once("for r in root.rows").unwrap().1;
    assert_eq!(grid.matches("width: root.col-key;").count(), 1);
    assert_eq!(grid.matches("width: root.col-pin;").count(), 1);
    assert!(!grid.contains("96px"));
    assert!(!grid.contains("78px"));
    assert!(grid.contains("x: parent.width - self.width;"));
    assert!(grid.contains("overflow: elide; min-width: 0px;"));
    assert!(grid.contains("height: r.head ? 22px : 28px;"));
    assert!(grid.contains(
        "padding-left: root.col-pad; padding-right: root.col-pad; spacing: root.col-gap;"
    ));
    assert!(grid.contains("width: root.col-icon;"));
    assert!(menu.contains("property <length> col-icon: 14px;"));
    assert!(menu.contains("property <length> col-gap: 8px;"));
    let name = grid
        .split_once("row-name := Text {")
        .unwrap()
        .1
        .split_once("\n                                }")
        .unwrap()
        .0;
    for contract in [
        "x: 0px; y: 0px;",
        "width: parent.width; height: parent.height;",
        "horizontal-alignment: left;",
        "vertical-alignment: center;",
        "overflow: elide; min-width: 0px;",
        "root.confirmation == r.id",
        "@tr(\"Quit?\")",
        "@tr(\"Restart?\")",
    ] {
        assert!(
            name.contains(contract),
            "missing name-column contract: {contract}"
        );
    }
    assert!(!name.contains("horizontal-alignment: center"));
    let key = grid
        .split_once("if root.confirmation != r.id && r.key != \"\" : Rectangle {")
        .unwrap()
        .1
        .split_once("TouchArea")
        .unwrap()
        .0;
    for contract in [
        "height: 20px;",
        "x: parent.width - self.width;",
        "width: min(kt.preferred-width + 14px, parent.width);",
        "border-radius: 5px;",
        "border-width: 1px; border-color: Theme.border;",
        "background: Theme.bg-surface;",
        "font-family: Theme.font-mono;",
        "font-size: Metrics.font-caption;",
    ] {
        assert!(
            key.contains(contract),
            "missing prototype key-chip contract: {contract}"
        );
    }
    assert_eq!(key.matches("border-radius:").count(), 1);
    assert!(
        !key.contains("Metrics.radius-control"),
        "key chip must not use the full pill radius"
    );
    assert!(!menu.contains("+ key"));
    for (id, key) in [
        ("capture", "F8"),
        ("archive", "F7"),
        ("kb", "F4"),
        ("help", "F1"),
    ] {
        let row = menu
            .lines()
            .find(|line| line.contains(&format!("id: \"{id}\"")))
            .unwrap();
        assert!(row.contains(&format!("key: \"{key}\"")));
    }
    let rows = menu
        .split_once("property <[MenuRow]> rows:")
        .unwrap()
        .1
        .split_once("];")
        .unwrap()
        .0;
    let mut last = 0;
    for id in [
        "capture",
        "aggressive",
        "restore",
        "archive",
        "summary",
        "kb",
        "settings",
        "help",
        "tray",
        "restart",
        "quit",
    ] {
        let at = rows.find(&format!("id: \"{id}\"")).unwrap();
        assert!(at >= last, "action order: {id}");
        last = at;
        let row = rows
            .lines()
            .find(|line| line.contains(&format!("id: \"{id}\"")))
            .unwrap();
        assert!(row.contains(if matches!(id, "restart" | "quit") {
            "pinnable: false"
        } else {
            "pinnable: true"
        }));
    }
}

#[test]
fn confirmation_and_pin_controls_stay_inside_their_rows() {
    let menu = source("ui/bar_menu.slint");
    for contract in [
        "callback action(string);",
        "callback pin-toggled(string);",
        "callback dismissed();",
        "Theme.accent.with-alpha(0.10)",
        "Theme.on-accent",
        "@tr(\"on the bar\")",
        "@tr(\"pin\")",
        "@tr(\"Restart?\")",
        "@tr(\"Quit?\")",
        "@tr(\"Yes\")",
        "@tr(\"No\")",
        "interval: 100ms;",
        "root.inactive-ticks >= 40",
        "Key.Escape",
        "if (root.confirmation != \"\") { root.confirmation = \"\"; }",
        "if r.id != \"tray\" || root.tray-available",
        "r.id != \"restore\" || root.can-restore-tile",
    ] {
        assert!(menu.contains(contract), "missing menu contract: {contract}");
    }
    let pin = menu.split_once("if r.pinnable : Rectangle").unwrap().1;
    assert!(pin.contains("root.pin-toggled(r.id)"));
    assert!(!pin.contains("root.action(r.id)"));
    assert!(!pin.contains("root.dismissed()"));
}

#[test]
fn menu_and_pins_dispatch_existing_callbacks_and_persist_only_the_pin_list() {
    let host = source("src/bin/overlay_host/bar_menu.rs");
    let root = source("src/bin/overlay_host_windows.rs");
    assert!(root
        .contains("bar_menu::wire_bar_menu(&overlay, &bar_menu, &cfg, &palette, &tiles, &state)"));
    for (id, invocation) in [
        ("capture", "invoke_capture_clicked"),
        ("aggressive", "invoke_aggressive_toggle_clicked"),
        ("restore", "invoke_restore_tile_clicked"),
        ("archive", "invoke_archive_clicked"),
        ("summary", "invoke_summary_clicked"),
        ("settings", "invoke_open_settings_clicked"),
        ("help", "invoke_help_clicked"),
        ("tray", "invoke_hide_to_tray_clicked"),
        ("restart", "invoke_restart_confirm"),
        ("quit", "invoke_quit_confirm"),
    ] {
        let route = host
            .lines()
            .find(|line| line.contains(&format!("\"{id}\"")) && line.contains("=>"))
            .unwrap();
        assert!(route.contains(invocation), "wrong dispatch: {id}");
    }
    assert!(host.contains("\"kb\" => kb()"));
    assert!(host.contains("open_palette(&palette, &tiles, &state, &weak_for_kb)"));
    assert_eq!(
        host.matches("dispatch_bar_action(&bar, id.as_str(), kb.as_ref())")
            .count(),
        2
    );
    assert!(host.contains("overlay.on_pin_activated"));
    assert!(host.contains("overlay.on_menu_opened"));
    let selection = host
        .split_once("menu.on_action")
        .unwrap()
        .1
        .split_once("menu.on_dismissed")
        .unwrap()
        .0;
    assert!(
        selection.find("close_menu(").unwrap() < selection.find("dispatch_bar_action(").unwrap()
    );
    let pinnable = host
        .split_once("const PINNABLE_IDS")
        .unwrap()
        .1
        .split_once("];")
        .unwrap()
        .0;
    assert!(!pinnable.contains("\"restart\""));
    assert!(!pinnable.contains("\"quit\""));
    assert!(host.contains("const MAX_PINS: usize = 8;"));
    assert!(host.contains("pins_model(&cfg.read().bar_pins)"));
    assert!(host.contains("candidate = c.bar_pins.clone()"));
    assert!(host.contains("candidate.retain(|pin| pin != id.as_str())"));
    assert!(host.contains("candidate.push(id.to_string())"));
    assert!(host.contains("overlay_backend::config::save(&c)"));
    assert!(host.contains("c.bar_pins = previous;"));
    assert!(host.contains("bar.set_pins(model)"));
    assert!(host.contains("refresh_menu(&menu, &bar)"));
    let refresh = host
        .split_once("fn refresh_menu(")
        .unwrap()
        .1
        .split_once("fn close_menu(")
        .unwrap()
        .0;
    for property in [
        "pins",
        "aggressive_active",
        "can_restore_tile",
        "tray_available",
        "settings_open",
        "archive_open",
        "help_open",
        "summary_busy",
    ] {
        assert!(refresh.contains(&format!("menu.set_{property}(bar.get_{property}())")));
    }
    let open = host.split_once("overlay.on_menu_opened").unwrap().1;
    assert!(
        open.find("refresh_menu(&menu, &bar)").unwrap() < open.find("menu.show()").unwrap(),
        "every open must seed current bar state before showing"
    );
    let config = source("../overlay-backend/src/config.rs");
    assert!(config.contains("#[serde(default)]\n    pub bar_pins: Vec<String>"));
    assert!(config.contains("bar_pins: Vec::new()"));
}

#[test]
fn native_lifecycle_matches_lock_menu_and_the_bottom_row_stays_bare() {
    let host = source("src/bin/overlay_host/bar_menu.rs");
    for contract in [
        "work_area_for_point(anchor_x, anchor_y)",
        "set_window_owner(hwnd, bar_hwnd)",
        "set_skip_taskbar(hwnd, true)",
        "set_always_on_top(hwnd, true)",
        "set_stealth(hwnd, true)",
        "set_platform_window_position(menu.window(), -32000, -32000)",
        "realize_with_retries",
        "FocusTransition::DismissOutside",
        "consume_suppressed_open()",
        "is_current_open(generation)",
        "bar.set_menu_open(true)",
        "bar.set_menu_open(false)",
        "bar_top - height - gap",
    ] {
        assert!(
            host.contains(contract),
            "missing native lifecycle: {contract}"
        );
    }
    let registry = source("src/bin/overlay_host/window_lifecycle.rs");
    assert_eq!(
        registry.matches("self.bar_menu.borrow().as_ref()").count(),
        2
    );
    let bar = source("ui/overlay_bar.slint");
    assert!(bar.contains("in property <[string]> pins;"));
    assert!(bar.contains("callback pin-activated(string);"));
    assert!(bar.contains("if root.pins.length == 0 : PinChip"));
    assert!(bar.contains("for id in root.pins : PinChip"));
    assert!(bar.contains("!root.is-pinned(\"restore\")"));
    assert!(bar.contains("!root.is-pinned(\"aggressive\")"));
    let chip = bar
        .split_once("component PinChip")
        .unwrap()
        .1
        .split_once("component MoreChip")
        .unwrap()
        .0;
    assert!(!chip.contains("border-width"));
    assert!(!chip.contains("background:"));
    assert!(chip.contains("height: 22px;"));
}

#[test]
fn bar_interactions_and_focus_loss_share_the_close_path() {
    let host = source("src/bin/overlay_host/bar_menu.rs");
    let bar = source("ui/overlay_bar.slint");
    assert!(bar.contains("in-out property <bool> menu-open: false;"));
    assert!(bar.contains("callback menu-dismissed();"));
    assert!(bar.contains("function dismiss-menu() {\n        root.menu-open = false;\n        root.menu-dismissed();\n    }"));
    assert!(bar.contains("changed compact-bar => { root.dismiss-menu(); }"));
    let dismiss = host
        .split_once("overlay.on_menu_dismissed")
        .unwrap()
        .1
        .split_once("menu.on_dismissed")
        .unwrap()
        .0;
    assert!(dismiss.contains("close_menu(&menu, &weak, &focus)"));
    let focus = host
        .split_once("if transition == FocusTransition::DismissOutside")
        .unwrap()
        .1
        .split_once("EventResult::Propagate")
        .unwrap()
        .0;
    assert!(focus.contains("suppress_next_open()"));
    assert!(focus.contains("close_menu(&menu, &weak, &focus)"));
    assert_eq!(
        host.matches("menu.hide()").count(), 1,
        "hide belongs only to close_menu"
    );
    let open = host.split_once("overlay.on_menu_opened").unwrap().1;
    assert!(open.contains("if menu.window().is_visible() {\n                close_menu(&menu, &weak, &focus);\n                return;\n            }"));
    let bar = bar
        .split_once("export component OverlayBarWindow")
        .unwrap()
        .1;
    let callbacks: Vec<_> = bar
        .lines()
        .filter_map(|line| line.trim().strip_prefix("callback "))
        .map(|declaration| declaration.split_once('(').unwrap().0)
        .filter(|name| !matches!(*name, "menu-opened" | "menu-dismissed"))
        .collect();
    for line in bar.lines() {
        for name in &callbacks {
            if let Some(at) = line.find(&format!("root.{name}(")) {
                assert!(
                    line[..at].contains("root.dismiss-menu();"),
                    "bar interaction bypasses dismissal: {line}"
                );
            }
        }
    }
    let menu = source("ui/bar_menu.slint");
    let escape = menu
        .split_once("if (e.text == Key.Escape)")
        .unwrap()
        .1
        .split_once("return accept;")
        .unwrap()
        .0;
    assert!(escape.contains("root.dismissed();"));
    assert!(!escape.contains("else"), "Esc also closes an armed confirmation");
}
