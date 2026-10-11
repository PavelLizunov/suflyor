//! Headless geometry regression for both fixed capsule layouts in English and Russian.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize};
use std::fs;
use std::path::Path;

mod ui {
    slint::include_modules!();
}

fn element(bar: &ui::OverlayBarWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(bar, label)
        .next()
        .unwrap_or_else(|| panic!("find element with accessible label: {label}"))
}

struct Labels {
    ask: &'static str,
    grab: &'static str,
    close_all: &'static str,
    tile: &'static str,
    deep_lock: &'static str,
    write: &'static str,
    compact: &'static str,
    expand: &'static str,
    more: &'static str,
    tray: &'static str,
    app_ram: &'static str,
    you: &'static str,
    them: &'static str,
}

const ENGLISH: Labels = Labels {
    ask: "ask",
    grab: "grab",
    close_all: "close all",
    tile: "New tile, {} open",
    deep_lock: "Local AI unloaded",
    write: "Write a question",
    compact: "Compact reader mode",
    expand: "Expand the bar",
    more: "More",
    tray: "Hide to tray",
    app_ram: "App RAM",
    you: "you:",
    them: "them:",
};
const RUSSIAN: Labels = Labels {
    ask: "спросить",
    grab: "захватить",
    close_all: "закрыть все",
    tile: "Новый тайл, открыто: {}",
    deep_lock: "Локальный AI выгружен",
    write: "Написать вопрос",
    compact: "Компактный режим чтения",
    expand: "Развернуть бар",
    more: "Ещё",
    tray: "Свернуть в трей",
    app_ram: "RAM приложения",
    you: "вы:",
    them: "собеседник:",
};

fn assert_top_bounds(item: &ElementHandle, width: f32, name: &str) {
    let pos = item.absolute_position();
    let size = item.size();
    assert!(
        size.width > 0.0 && size.height > 0.0,
        "{name} has an empty target"
    );
    assert!(
        pos.y >= 0.0 && pos.y + size.height <= 34.0,
        "{name} entered the footer"
    );
    assert!(
        pos.x >= 0.0 && pos.x + size.width <= width,
        "{name} escaped the bar"
    );
}

