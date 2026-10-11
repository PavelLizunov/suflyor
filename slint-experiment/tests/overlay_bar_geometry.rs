//! Headless geometry regression for all three fixed capsule layouts in English and Russian.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize};
use std::fs;
use std::path::Path;

mod ui {
    slint::include_modules!();
}

fn element(bar: &ui::OverlayBarWindow, label: &str) -> ElementHandle {
    // Settle lazily instantiated `if` children before any caller reads layout.
    assert!(ElementHandle::find_by_accessible_label(bar, label).count() > 0);
    ElementHandle::find_by_accessible_label(bar, label)
        .next()
        .unwrap_or_else(|| panic!("find element with accessible label: {label}"))
}

struct Labels {
    russian: bool,
    ask: &'static str,
    grab: &'static str,
    stop: &'static str,
    close_all: &'static str,
    tile: &'static str,
    deep_lock: &'static str,
    write: &'static str,
    compact: &'static str,
    expand: &'static str,
    more: &'static str,
    tray: &'static str,
    stealth: &'static str,
    app_ram: &'static str,
    you: &'static str,
    them: &'static str,
}

const ENGLISH: Labels = Labels {
    russian: false,
    ask: "Hold to ask with your voice",
    grab: "Hold to ask about what they said",
    stop: "Stop the session",
    close_all: "close all",
    tile: "New tile, {} open",
    deep_lock: "Deep AI hint",
    write: "Write a question",
    compact: "Compact reader mode",
    expand: "Expand the bar",
    more: "More",
    tray: "Hide to tray",
    stealth: "Stealth (hide from capture)",
    app_ram: "App RAM",
    you: "you:",
    them: "them:",
};
const RUSSIAN: Labels = Labels {
    russian: true,
    ask: "Удерживайте: вопрос своим голосом",
    grab: "Удерживайте: вопрос по словам собеседника",
    stop: "Остановить сессию",
    close_all: "закрыть все",
    tile: "Новый тайл, открыто: {}",
    deep_lock: "Deep AI hint",
    write: "Написать вопрос",
    compact: "Компактный режим чтения",
    expand: "Развернуть панель",
    more: "Ещё",
    tray: "Свернуть в трей",
    stealth: "Скрытность (не показывать при захвате экрана)",
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

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    fn of(item: &ElementHandle) -> Self {
        let pos = item.absolute_position();
        let size = item.size();
        Self {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        }
    }

    fn right(self) -> f32 {
        self.x + self.width
    }

    fn bottom(self) -> f32 {
        self.y + self.height
    }

    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }
}

fn mic_label(bar: &ui::OverlayBarWindow, labels: &Labels) -> &'static str {
    match (labels.russian, bar.get_mic_muted()) {
        (true, true) => "Микрофон выключен (нажмите, чтобы включить)",
        (true, false) => "Микрофон включён (нажмите, чтобы выключить)",
        (false, true) => "Mic muted (click to unmute)",
        (false, false) => "Mic live (click to mute)",
    }
}

