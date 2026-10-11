//! Source-level regression guard for the explicit lock-mode menu.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

#[test]
fn auto_tiles_menu_is_a_top_level_window_with_three_unconditional_choices() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("ui/overlay_bar.slint")).expect("read overlay bar");
    let menu = fs::read_to_string(root.join("ui/lock_mode_menu.slint")).expect("read lock menu");
    let lock_chip = source
        .split("component AutotileChip inherits Rectangle {")
        .nth(1)
        .and_then(|rest| rest.split_once("// Capsule children").map(|(chip, _)| chip))
        .expect("find AutotileChip component");

    for label in [
        "Silent: no automatic tiles",
        "On questions",
        "On every line",
    ] {
        assert!(menu.contains(&format!("@tr(\"{label}\")")));
    }
    assert!(menu.contains("mode-color: Theme.warning;"));
    assert!(menu.contains("mode-color: Theme.cat-orange;"));
    assert!(menu.contains("mode-color: Theme.text-secondary;"));
    for icon in ["lock", "unlock", "flame"] {
        assert!(menu.contains(&format!(
            "icon: @image-url(\"../assets/icons/{icon}.svg\");"
        )));
    }
    assert_eq!(menu.matches("LockModeMenuRow {").count(), 3);
    assert!(menu.contains("preferred-width: 320px;"));
    assert!(menu.contains("preferred-height: 106px;"));
    assert!(source.contains("in property <color> mode-color: Theme.text-secondary;"));
    assert!(source.contains("background: root.current ? root.mode-color.with-alpha(0.14)"));
    assert!(menu.contains("inherits Window"));
    assert!(!menu.contains("root.managed"));
    assert!(!menu.contains("Unload local AI"));
    assert!(source.contains("callback menu-opened(length, length);"));
    assert!(!lock_chip.contains("PopupWindow"));
    assert!(source.contains("callback autotile-mode-selected(int);"));
    assert!(source.contains("callback local-ai-mode-selected(int);"));
    for mode in [0, 1, 2] {
        assert!(menu.contains(&format!("current: root.mode == {mode};")));
        assert!(menu.contains(&format!("root.mode-selected({mode})")));
    }
}

#[test]
fn durable_mode_wiring_preserves_the_managed_deep_lock_until_ready() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs")).expect("host");
    let refresh =
        fs::read_to_string(root.join("src/bin/overlay_host/status_copy.rs")).expect("refresh");
    let settings = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("settings");
    let mode = host
        .split_once("overlay.on_autotile_mode_selected")
        .expect("mode handler")
        .1
        .split_once("overlay.on_local_ai_mode_selected")
        .expect("lifecycle boundary")
        .0;
    for contract in [
        "flags_from_mode(mode)",
        "busy.load(std::sync::atomic::Ordering::Acquire)",
        "c.deep_lock && overlay_backend::deep_lock::cfg_is_managed_local(&c)",
        "if !suppress && deep_managed",
        "bar.invoke_local_ai_mode_selected(if every_line { 3 } else { 4 })",
        "c.suppress_tiles = suppress;",
        "c.auto_tile_every_line = every_line;",
        "overlay_backend::config::save(&c)",
        "(c.suppress_tiles, c.auto_tile_every_line) = previous;",
        "refresh_lock_chip(&bar, &cfg)",
    ] {
        assert!(
            mode.contains(contract),
            "missing durable mode seam: {contract}"
        );
    }
    assert!(!mode.contains("c.deep_lock = false"));
    let lifecycle = host
        .split_once("overlay.on_local_ai_mode_selected")
        .expect("lifecycle")
        .1;
    assert!(lifecycle.contains("overlay_backend::deep_lock::lock_mode_action("));
    assert!(lifecycle.contains("3 => true,\n                        4 => false,"));
    assert!(lifecycle.contains("start_mlx_for_unlock(&cfg_unlock)"));
    assert!(lifecycle.contains("restart_llama_server_for_unlock("));
    for owner in ["config", "c"] {
        assert!(lifecycle.contains(&format!(
            "{owner}.auto_tile_every_line = target_every_line;"
        )));
        assert!(lifecycle.contains(&format!(
            "{owner}.auto_tile_every_line = previous_every_line;"
        )));
        assert!(lifecycle.contains(&format!("{owner}.deep_lock = true;")));
        assert!(lifecycle.contains(&format!("{owner}.suppress_tiles = true;")));
    }
    assert!(refresh.contains("mode_from_flags(snap.suppress_tiles, snap.auto_tile_every_line)"));
    assert!(refresh.contains("o.set_autotile_mode(mode)"));
    assert!(refresh.contains("o.set_suppress_tiles(suppress)"));
    assert!(refresh.contains("o.set_aggressive_active(every_line)"));
    assert!(refresh.contains("o.set_deep_lock(managed && snap.deep_lock)"));
    assert!(refresh.contains("overlay_backend::deep_lock::state_hint("));
    assert!(refresh.contains("o.invoke_autotile_state_refreshed()"));
    assert!(
        host.contains("settings.try_borrow()"),
        "refresh must not panic during Settings construction"
    );
    assert!(host.contains("win.set_suppress_tiles(bar.get_suppress_tiles())"));
    assert!(settings.contains("crate::refresh_lock_chip(&o, &cfg_c)"));
}