fn assert_layout(bar: &ui::OverlayBarWindow, labels: &Labels, compact: bool) {
    bar.set_compact_bar(compact);
    bar.set_lock_a11y("Lock mode".into());
    bar.set_suppress_tiles(false);
    bar.set_deep_lock(false);
    bar.set_timer_active(true);
    bar.set_timer_label("1:07:31".into());
    bar.set_stealth_fault(true);
    bar.set_tray_available(true);
    let spec = bar.global::<ui::BarSpec>();
    let width = if compact {
        spec.get_compact_width()
    } else {
        spec.get_full_width()
    };
    let height = spec.get_height();
    assert_eq!(height, 64.0);
    assert_eq!(spec.get_full_tiles(), 226.0);
    assert_eq!(spec.get_compact_tiles(), 30.0);
    assert_eq!(spec.get_full_width(), 854.0);
    assert_eq!(spec.get_compact_width(), 586.0);
    assert!(spec.get_full_width() > spec.get_compact_width());
    bar.window().set_size(LogicalSize::new(width, height));
    let stable_names = if compact {
        vec![
            labels.ask,
            labels.grab,
            "1:07:31",
            "Lock mode",
            labels.expand,
            labels.more,
        ]
    } else {
        vec![
            labels.ask,
            labels.grab,
            "1:07:31",
            "Lock mode",
            labels.write,
            labels.compact,
            labels.more,
        ]
    };
    let stable_x: Vec<f32> = stable_names
        .iter()
        .map(|name| element(bar, name).absolute_position().x)
        .collect();

    for count in [0, 1, 2, 99, 128, 999] {
        bar.set_open_tiles(count);
        for mode in [0, 1, 2] {
            bar.set_suppress_tiles(mode != 0);
            bar.set_deep_lock(mode == 2);
            for recording in [false, true] {
                bar.set_mic_recording(recording);
                bar.set_sys_recording(recording);
                bar.set_session_paused(recording);
                bar.set_menu_open(recording);
                // Armed confirmations must no longer expand or hide bar controls.
                bar.set_quit_armed(recording);
                bar.set_restart_armed(recording);
                let lock_label = if mode == 2 {
                    labels.deep_lock
                } else {
                    "Lock mode"
                };
                let mut names = vec![labels.ask, labels.grab, "1:07:31", lock_label];
                let tile_label = labels.tile.replace("{}", &count.to_string());
                if !compact {
                    names.push(&tile_label);
                    if count > 0 {
                        names.push(labels.close_all);
                    }
                    names.push(labels.write);
                    names.push(labels.compact);
                } else {
                    names.push(labels.tray);
                    names.push(labels.expand);
                }
                names.push(labels.more);
                bar.window()
                    .set_size(LogicalSize::new(width + 300.0, height));
                let natural: Vec<f32> = names
                    .iter()
                    .map(|name| element(bar, name).size().width)
                    .collect();
                bar.window().set_size(LogicalSize::new(width, height));
                let items: Vec<ElementHandle> =
                    names.iter().map(|name| element(bar, name)).collect();
                assert!(bar.get_all_regions_fit(), "capsule overflow: compact={compact}, mode={mode}, tiles={count}, recording={recording}");
                for ((name, item), natural_width) in names.iter().zip(&items).zip(natural) {
                    assert_top_bounds(item, width, name);
                    assert_eq!(item.size().width, natural_width, "{name} was compressed");
                }
                for pair in items.windows(2) {
                    assert!(
                        pair[0].absolute_position().x + pair[0].size().width
                            <= pair[1].absolute_position().x,
                        "capsule controls overlap: compact={compact}, mode={mode}, tiles={count}"
                    );
                }
                for (name, x) in stable_names.iter().zip(&stable_x) {
                    let name = if *name == "Lock mode" {
                        lock_label
                    } else {
                        *name
                    };
                    assert_eq!(
                        element(bar, name).absolute_position().x,
                        *x,
                        "{name} moved on a state change"
                    );
                }
                assert_eq!(element(bar, labels.ask).size().width, 102.0);
                assert_eq!(element(bar, labels.grab).size().width, 108.0);
                assert_eq!(
                    element(bar, lock_label).size().width,
                    if compact { 26.0 } else { 76.0 }
                );
            }
        }
    }
    bar.set_quit_armed(false);
    bar.set_restart_armed(false);
    bar.set_mic_recording(false);
    bar.set_sys_recording(false);
    bar.set_stealth_fault(false);
    bar.set_status_tone(ui::StatusTone::Neutral);
    bar.set_session_name("SESSION".into());
    bar.set_last_transcript_line("TRANSCRIPT".into());
    for (source, speaker) in [("mic", labels.you), ("sys", labels.them)] {
        bar.set_last_transcript_source(source.into());
        let word = element(bar, speaker);
        let line = element(bar, "TRANSCRIPT");
        assert!(
            word.absolute_position().y >= 34.0,
            "speaker entered the action row"
        );
        assert!(
            word.absolute_position().x + word.size().width <= line.absolute_position().x,
            "speaker overlaps transcript"
        );
    }
    bar.set_last_transcript_source("".into());
    for (app_memory, mlx_memory) in [
        ("", ""),
        ("123 MB", ""),
        ("", "456 MB"),
        ("123 MB", "456 MB"),
    ] {
        bar.set_app_memory(app_memory.into());
        bar.set_mlx_memory(mlx_memory.into());
        let app_label = format!("{} {app_memory}", labels.app_ram);
        let mut names = vec!["SESSION", "TRANSCRIPT"];
        if !app_memory.is_empty() {
            names.push(&app_label);
        }
        if !mlx_memory.is_empty() {
            names.push("MLX 456 MB");
        }
        let items: Vec<ElementHandle> = names.iter().map(|name| element(bar, name)).collect();
        for (name, item) in names.iter().zip(&items) {
            let pos = item.absolute_position();
            let size = item.size();
            assert!(
                pos.y >= 34.0 && pos.y + size.height <= height,
                "{name} escaped the footer"
            );
            assert!(
                pos.x >= 0.0 && pos.x + size.width <= width,
                "{name} escaped footer width"
            );
        }
        for pair in items.windows(2) {
            assert!(
                pair[0].absolute_position().x + pair[0].size().width
                    <= pair[1].absolute_position().x,
                "footer labels overlap"
            );
        }
    }
    bar.set_app_memory("".into());
    bar.set_mlx_memory("".into());
    bar.set_session_name("".into());
    bar.set_last_transcript_line("".into());
    let window_label = if compact {
        labels.expand
    } else {
        labels.compact
    };
    let window_x = element(bar, window_label).absolute_position().x;
    bar.set_tray_available(false);
    bar.set_timer_active(false);
    assert_eq!(
        element(bar, window_label).absolute_position().x,
        window_x,
        "empty tray/pause slots moved the window controls"
    );
    assert!(
        bar.get_all_regions_fit(),
        "empty conditional slots must still fit"
    );
}