fn assert_top_order_and_capsules(
    bar: &ui::OverlayBarWindow,
    labels: &Labels,
    form: i32,
    state: &str,
) {
    // No layout-derived reads until `if` children have been instantiated.
    assert_eq!(
        ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
        1,
        "settle top row: form={form}, russian={}, {state}",
        labels.russian,
    );
    let spec = bar.global::<ui::BarSpec>();
    let capsules = [
        Rect {
            x: bar.get_state_capsule_x(),
            y: bar.get_state_capsule_y(),
            width: bar.get_state_capsule_width(),
            height: bar.get_state_capsule_height(),
        },
        Rect {
            x: bar.get_voice_capsule_x(),
            y: bar.get_voice_capsule_y(),
            width: bar.get_voice_capsule_width(),
            height: bar.get_voice_capsule_height(),
        },
        Rect {
            x: bar.get_session_capsule_x(),
            y: bar.get_session_capsule_y(),
            width: bar.get_session_capsule_width(),
            height: bar.get_session_capsule_height(),
        },
        Rect {
            x: bar.get_tiles_capsule_x(),
            y: bar.get_tiles_capsule_y(),
            width: bar.get_tiles_capsule_width(),
            height: bar.get_tiles_capsule_height(),
        },
        Rect {
            x: bar.get_window_capsule_x(),
            y: bar.get_window_capsule_y(),
            width: bar.get_window_capsule_width(),
            height: bar.get_window_capsule_height(),
        },
    ];
    let first = usize::from(form == 2);
    let width = match form {
        0 => spec.get_full_width(),
        1 => spec.get_compact_width(),
        2 => spec.get_mini_width(),
        _ => panic!("unsupported bar form: {form}"),
    };
    let declared = match form {
        0 => [
            spec.get_full_state(),
            spec.get_full_voice(),
            spec.get_full_session(),
            spec.get_full_tiles(),
            spec.get_full_window(),
        ],
        1 => [
            spec.get_compact_state(),
            spec.get_compact_voice(),
            spec.get_compact_session(),
            spec.get_compact_tiles(),
            spec.get_compact_window(),
        ],
        2 => [
            0.0,
            spec.get_mini_voice(),
            spec.get_mini_session(),
            spec.get_mini_tiles(),
            spec.get_mini_window(),
        ],
        _ => panic!("unsupported bar form: {form}"),
    };
    for (index, capsule) in capsules.iter().enumerate().skip(first) {
        assert!(capsule.width > 0.0 && capsule.height > 0.0);
        assert_eq!(
            capsule.width, declared[index],
            "actual capsule width: {state}"
        );
        assert!(capsule.y >= 0.0 && capsule.bottom() <= 34.0);
        assert!(
            capsule.x >= spec.get_pad() + spec.get_badge() + spec.get_pad()
                && capsule.right() <= width - spec.get_pad(),
            "capsule={index} outside content: {capsule:?}, form={form}, {state}"
        );
        assert_eq!(capsule.y, capsules[first].y, "capsule row: {state}");
        assert_eq!(
            capsule.height, capsules[first].height,
            "capsule height: {state}"
        );
    }
    for (index, pair) in capsules[first..].windows(2).enumerate() {
        let separation = pair[1].x - pair[0].right();
        assert!(
            separation > 0.0,
            "capsules overlap: form={form}, {state}, {pair:?}"
        );
        if index + first == 3 {
            // The spring is a separate layout child with a gap on each side.
            assert!(separation >= 2.0 * spec.get_gap() + spec.get_spring());
        } else {
            assert_eq!(
                separation,
                spec.get_gap(),
                "capsule gap: form={form}, {state}"
            );
        }
    }

    let mut names = Vec::new();
    if form != 2 {
        names.push((labels.stealth.to_owned(), 0));
        if form == 0 {
            names.push((mic_label(bar, labels).to_owned(), 0));
            names.push((
                if labels.russian {
                    "Захват системного звука"
                } else {
                    "System audio capture"
                }
                .to_owned(),
                0,
            ));
        }
    }
    names.extend([(labels.ask.to_owned(), 1), (labels.grab.to_owned(), 1)]);
    let session = if bar.get_timer_active() {
        labels.stop
    } else if labels.russian {
        "Начать сессию"
    } else {
        "Start the session"
    };
    names.push((session.to_owned(), 2));
    if bar.get_timer_active() {
        let pause = match (labels.russian, bar.get_session_paused()) {
            (true, true) => "Продолжить сессию",
            (true, false) => "Приостановить сессию",
            (false, true) => "Resume the session",
            (false, false) => "Pause the session",
        };
        names.push((pause.to_owned(), 2));
    }
    if form == 2 {
        names.push((labels.stealth.to_owned(), 3));
    }
    let switch = match (labels.russian, bar.get_autotile_mode()) {
        (true, 0) => "Молчат: сами не появляются",
        (true, 2) => "На каждую реплику",
        (true, _) => "По вопросам",
        (false, 0) => "Silent: no automatic tiles",
        (false, 2) => "On every line",
        (false, _) => "On questions",
    };
    names.push((switch.to_owned(), 3));
    if form == 0 {
        names.push((
            labels.tile.replace("{}", &bar.get_open_tiles().to_string()),
            3,
        ));
        if bar.get_open_tiles() > 0 {
            names.push((labels.close_all.to_owned(), 3));
        }
        names.push((labels.write.to_owned(), 3));
    }
    // Compact's approved order is tray, expand, More (not expand, tray).
    if form == 1 && bar.get_tray_available() {
        names.push((labels.tray.to_owned(), 4));
    }
    names.push((
        if form == 0 {
            labels.compact
        } else {
            labels.expand
        }
        .to_owned(),
        4,
    ));
    names.push((labels.more.to_owned(), 4));
    let controls: Vec<_> = names
        .iter()
        .map(|(name, capsule)| {
            let matches: Vec<_> = ElementHandle::find_by_accessible_label(bar, name).collect();
            // A full-form tray pin is in the footer, never in this list.
            assert_eq!(
                matches.len(),
                1,
                "unique top control: {name}, form={form}, {state}"
            );
            let item = &matches[0];
            assert_top_bounds(item, width, name);
            let rect = Rect::of(item);
            let parent = capsules[*capsule];
            assert!(
                parent.contains(rect.x, rect.y) && parent.contains(rect.right(), rect.bottom()),
                "child outside capsule: {name}, rect={rect:?}, capsule={parent:?}, {state}"
            );
            (name.as_str(), rect)
        })
        .collect();
    let status = if bar.get_stealth_fault() {
        if labels.russian {
            "[!] скрытность недоступна"
        } else {
            "[!] stealth unavailable"
        }
        .to_owned()
    } else {
        let status = bar.get_status_text().to_string();
        if labels.russian && status == "idle" {
            "ожидание".to_owned()
        } else {
            status
        }
    };
    let mascot_label = format!(
        "{status}. {}",
        if labels.russian {
            "Перетащить = двигать панель; показывает состояние"
        } else {
            "Drag to move the bar; shows the live state"
        },
    );
    let mascot = Rect::of(&element(bar, &mascot_label));
    assert_eq!(mascot.width, spec.get_badge());
    assert!(
        controls[0].1.x > mascot.right(),
        "first control must start after mascot: form={form}, {state}"
    );
    for pair in controls.windows(2) {
        assert!(
            pair[0].1.right() < pair[1].1.x,
            "top order/separation: {} before {}, form={form}, {state}: {pair:?}",
            pair[0].0,
            pair[1].0,
        );
        assert_eq!(pair[0].1.y, pair[1].1.y, "top y: {state}");
        assert_eq!(
            pair[0].1.bottom(),
            pair[1].1.bottom(),
            "top bottom: {state}"
        );
    }
    for target in [labels.stealth, mic_label(bar, labels), labels.more] {
        if let Some((_, rect)) = controls.iter().find(|(name, _)| *name == target) {
            let centre = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            for (other, bounds) in &controls {
                if *other != target {
                    assert!(
                        !bounds.contains(centre.0, centre.1),
                        "centre covered: {target} by {other}, form={form}, {state}"
                    );
                }
            }
        } else {
            assert!(form != 0 && target == mic_label(bar, labels));
        }
    }
}