fn assert_pins(bar: &ui::OverlayBarWindow, russian: bool) {
    let ids = ["capture", "aggressive", "restore", "archive", "summary", "kb", "settings", "help"];
    let names = if russian {
        ["Вопрос по снимку экрана", "Автотайлы на всё", "Вернуть закрытый тайл", "Архив сессий", "Итог встречи", "База знаний", "Настройки", "Справка"]
    } else {
        ["Ask about a screenshot", "Auto-tiles on every line", "Restore closed tile", "Session archive", "Meeting summary", "Knowledge base", "Settings", "Help"]
    };
    let hint = if russian { "закрепить" } else { "pin" };
    let width = bar.global::<ui::BarSpec>().get_full_width();
    let height = bar.global::<ui::BarSpec>().get_height();
    bar.set_compact_bar(false);
    bar.set_last_transcript_line("PIN GEOMETRY TRANSCRIPT".into());
    bar.set_can_restore_tile(true);
    bar.set_aggressive_active(true);
    for count in [0, 3, 8] {
        bar.set_pins(slint::ModelRc::new(slint::VecModel::from(
            ids[..count].iter().map(|id| slint::SharedString::from(*id)).collect::<Vec<_>>(),
        )));
        bar.window().set_size(LogicalSize::new(width, height));
        assert_eq!(bar.window().size().width as f32, width, "pins must not resize the bar");
        let hints: Vec<_> = ElementHandle::find_by_accessible_label(bar, hint).collect();
        assert_eq!(hints.len(), usize::from(count == 0), "hint presence with {count} pins");
        if let Some(hint) = hints.first() {
            let more = element(bar, if russian { "Ещё" } else { "More" });
            let anchor = std::rc::Rc::new(std::cell::Cell::new((0.0_f32, 0.0_f32)));
            let anchor_for_callback = anchor.clone();
            bar.on_menu_opened(move |x, y| anchor_for_callback.set((x, y)));
            hint.invoke_accessible_default_action();
            let pos = more.absolute_position();
            let size = more.size();
            assert_eq!(anchor.get(), (pos.x + size.width, pos.y + size.height), "hint must use the More chip's anchor");
        }
        let transcript = element(bar, "PIN GEOMETRY TRANSCRIPT");
        let mut right = transcript.absolute_position().x + transcript.size().width;
        for name in &names[..count] {
            let name = *name;
            let matching: Vec<_> = ElementHandle::find_by_accessible_label(bar, name).collect();
            assert_eq!(matching.len(), 1, "duplicate pinned action: {name}");
            let item = &matching[0];
            let pos = item.absolute_position();
            let size = item.size();
            assert_eq!(size.width, 26.0, "fixed bare pin width: {name}");
            assert_eq!(size.height, 22.0, "fixed bare pin height: {name}");
            assert!(pos.y >= 34.0 && pos.y + size.height <= height, "pin escaped bottom row: {name}");
            assert!(pos.x >= right && pos.x + size.width <= width, "pin overlaps transcript/previous pin or escapes bar: {name}");
            right = pos.x + size.width;
        }
        for (id, legacy_label) in [("restore", "Restore last tile"), ("aggressive", "Aggressive auto-tile")] {
            let legacy_label = if russian {
                if id == "restore" { "Вернуть последний тайл" } else { "Агрессивные авто-тайлы" }
            } else { legacy_label };
            assert_eq!(ElementHandle::find_by_accessible_label(bar, legacy_label).count(), usize::from(!ids[..count].contains(&id)), "conditional action must not duplicate a pin: {id}");
        }
        bar.set_compact_bar(true);
        bar.window().set_size(LogicalSize::new(bar.global::<ui::BarSpec>().get_compact_width(), height));
        assert_eq!(ElementHandle::find_by_accessible_label(bar, hint).count(), 0);
        for name in &names[..count] {
            let name = *name;
            assert_eq!(ElementHandle::find_by_accessible_label(bar, name).count(), 0, "compact bar must not show pins: {name}");
        }
        bar.set_compact_bar(false);
    }
    bar.set_pins(slint::ModelRc::new(slint::VecModel::<slint::SharedString>::default()));
    bar.set_can_restore_tile(false);
    bar.set_aggressive_active(false);
    bar.set_last_transcript_line("".into());
}