fn assert_form_contract(bar: &ui::OverlayBarWindow, labels: &Labels, form: i32, state: &str) {
    assert_top_order_and_capsules(bar, labels, form, state);
    let spec = bar.global::<ui::BarSpec>();
    let (width, declared) = match form {
        0 => (
            spec.get_full_width(),
            [
                spec.get_full_state(),
                spec.get_full_voice(),
                spec.get_full_session(),
                spec.get_full_tiles(),
                spec.get_full_window(),
            ],
        ),
        1 => (
            spec.get_compact_width(),
            [
                spec.get_compact_state(),
                spec.get_compact_voice(),
                spec.get_compact_session(),
                spec.get_compact_tiles(),
                spec.get_compact_window(),
            ],
        ),
        2 => (
            spec.get_mini_width(),
            [
                0.0,
                spec.get_mini_voice(),
                spec.get_mini_session(),
                spec.get_mini_tiles(),
                spec.get_mini_window(),
            ],
        ),
        _ => panic!("unsupported bar form: {form}"),
    };
    let stealth: Vec<_> = ElementHandle::find_by_accessible_label(bar, labels.stealth).collect();
    assert_eq!(
        stealth.len(),
        1,
        "exactly one stealth chip: form={form}, language={}, {state}",
        labels.russian,
    );
    assert_top_bounds(
        &stealth[0],
        width,
        &format!("label={}, form={form}, {state}", labels.stealth),
    );
    assert_eq!(
        stealth[0].size().width,
        if form == 0 { 70.0 } else { 26.0 },
        "stealth chip width: form={form}, {state}"
    );
    if form == 2 {
        assert_eq!(
            ElementHandle::find_by_accessible_label(bar, labels.tray).count(),
            0,
            "mini must not instantiate tray: language={}, {state}",
            labels.russian,
        );
        for label in [labels.expand, labels.more] {
            assert_eq!(
                ElementHandle::find_by_accessible_label(bar, label).count(),
                1,
                "mini window action must be unique: label={label}, language={}, {state}",
                labels.russian,
            );
        }
    }
    // Mini has no state capsule. The existing mini-prefixed accessors expose
    // the other four named capsules in every form, not just the mini form.
    for (index, (capsule, preferred)) in [
        ("state", bar.get_state_content_width()),
        ("voice", bar.get_mini_voice_content_width()),
        ("session", bar.get_mini_session_content_width()),
        ("tiles", bar.get_mini_tiles_content_width()),
        ("window", bar.get_mini_window_content_width()),
    ]
    .into_iter()
    .enumerate()
    {
        if form == 2 && index == 0 {
            continue;
        }
        assert_eq!(
            preferred, declared[index],
            "{capsule} preferred/declared width mismatch: form={form}, language={}, {state}",
            labels.russian,
        );
    }
}