#[test]
fn capsules_stay_inside_spec_sizes_in_english_and_russian() {
    i_slint_backend_testing::init_no_event_loop();
    let english = ui::OverlayBarWindow::new().expect("create English bar");
    assert_layout(&english, &ENGLISH, false);
    assert_layout(&english, &ENGLISH, true);
    assert_pins(&english, false);
    slint::select_bundled_translation("ru").expect("select Russian translation");
    let russian = ui::OverlayBarWindow::new().expect("create Russian bar");
    assert_layout(&russian, &RUSSIAN, false);
    assert_layout(&russian, &RUSSIAN, true);
    assert_pins(&russian, true);
}

#[test]
fn record_indicator_reserves_an_empty_idle_cell_and_keeps_active_shapes() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint"))
            .expect("read overlay bar source");
    let record = source
        .split_once("component RecordBtn inherits Rectangle {")
        .expect("RecordBtn component")
        .1
        .split_once("// ===== Main overlay bar =====")
        .expect("RecordBtn boundary")
        .0;
    let indicator = record
        .split_once("// Reserve the cell while idle;")
        .expect("unconditional indicator slot")
        .1
        .split_once("if root.has-icon : IconSlot {")
        .expect("source icon follows indicator")
        .0;
    assert!(indicator.contains("Rectangle {"));
    assert!(
        !indicator.contains("if "),
        "idle cell must remain in the layout"
    );
    for binding in [
        "width: 8px;",
        "height: 8px;",
        "horizontal-stretch: 0;",
        "border-radius: root.locked ? 0px : 4px;",
        "y: (parent.height - self.height) / 2;",
        "background: root.recording ? Theme.danger : transparent;",
        "border-width: 0px;",
    ] {
        assert!(
            indicator.contains(binding),
            "missing indicator binding: {binding}"
        );
    }
    assert!(
        !indicator.contains("border-color:"),
        "idle cell must not draw a ring"
    );
    assert!(!indicator.contains("visible:"), "idle cell must not collapse");
    assert!(record.contains("if (!self.recording) {\n            self.locked = false;"));
}

#[test]
fn tile_counter_keeps_text_and_close_all_uses_a_reserved_icon_slot() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint"))
            .expect("read overlay bar source");
    let tile_chip = source
        .split_once("width: 92px;")
        .expect("fixed tile counter slot")
        .1
        .split_once("clicked => { root.spawn-tile-clicked(); }")
        .expect("tile action")
        .0;
    assert!(tile_chip.contains("icon: @image-url(\"../assets/icons/plus.svg\"); has-icon: true;"));
    assert!(tile_chip.contains("label: @tr(\"tile · {}\", root.open-tiles);"));
    assert!(tile_chip.contains("a11y: @tr(\"New tile, {} open\", root.open-tiles);"));
    assert!(!source.contains("@tr(\"+ tile ({})\""));
    assert!(source.contains("if root.open-tiles > 0 : BarChip"));
    assert!(source.contains("a11y: @tr(\"close all\");"));
    assert!(!source.contains("label: @tr(\"close all\");"));
    assert!(!source.contains("@image-url(\"../assets/icons/trash.svg\")"));
    assert!(!source.contains("@image-url(\"../assets/icons/tiles.svg\")"));
    let normal = source
        .split_once("// One set of named capsules")
        .expect("normal layout")
        .1
        .split_once("// First honest production surface")
        .expect("bootstrap boundary")
        .0;
    for removed in [
        "root.active-stack",
        "root.tok-per-sec",
        "icons/grip.svg",
        "icons/camera.svg",
        "icons/gear.svg",
        "icons/sos.svg",
        "icons/restart.svg",
        "icons/archive.svg",
        "icons/summary.svg",
        "root.quit-confirm()",
        "root.restart-confirm()",
        "waiting for transcript",
        "AI streaming",
    ] {
        assert!(!normal.contains(removed), "obsolete bar surface: {removed}");
    }
}