fn assert_form_states(bar: &ui::OverlayBarWindow, labels: &Labels) {
    let spec = bar.global::<ui::BarSpec>();
    // Include a tray pin: mini must expose neither it nor the compact tray chip.
    bar.set_pins(slint::ModelRc::new(slint::VecModel::from(vec![
        slint::SharedString::from("tray"),
    ])));
    for compact in [false, true] {
        bar.set_compact_bar(compact);
        // Exercise both entry from compact and return from mini.
        for form in [0, 1, 2, 1, 0] {
            bar.set_bar_form(form);
            assert_eq!(
                ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
                1
            );
            let width = match form {
                0 => spec.get_full_width(),
                1 => spec.get_compact_width(),
                2 => spec.get_mini_width(),
                _ => panic!("unsupported bar form: {form}"),
            };
            bar.window()
                .set_size(LogicalSize::new(width, spec.get_height()));
            for active in [false, true] {
                bar.set_stealth_active(active);
                for fault in [false, true] {
                    bar.set_stealth_fault(fault);
                    for tray in [false, true] {
                        bar.set_tray_available(tray);
                        for timer in [false, true] {
                            bar.set_timer_active(timer);
                            for mode in [0, 1, 2] {
                                bar.set_autotile_mode(mode);
                                // The testing backend has no event loop. Settle
                                // conditional children before reading geometry.
                                slint::platform::update_timers_and_animations();
                                assert_form_contract(
                                    bar,
                                    labels,
                                    form,
                                    &format!(
                                        "compact={compact}, active={active}, fault={fault}, \
                                         tray={tray}, timer={timer}, mode={mode}",
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    bar.set_pins(slint::ModelRc::new(
        slint::VecModel::<slint::SharedString>::default(),
    ));
    bar.set_stealth_active(false);
    bar.set_stealth_fault(false);
    bar.set_tray_available(false);
    bar.set_timer_active(false);
    bar.set_autotile_mode(1);
    bar.set_compact_bar(false);
    bar.set_bar_form(0);
    assert_eq!(
        ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
        1
    );
    slint::platform::update_timers_and_animations();
}

fn assert_layout(bar: &ui::OverlayBarWindow, labels: &Labels, compact: bool) {
    bar.set_compact_bar(compact);
    bar.set_bar_form(i32::from(compact));
    assert_eq!(
        ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
        1
    );
    bar.set_lock_a11y(labels.deep_lock.into());
    bar.set_autotile_mode(1);
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
    assert_eq!(spec.get_full_tiles(), 246.0);
    assert_eq!(spec.get_compact_tiles(), 30.0);
    assert_eq!(spec.get_full_width(), 874.0);
    assert_eq!(spec.get_compact_width(), 586.0);
    assert!(spec.get_full_width() > spec.get_compact_width());
    bar.window().set_size(LogicalSize::new(width, height));
    slint::platform::update_timers_and_animations();
    let stable_names = if compact {
        vec![
            labels.ask,
            labels.grab,
            labels.stop,
            if labels.russian {
                "По вопросам"
            } else {
                "On questions"
            },
            labels.expand,
            labels.more,
        ]
    } else {
        vec![
            labels.ask,
            labels.grab,
            labels.stop,
            if labels.russian {
                "По вопросам"
            } else {
                "On questions"
            },
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
            bar.set_autotile_mode(mode);
            for deep in [false, true] {
                bar.set_deep_lock(deep);
                for recording in [false, true] {
                    bar.set_mic_recording(recording);
                    bar.set_sys_recording(recording);
                    bar.set_session_paused(recording);
                    bar.set_menu_open(recording);
                    // Armed confirmations must no longer expand or hide bar controls.
                    bar.set_quit_armed(recording);
                    bar.set_restart_armed(recording);
                    let russian = labels.russian;
                    let lock_label = match (russian, mode) {
                        (true, 0) => "Молчат: сами не появляются",
                        (true, 2) => "На каждую реплику",
                        (true, _) => "По вопросам",
                        (false, 0) => "Silent: no automatic tiles",
                        (false, 2) => "On every line",
                        (false, _) => "On questions",
                    };
                    let mut names = vec![labels.ask, labels.grab, labels.stop, lock_label];
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
                    slint::platform::update_timers_and_animations();
                    let natural: Vec<f32> = names
                        .iter()
                        .map(|name| element(bar, name).size().width)
                        .collect();
                    bar.window().set_size(LogicalSize::new(width, height));
                    assert_eq!(
                        bar.window().size().width as f32,
                        width,
                        "mode/deep must not resize bar"
                    );
                    assert_eq!(
                        if compact {
                            spec.get_compact_width()
                        } else {
                            spec.get_full_width()
                        },
                        width
                    );
                    if deep {
                        let item = element(bar, labels.deep_lock);
                        let pos = item.absolute_position();
                        let size = item.size();
                        assert_eq!(size.width, 70.0);
                        let status = element(
                            bar,
                            if labels.russian {
                                "[!] скрытность недоступна"
                            } else {
                                "[!] stealth unavailable"
                            },
                        );
                        assert!(
                            status.absolute_position().x + status.size().width <= pos.x,
                            "AI off must follow status without overlap"
                        );
                        assert!(
                            pos.y >= 34.0 && pos.y + size.height <= height,
                            "AI off outside footer"
                        );
                        assert!(
                            pos.x >= 0.0 && pos.x + size.width <= width,
                            "AI off outside bar"
                        );
                    } else {
                        assert_eq!(
                            ElementHandle::find_by_accessible_label(bar, labels.deep_lock).count(),
                            0
                        );
                    }
                    let items: Vec<ElementHandle> =
                        names.iter().map(|name| element(bar, name)).collect();
                    assert!(
                        bar.get_autotile_word_fits(),
                        "auto-tiles word clipped: compact={compact}, mode={mode}"
                    );
                    assert!(
                        bar.get_all_regions_fit(),
                        "capsule overflow: compact={compact}, mode={mode}, \
                         tiles={count}, recording={recording}",
                    );
                    assert_form_contract(
                        bar,
                        labels,
                        i32::from(compact),
                        &format!("mode={mode}, tiles={count}, recording={recording}, deep={deep}"),
                    );
                    for ((name, item), natural_width) in names.iter().zip(&items).zip(natural) {
                        assert_top_bounds(item, width, name);
                        assert_eq!(item.size().width, natural_width, "{name} was compressed");
                    }
                    for pair in items.windows(2) {
                        assert!(
                            pair[0].absolute_position().x + pair[0].size().width
                                <= pair[1].absolute_position().x,
                            "capsule controls overlap: compact={compact}, \
                             mode={mode}, tiles={count}",
                        );
                    }
                    for (name, x) in stable_names.iter().zip(&stable_x) {
                        let name = if *name == "On questions" || *name == "По вопросам" {
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
                        if compact { 26.0 } else { 96.0 }
                    );
                }
            }
        }
    }
    bar.set_deep_lock(false);
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
    slint::platform::update_timers_and_animations();
    assert_form_contract(bar, labels, i32::from(compact), "empty tray/pause slots");
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

fn assert_mini(bar: &ui::OverlayBarWindow, labels: &Labels) {
    let spec = bar.global::<ui::BarSpec>();
    let width = spec.get_mini_width();
    let height = spec.get_height();
    assert_eq!(spec.get_mini_voice(), 74.0);
    assert_eq!(spec.get_mini_session(), 114.0);
    assert_eq!(spec.get_mini_tiles(), 58.0);
    assert_eq!(spec.get_mini_window(), 58.0);
    assert_eq!(width, 408.0);
    bar.set_lock_a11y(labels.deep_lock.into());
    bar.set_timer_active(true);
    bar.set_timer_label("1:07:31".into());
    bar.set_session_name("HIDDEN SESSION".into());
    bar.set_last_transcript_line("MINI TRANSCRIPT".into());
    bar.set_app_memory("HIDDEN APP RAM".into());
    bar.set_mlx_memory("HIDDEN MLX RAM".into());
    bar.set_tray_available(true);
    bar.set_can_restore_tile(true);
    bar.set_pins(slint::ModelRc::new(slint::VecModel::from(vec![
        slint::SharedString::from("archive"),
    ])));
    bar.set_compact_bar(false); // Mini must work even with the full preference.
    bar.set_bar_form(2);
    assert_eq!(
        ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
        1
    );
    for fault in [false, true] {
        bar.set_stealth_fault(fault);
        for tone in [
            ui::StatusTone::Neutral,
            ui::StatusTone::Ok,
            ui::StatusTone::Warning,
            ui::StatusTone::Danger,
            ui::StatusTone::Info,
        ] {
            bar.set_status_tone(tone);
            bar.set_status_text("STATUS".into());
            let status = if fault {
                if labels.russian {
                    "[!] скрытность недоступна"
                } else {
                    "[!] stealth unavailable"
                }
            } else {
                "STATUS"
            };
            for mode in [0, 1, 2] {
                bar.set_autotile_mode(mode);
                let switch = match (labels.russian, mode) {
                    (true, 0) => "Молчат: сами не появляются",
                    (true, 2) => "На каждую реплику",
                    (true, _) => "По вопросам",
                    (false, 0) => "Silent: no automatic tiles",
                    (false, 2) => "On every line",
                    (false, _) => "On questions",
                };
                for recording in [false, true] {
                    bar.set_mic_recording(recording);
                    bar.set_sys_recording(recording);
                    bar.set_session_paused(recording);
                    let pause = match (labels.russian, recording) {
                        (true, true) => "Продолжить сессию",
                        (true, false) => "Приостановить сессию",
                        (false, true) => "Resume the session",
                        (false, false) => "Pause the session",
                    };
                    for deep in [false, true] {
                        bar.set_deep_lock(deep);
                        for (source, speaker) in [("mic", labels.you), ("sys", labels.them)] {
                            bar.set_last_transcript_source(source.into());
                            bar.window().set_size(LogicalSize::new(width, height));
                            slint::platform::update_timers_and_animations();
                            // Conditional children are created lazily: walk the tree
                            // once so the capsules hold the mini form's chips before
                            // their preferred widths are read.
                            assert_eq!(
                                ElementHandle::find_by_accessible_label(bar, labels.stealth)
                                    .count(),
                                1,
                                "mini form must show exactly one stealth chip"
                            );
                            assert_eq!(bar.window().size().width as f32, width);
                            assert!(
                                bar.get_all_regions_fit(),
                                "mini capsule overflow: language={}, fault={fault}, \
                                 mode={mode}, recording={recording}, deep={deep}; \
                                 fits: voice={}, session={}, tiles={}, window={}; \
                                 preferred/declared px: voice={}/{}, session={}/{}, \
                                 tiles={}/{}, window={}/{}",
                                labels.russian,
                                bar.get_mini_voice_fits(),
                                bar.get_mini_session_fits(),
                                bar.get_mini_tiles_fits(),
                                bar.get_mini_window_fits(),
                                bar.get_mini_voice_content_width(),
                                spec.get_mini_voice(),
                                bar.get_mini_session_content_width(),
                                spec.get_mini_session(),
                                bar.get_mini_tiles_content_width(),
                                spec.get_mini_tiles(),
                                bar.get_mini_window_content_width(),
                                spec.get_mini_window(),
                            );
                            assert_form_contract(
                                bar,
                                labels,
                                2,
                                &format!(
                                    "fault={fault}, mode={mode}, recording={recording}, \
                                     deep={deep}, source={source}",
                                ),
                            );
                            assert!(bar.get_autotile_word_fits());
                            let top_names = [
                                labels.ask,
                                labels.grab,
                                labels.stop,
                                pause,
                                labels.stealth,
                                switch,
                                labels.expand,
                                labels.more,
                            ];
                            let top: Vec<_> =
                                top_names.iter().map(|name| element(bar, name)).collect();
                            for (name, item) in top_names.iter().zip(&top) {
                                assert_top_bounds(item, width, name);
                            }
                            for pair in top.windows(2) {
                                assert!(
                                    pair[0].absolute_position().x + pair[0].size().width
                                        <= pair[1].absolute_position().x,
                                    "mini top controls overlap"
                                );
                            }
                            for name in [labels.ask, labels.grab] {
                                assert_eq!(
                                    element(bar, name).size().width,
                                    34.0,
                                    "mini voice icon width"
                                );
                            }
                            assert_eq!(element(bar, switch).size().width, 26.0);
                            let mut footer_names = vec![status];
                            if deep {
                                footer_names.push(labels.deep_lock);
                            }
                            footer_names.extend([speaker, "MINI TRANSCRIPT"]);
                            let footer: Vec<_> =
                                footer_names.iter().map(|name| element(bar, name)).collect();
                            for (name, item) in footer_names.iter().zip(&footer) {
                                let pos = item.absolute_position();
                                let size = item.size();
                                assert!(
                                    pos.y >= 34.0 && pos.y + size.height <= height,
                                    "{name} escaped mini footer"
                                );
                                assert!(
                                    pos.x >= spec.get_pad() + spec.get_badge()
                                        && pos.x + size.width <= width,
                                    "{name} escaped mini bounds"
                                );
                            }
                            for pair in footer.windows(2) {
                                assert!(
                                    pair[0].absolute_position().x + pair[0].size().width
                                        <= pair[1].absolute_position().x,
                                    "mini footer overlaps"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    for hidden in [
        labels.tray,
        labels.write,
        labels.compact,
        "HIDDEN SESSION",
        "MLX HIDDEN MLX RAM",
        if labels.russian {
            "закрепить"
        } else {
            "pin"
        },
        if labels.russian {
            "Архив сессий"
        } else {
            "Session archive"
        },
    ] {
        assert_eq!(
            ElementHandle::find_by_accessible_label(bar, hidden).count(),
            0,
            "mini must not expose {hidden}"
        );
    }
    bar.set_pins(slint::ModelRc::new(
        slint::VecModel::<slint::SharedString>::default(),
    ));
    bar.set_app_memory("".into());
    bar.set_mlx_memory("".into());
    bar.set_session_name("".into());
    bar.set_last_transcript_line("".into());
    bar.set_deep_lock(false);
    bar.set_mic_recording(false);
    bar.set_sys_recording(false);
    bar.set_stealth_fault(false);
    bar.set_status_text("STATUS".into());
    let mascot_label = format!(
        "STATUS. {}",
        if labels.russian {
            "Перетащить = двигать панель; показывает состояние"
        } else {
            "Drag to move the bar; shows the live state"
        }
    );
    let mut mascot_position = None;
    for (form, form_width) in [
        (0, spec.get_full_width()),
        (1, spec.get_compact_width()),
        (2, width),
    ] {
        bar.set_bar_form(form);
        assert_eq!(
            ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
            1
        );
        bar.window().set_size(LogicalSize::new(form_width, height));
        slint::platform::update_timers_and_animations();
        let mascot = element(bar, &mascot_label);
        let pos = mascot.absolute_position();
        let size = mascot.size();
        assert_eq!((size.width, size.height), (48.0, 48.0));
        assert!(
            pos.x >= 0.0
                && pos.y >= 0.0
                && pos.x + size.width <= form_width
                && pos.y + size.height <= height
        );
        if let Some(expected) = mascot_position {
            assert_eq!((pos.x, pos.y), expected, "form={form} moved mascot");
        } else {
            mascot_position = Some((pos.x, pos.y));
        }
    }
}

fn assert_pins(bar: &ui::OverlayBarWindow, russian: bool) {
    let ids = [
        "capture", "restore", "archive", "summary", "kb", "settings", "help", "tray",
    ];
    let names = if russian {
        [
            "Вопрос по снимку экрана",
            "Вернуть закрытый тайл",
            "Архив сессий",
            "Итог встречи",
            "База знаний",
            "Настройки",
            "Справка",
            "Свернуть в трей",
        ]
    } else {
        [
            "Ask about a screenshot",
            "Restore closed tile",
            "Session archive",
            "Meeting summary",
            "Knowledge base",
            "Settings",
            "Help",
            "Hide to tray",
        ]
    };
    let hint = if russian { "закрепить" } else { "pin" };
    let width = bar.global::<ui::BarSpec>().get_full_width();
    let height = bar.global::<ui::BarSpec>().get_height();
    bar.set_compact_bar(false);
    bar.set_bar_form(0);
    assert_eq!(
        ElementHandle::find_by_accessible_label(
            bar,
            if russian {
                RUSSIAN.stealth
            } else {
                ENGLISH.stealth
            },
        )
        .count(),
        1,
    );
    bar.set_last_transcript_line("PIN GEOMETRY TRANSCRIPT".into());
    bar.set_can_restore_tile(true);
    bar.set_autotile_mode(2);
    bar.set_tray_available(true);
    for (iteration, count) in [0, 3, 8].into_iter().enumerate() {
        let state = format!(
            "language={}, pins={count}, iteration={iteration}",
            if russian { "ru" } else { "en" }
        );
        bar.set_pins(slint::ModelRc::new(slint::VecModel::from(
            ids[..count]
                .iter()
                .map(|id| slint::SharedString::from(*id))
                .collect::<Vec<_>>(),
        )));
        bar.window().set_size(LogicalSize::new(width, height));
        slint::platform::update_timers_and_animations();
        assert_eq!(
            bar.window().size().width as f32,
            width,
            "full bar, {state}: pins must not resize the bar"
        );
        let hints: Vec<_> = ElementHandle::find_by_accessible_label(bar, hint).collect();
        assert_eq!(
            hints.len(),
            usize::from(count == 0),
            "label={hint}, full bar, {state}: hint presence"
        );
        if let Some(hint) = hints.first() {
            let more = element(bar, if russian { "Ещё" } else { "More" });
            let anchor = std::rc::Rc::new(std::cell::Cell::new((0.0_f32, 0.0_f32)));
            let anchor_for_callback = anchor.clone();
            bar.on_menu_opened(move |x, y| anchor_for_callback.set((x, y)));
            hint.invoke_accessible_default_action();
            let pos = more.absolute_position();
            let size = more.size();
            assert_eq!(
                anchor.get(),
                (pos.x + size.width, pos.y + size.height),
                "label=pin/More, full bar, {state}: hint must use the More chip's anchor"
            );
        }
        let transcript = element(bar, "PIN GEOMETRY TRANSCRIPT");
        let mut right = transcript.absolute_position().x + transcript.size().width;
        for name in &names[..count] {
            let name = *name;
            let matching: Vec<_> = ElementHandle::find_by_accessible_label(bar, name).collect();
            assert_eq!(
                matching.len(),
                1,
                "label={name}, full bar, {state}: one pinned action"
            );
            let item = &matching[0];
            let pos = item.absolute_position();
            let size = item.size();
            assert_eq!(
                size.width, 26.0,
                "label={name}, full bar, {state}: fixed bare pin width"
            );
            assert_eq!(
                size.height, 22.0,
                "label={name}, full bar, {state}: fixed bare pin height"
            );
            assert!(
                pos.y >= 34.0 && pos.y + size.height <= height,
                "label={name}, full bar, {state}: pin escaped bottom row"
            );
            assert!(
                pos.x >= right && pos.x + size.width <= width,
                "label={name}, full bar, {state}: \
                 pin overlaps transcript/previous pin or escapes bar",
            );
            right = pos.x + size.width;
        }
        assert_eq!(
            ElementHandle::find_by_accessible_label(
                bar,
                if russian {
                    "Агрессивные авто-тайлы"
                } else {
                    "Aggressive auto-tile"
                }
            )
            .count(),
            0,
            "label=Aggressive auto-tile, full bar, {state}: legacy flame is removed"
        );
        {
            let id = "restore";
            let restore_label = if russian {
                "Вернуть закрытый тайл"
            } else {
                "Restore closed tile"
            };
            assert_eq!(
                ElementHandle::find_by_accessible_label(bar, restore_label).count(),
                1,
                "label={restore_label}, id={id}, full bar, {state}: fallback or pin, never both"
            );
        }
        bar.set_compact_bar(true);
        bar.set_bar_form(1);
        assert_eq!(
            ElementHandle::find_by_accessible_label(
                bar,
                if russian {
                    RUSSIAN.stealth
                } else {
                    ENGLISH.stealth
                },
            )
            .count(),
            1,
        );
        bar.window().set_size(LogicalSize::new(
            bar.global::<ui::BarSpec>().get_compact_width(),
            height,
        ));
        slint::platform::update_timers_and_animations();
        assert_eq!(
            ElementHandle::find_by_accessible_label(bar, hint).count(),
            0,
            "label={hint}, compact bar, {state}: compact bar must not show pin hint"
        );
        // Labels identify actions, not rows: the compact window capsule has
        // its own tray chip. Pins belong to the footer; do not count that chip
        // as a pin, but still require exactly one top-row tray action.
        for (id, name) in ids.iter().zip(&names) {
            let matching: Vec<_> = ElementHandle::find_by_accessible_label(bar, name).collect();
            assert_eq!(
                matching.iter().filter(|item| item.absolute_position().y >= 34.0).count(),
                0,
                "label={name}, id={id}, compact bar, {state}: compact bar must not show footer pins"
            );
            assert_eq!(
                matching.len(),
                usize::from(*id == "tray"),
                "label={name}, id={id}, compact bar, {state}: \
                 only the built-in tray action may remain",
            );
            for item in &matching {
                assert_top_bounds(
                    item,
                    bar.global::<ui::BarSpec>().get_compact_width(),
                    &format!("label={name}, compact bar, {state}"),
                );
            }
        }
        bar.set_compact_bar(false);
        bar.set_bar_form(0);
        assert_eq!(
            ElementHandle::find_by_accessible_label(
                bar,
                if russian {
                    RUSSIAN.stealth
                } else {
                    ENGLISH.stealth
                },
            )
            .count(),
            1,
        );
        slint::platform::update_timers_and_animations();
    }
    bar.set_pins(slint::ModelRc::new(
        slint::VecModel::<slint::SharedString>::default(),
    ));
    bar.set_can_restore_tile(false);
    bar.set_autotile_mode(1);
    bar.set_last_transcript_line("".into());
}

fn assert_audit_states(bar: &ui::OverlayBarWindow, labels: &Labels) {
    let start = if labels.russian {
        "Начать сессию"
    } else {
        "Start the session"
    };
    let listening = if labels.russian {
        "слушаю…"
    } else {
        "listening…"
    };
    let toggles = std::rc::Rc::new(std::cell::Cell::new(0));
    let toggles_for_callback = toggles.clone();
    bar.on_timer_toggle_clicked(move || toggles_for_callback.set(toggles_for_callback.get() + 1));
    bar.set_status_text("idle".into());
    bar.set_session_name("".into());
    bar.set_last_transcript_source("".into());
    bar.set_deep_lock(false);
    bar.set_tray_available(false);
    for form in [0, 1, 2] {
        bar.set_bar_form(form);
        assert_eq!(
            ElementHandle::find_by_accessible_label(bar, labels.stealth).count(),
            1
        );
        let spec = bar.global::<ui::BarSpec>();
        let width = match form {
            0 => spec.get_full_width(),
            1 => spec.get_compact_width(),
            _ => spec.get_mini_width(),
        };
        bar.window()
            .set_size(LogicalSize::new(width, spec.get_height()));
        for active in [false, true] {
            bar.set_timer_active(active);
            let timer_label = if active { labels.stop } else { start };
            assert_eq!(
                ElementHandle::find_by_accessible_label(bar, timer_label).count(),
                1
            );
            let before = toggles.get();
            element(bar, timer_label).invoke_accessible_default_action();
            assert_eq!(
                toggles.get(),
                before + 1,
                "timer accessible dispatch: form={form}"
            );
            for paused in [false, true] {
                bar.set_session_paused(paused);
                for transcript in ["", "AUDIT TRANSCRIPT"] {
                    bar.set_last_transcript_line(transcript.into());
                    for fault in [false, true] {
                        bar.set_stealth_fault(fault);
                        for tone in [
                            ui::StatusTone::Neutral,
                            ui::StatusTone::Ok,
                            ui::StatusTone::Info,
                            ui::StatusTone::Warning,
                            ui::StatusTone::Danger,
                        ] {
                            bar.set_status_tone(tone);
                            slint::platform::update_timers_and_animations();
                            // Drain conditional children before layout-derived reads.
                            let count =
                                ElementHandle::find_by_accessible_label(bar, listening).count();
                            let expected = form != 2
                                && active
                                && !paused
                                && transcript.is_empty()
                                && !fault
                                && !matches!(
                                    tone,
                                    ui::StatusTone::Warning | ui::StatusTone::Danger
                                );
                            assert_eq!(
                                count,
                                usize::from(expected),
                                "listening: form={form}, active={active}, paused={paused}, \
                                 transcript={transcript}, fault={fault}, tone={tone:?}",
                            );
                            if expected {
                                let cue = element(bar, listening);
                                let pos = cue.absolute_position();
                                let size = cue.size();
                                assert!(pos.y >= 34.0 && pos.y + size.height <= spec.get_height());
                                assert!(pos.x >= 0.0 && pos.x + size.width <= width);
                            }
                            assert_eq!(
                                ElementHandle::find_by_accessible_label(bar, labels.ask).count(),
                                1
                            );
                            assert_eq!(
                                ElementHandle::find_by_accessible_label(bar, labels.grab).count(),
                                1
                            );
                            assert_form_contract(
                                bar,
                                labels,
                                form,
                                "audit cue and accessible labels",
                            );
                        }
                    }
                }
            }
        }
    }
    bar.set_stealth_fault(false);
    bar.set_timer_active(false);
    bar.set_session_paused(false);
    bar.set_last_transcript_line("".into());
    bar.set_status_tone(ui::StatusTone::Neutral);
}

#[test]
fn capsules_stay_inside_spec_sizes_in_english_and_russian() {
    i_slint_backend_testing::init_no_event_loop();
    let english = ui::OverlayBarWindow::new().expect("create English bar");
    assert_layout(&english, &ENGLISH, false);
    assert_layout(&english, &ENGLISH, true);
    assert_mini(&english, &ENGLISH);
    assert_pins(&english, false);
    assert_form_states(&english, &ENGLISH);
    assert_audit_states(&english, &ENGLISH);
    slint::select_bundled_translation("ru").expect("select Russian translation");
    let russian = ui::OverlayBarWindow::new().expect("create Russian bar");
    assert_layout(&russian, &RUSSIAN, false);
    assert_layout(&russian, &RUSSIAN, true);
    assert_mini(&russian, &RUSSIAN);
    assert_pins(&russian, true);
    assert_form_states(&russian, &RUSSIAN);
    assert_audit_states(&russian, &RUSSIAN);
}

#[test]
fn state_capsule_centred_under_session_keeps_full_stealth_before_ask() {
    i_slint_backend_testing::init_no_event_loop();
    for (language, labels) in [("en", &ENGLISH), ("ru", &RUSSIAN)] {
        slint::select_bundled_translation(language).expect("select regression language");
        let bar = ui::OverlayBarWindow::new().expect("create regression bar");
        bar.set_bar_form(0);
        assert_eq!(
            ElementHandle::find_by_accessible_label(&bar, labels.stealth).count(),
            1,
        );
        let spec = bar.global::<ui::BarSpec>();
        bar.window()
            .set_size(LogicalSize::new(spec.get_full_width(), spec.get_height()));
        assert_eq!(
            ElementHandle::find_by_accessible_label(&bar, labels.stealth).count(),
            1,
        );
        for muted in [false, true] {
            bar.set_mic_muted(muted);
            bar.set_sys_active(!muted);
            assert_eq!(
                ElementHandle::find_by_accessible_label(&bar, labels.stealth).count(),
                1,
            );
            let stealth = element(&bar, labels.stealth);
            let ask = element(&bar, labels.ask);
            assert!(
                stealth.absolute_position().x + stealth.size().width / 2.0
                    < ask.absolute_position().x,
                "centred state capsule hides stealth under session: language={language}"
            );
            assert_top_order_and_capsules(&bar, labels, 0, "incident regression, mic state");
        }
    }
    slint::select_bundled_translation("en").expect("restore English translation");
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
    assert!(
        !indicator.contains("visible:"),
        "idle cell must not collapse"
    );
    assert!(record.contains("if (!self.recording) {\n            self.locked = false;"));
    assert!(record.contains("in property <bool> icon-only: false;"));
    for binding in [
        "padding-left: Metrics.space-xs;",
        "padding-right: Metrics.space-xs;",
        "spacing: root.icon-only ? 0px : 2px;",
        "slot-width: root.icon-only ? 18px : 14px;",
    ] {
        assert!(record.contains(binding), "mini record geometry: {binding}");
    }
    assert!(record.contains("accessible-label: root.a11y;"));
    assert!(
        record.contains("if !root.icon-only : Text {"),
        "mini has no text cell"
    );
    assert!(
        record.contains("if !root.icon-only : IconSlot {"),
        "mini has no tail slot"
    );
    assert!(record.contains("background: recording ? Theme.danger.with-alpha(0.2) : transparent;"));
    assert!(
        record.contains("root.press-y - self.mouse-y > root.lock-dy"),
        "mini retains swipe lock gesture"
    );
    assert!(
        source.contains("press-feedback: root.bar-form == 0 || root.bar-form == 1;"),
        "unsatisfied mini expand must not flash"
    );
}

#[test]
fn tile_counter_keeps_text_and_close_all_uses_a_reserved_icon_slot() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint"))
            .expect("read overlay bar source");
    let (tile_chip, tile_click) = source
        .split_once("width: 92px;")
        .expect("fixed tile counter slot")
        .1
        .split_once("clicked => {")
        .expect("tile action handler");
    let tile_click = tile_click
        .split_once('}')
        .expect("tile action handler boundary")
        .0;
    assert_eq!(
        tile_click.split_whitespace().collect::<Vec<_>>(),
        ["root.dismiss-menu();", "root.spawn-tile-clicked();"],
        "label=New tile, full bar, en/ru, all tile counts: dismiss More before spawning tile"
    );
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
