//! Headless action/pin/confirmation regression; native focus is audited separately.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize, Model, ModelRc, SharedString, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

mod ui {
    slint::include_modules!();
}

fn menu_element(menu: &ui::BarMenuWindow, label: &str) -> ElementHandle {
    // Yes/No also label their inner Text; preorder yields the enclosing cell.
    assert!(
        ElementHandle::find_by_accessible_label(menu, label).count() > 0,
        "missing menu element: {label}",
    );
    ElementHandle::find_by_accessible_label(menu, label)
        .next()
        .unwrap_or_else(|| panic!("missing menu target: {label}"))
}

fn activate(menu: &ui::BarMenuWindow, label: &str) {
    assert!(ElementHandle::find_by_accessible_label(menu, label).count() > 0);
    ElementHandle::find_by_accessible_label(menu, label)
        .next()
        .unwrap_or_else(|| panic!("missing menu target: {label}"))
        .invoke_accessible_default_action();
}

#[test]
fn rows_select_pins_do_not_select_and_danger_actions_require_yes() {
    i_slint_backend_testing::init_no_event_loop();
    let menu = ui::BarMenuWindow::new().expect("create More menu");
    menu.set_tray_available(true);
    menu.set_lock_menu_managed(true);
    menu.window().set_size(LogicalSize::new(408.0, 464.0));
    menu.show().expect("show menu");
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Ask about a screenshot").count(),
        1,
    );
    let mut name_x = None;
    let mut pin_x = None;
    for label in [
        "Ask about a screenshot",
        "Restore closed tile",
        "Session archive",
        "Meeting summary",
        "Knowledge base",
        "Settings",
        "Help",
        "Hide to tray",
        "Unload local AI",
        "Restart",
        "Quit",
    ] {
        let cell = ElementHandle::find_by_accessible_label(&menu, label)
            .next()
            .expect("name action cell");
        let text = ElementHandle::find_by_accessible_label(&menu, &format!("{label}."))
            .next()
            .expect("actual row name Text, not its enclosing cell");
        let x = text.absolute_position().x;
        assert_eq!(
            *name_x.get_or_insert(x),
            x,
            "every row name Text must start at the same x: {label}"
        );
        assert_eq!(
            x,
            cell.absolute_position().x,
            "name starts at the left edge of its cell"
        );
        assert_eq!(x, 36.0, "6px panel + 8px row pad + 14px icon + 8px gap");
        assert_eq!(
            text.size(),
            cell.size(),
            "name fills the elastic column for left alignment and right elision"
        );
    }
    for label in [
        "Ask about a screenshot",
        "Restore closed tile",
        "Session archive",
        "Meeting summary",
        "Knowledge base",
        "Settings",
        "Help",
        "Hide to tray",
    ] {
        let name = ElementHandle::find_by_accessible_label(&menu, label)
            .next()
            .expect("name cell");
        let pin = ElementHandle::find_by_accessible_label(&menu, &format!("{label}. pin"))
            .next()
            .expect("pin cell");
        let x = name.absolute_position().x;
        let px = pin.absolute_position().x;
        assert_eq!(*name_x.get_or_insert(x), x, "name columns must align");
        assert_eq!(*pin_x.get_or_insert(px), px, "pin columns must align");
        assert_eq!(pin.size().width, 78.0, "fixed pin column");
        assert_eq!(pin.size().height, 24.0, "pin target height");
        assert_eq!(pin.absolute_position().y, name.absolute_position().y + 2.0);
        assert_eq!(name.size().height, 28.0, "row height stays fixed");
        assert!(px + pin.size().width <= 408.0);
    }
    let actions = Rc::new(RefCell::new(Vec::<String>::new()));
    let actions_for_callback = actions.clone();
    let menu_for_action = menu.as_weak();
    menu.on_action(move |id| {
        actions_for_callback.borrow_mut().push(id.to_string());
        menu_for_action
            .upgrade()
            .unwrap()
            .hide()
            .expect("dismiss selected menu action");
    });
    let pins = Rc::new(RefCell::new(Vec::<String>::new()));
    let pins_for_callback = pins.clone();
    menu.on_pin_toggled(move |id| pins_for_callback.borrow_mut().push(id.to_string()));

    activate(&menu, "Ask about a screenshot. pin");
    assert_eq!(&*pins.borrow(), &["capture"]);
    assert!(
        actions.borrow().is_empty(),
        "pin button must not choose an action"
    );
    assert!(
        menu.window().is_visible(),
        "pin button must not dismiss the menu"
    );
    menu.set_pins(ModelRc::new(VecModel::from(vec![SharedString::from(
        "capture",
    )])));
    activate(&menu, "Ask about a screenshot. on the bar");
    assert_eq!(&*pins.borrow(), &["capture", "capture"]);
    assert!(actions.borrow().is_empty());

    activate(&menu, "Restore closed tile");
    activate(&menu, "Restore closed tile. pin");
    assert!(
        actions.borrow().is_empty(),
        "disabled restore action is inert"
    );
    assert_eq!(&*pins.borrow(), &["capture", "capture", "restore"]);
    menu.set_pins(ModelRc::new(VecModel::from(vec![SharedString::from(
        "restore",
    )])));
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Restore closed tile. on the bar").count(),
        1,
    );
    activate(&menu, "Restore closed tile. on the bar");
    assert_eq!(
        &*pins.borrow(),
        &["capture", "capture", "restore", "restore"]
    );
    assert!(
        actions.borrow().is_empty(),
        "unpin does not execute restore"
    );
    assert!(menu.window().is_visible(), "unavailable pins stay editable");
    menu.set_can_restore_tile(true);
    for (label, id) in [
        ("Ask about a screenshot", "capture"),
        ("Unload local AI", "unload_ai"),
        ("Restore closed tile", "restore"),
        ("Session archive", "archive"),
        ("Meeting summary", "summary"),
        ("Knowledge base", "kb"),
        ("Settings", "settings"),
        ("Help", "help"),
        ("Hide to tray", "tray"),
    ] {
        menu.show().expect("reopen before selecting each action");
        activate(&menu, label);
        assert_eq!(actions.borrow().last().map(String::as_str), Some(id));
        assert!(
            !menu.window().is_visible(),
            "menu action must dismiss: {id}"
        );
    }
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Auto-tiles on every line").count(),
        0
    );
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Unload local AI. pin").count(),
        0
    );
    menu.set_deep_lock(true);
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Unload local AI").count(),
        0
    );
    menu.show().expect("show load row");
    activate(&menu, "Load local AI");
    assert_eq!(
        actions.borrow().last().map(String::as_str),
        Some("unload_ai")
    );
    assert!(!menu.window().is_visible());
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Load local AI. pin").count(),
        0
    );
    menu.set_lock_menu_managed(false);
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Load local AI").count(),
        0
    );
    menu.set_deep_lock(false);
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Unload local AI").count(),
        0
    );
    menu.set_tray_available(false);
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Hide to tray").count(),
        0
    );
    for (label, id) in [("Restart", "restart"), ("Quit", "quit")] {
        menu.show().expect("reopen before danger action");
        let before = actions.borrow().len();
        activate(&menu, label);
        assert_eq!(menu.get_confirmation().as_str(), id);
        assert!(
            menu.window().is_visible(),
            "arming confirmation must not dismiss"
        );
        assert_eq!(
            actions.borrow().len(),
            before,
            "first click must not dispatch {id}"
        );
        assert!(ElementHandle::find_by_accessible_label(&menu, "Yes").count() > 0);
        let question = ElementHandle::find_by_accessible_label(&menu, &format!("{label}?."))
            .next()
            .expect("confirmation question Text in name column");
        assert_eq!(question.absolute_position().x, name_x.unwrap());
        let yes = ElementHandle::find_by_accessible_label(&menu, "Yes")
            .next()
            .expect("Yes in key cell");
        let no = ElementHandle::find_by_accessible_label(&menu, "No")
            .next()
            .expect("No in pin cell");
        assert_eq!(yes.size().width, 96.0);
        assert_eq!(no.size().width, 78.0);
        assert_eq!(yes.size().height, 24.0);
        assert_eq!(no.size().height, 24.0);
        assert_eq!(
            yes.absolute_position().y,
            question.absolute_position().y + 2.0
        );
        assert_eq!(no.absolute_position().y, yes.absolute_position().y);
        assert_eq!(no.absolute_position().x, pin_x.unwrap());
        assert_eq!(
            yes.absolute_position().x + yes.size().width + 8.0,
            no.absolute_position().x
        );
        activate(&menu, "No");
        assert_eq!(menu.get_confirmation().as_str(), "");
        assert_eq!(actions.borrow().len(), before);
        activate(&menu, label);
        activate(&menu, "Yes");
        assert_eq!(menu.get_confirmation().as_str(), "");
        assert_eq!(actions.borrow().len(), before + 1);
        assert_eq!(actions.borrow().last().map(String::as_str), Some(id));
        assert!(
            !menu.window().is_visible(),
            "confirmed danger action must dismiss"
        );
    }
    pin_save_failure_footer_is_bilingual_temporary_and_does_not_dispatch();
    sound_rows_dispatch_bar_toggles_without_dismissal_in_reduced_forms();
    more_menu_rows_and_columns_do_not_overlap_in_english_and_russian();
}

fn sound_rows_dispatch_bar_toggles_without_dismissal_in_reduced_forms() {
    for (language, mic, sys, on, off) in [
        ("en", "Microphone", "System audio", "on", "off"),
        ("ru", "Микрофон", "Звук системы", "вкл", "выкл"),
    ] {
        slint::select_bundled_translation(language).expect("sound language");
        for (form, too_narrow) in [(0, false), (1, false), (2, false), (2, true)] {
            let bar = ui::OverlayBarWindow::new().expect("sound bar");
            bar.set_bar_form(form);
            bar.set_too_narrow(too_narrow);
            let menu = ui::BarMenuWindow::new().expect("sound menu");
            menu.window().set_size(LogicalSize::new(408.0, 408.0));
            menu.show().expect("show sound menu");
            let calls = Rc::new(RefCell::new(Vec::new()));
            let weak = bar.as_weak();
            let mic_calls = calls.clone();
            bar.on_mic_toggle_clicked(move || {
                let bar = weak.upgrade().expect("mic bar");
                bar.set_mic_muted(!bar.get_mic_muted());
                mic_calls.borrow_mut().push("microphone");
            });
            let weak = bar.as_weak();
            let sys_calls = calls.clone();
            bar.on_sys_toggle_clicked(move || {
                let bar = weak.upgrade().expect("sys bar");
                bar.set_sys_active(!bar.get_sys_active());
                sys_calls.borrow_mut().push("system_audio");
            });
            let weak_bar = bar.as_weak();
            let weak_menu = menu.as_weak();
            menu.on_action(move |id| {
                let bar = weak_bar.upgrade().expect("dispatch bar");
                match id.as_str() {
                    "microphone" => bar.invoke_mic_toggle_clicked(),
                    "system_audio" => bar.invoke_sys_toggle_clicked(),
                    _ => panic!("unexpected sound action"),
                }
                let menu = weak_menu.upgrade().expect("refresh menu");
                menu.set_mic_muted(bar.get_mic_muted());
                menu.set_sys_active(bar.get_sys_active());
            });
            for (muted, active) in [(false, false), (true, true)] {
                bar.set_mic_muted(muted);
                bar.set_sys_active(active);
                menu.set_mic_muted(muted);
                menu.set_sys_active(active);
                for (label, enabled) in [(mic, !muted), (sys, active)] {
                    assert_eq!(
                        ElementHandle::find_by_accessible_label(&menu, label).count(),
                        1
                    );
                    let name = menu_element(&menu, label);
                    assert_eq!(name.absolute_position().x, 36.0);
                    assert_eq!(name.size().height, 28.0);
                    assert_eq!(
                        if label == mic {
                            !menu.get_mic_muted()
                        } else {
                            menu.get_sys_active()
                        },
                        enabled,
                    );
                    assert_eq!(
                        ElementHandle::find_by_accessible_label(&menu, &format!("{label}. pin"),)
                            .count(),
                        0,
                    );
                    let word = if enabled { on } else { off };
                    assert!(ElementHandle::find_by_accessible_label(&menu, word).count() > 0);
                }
                activate(&menu, mic);
                assert_eq!(menu.get_mic_muted(), !muted);
                assert!(menu.window().is_visible());
                activate(&menu, sys);
                assert_eq!(menu.get_sys_active(), !active);
                assert!(menu.window().is_visible());
                assert_eq!(!menu.get_mic_muted(), muted);
                assert_eq!(menu.get_sys_active(), !active);
            }
            assert_eq!(
                &*calls.borrow(),
                &["microphone", "system_audio", "microphone", "system_audio"],
            );
        }
    }
    slint::select_bundled_translation("en").expect("restore sound language");
}

fn assert_menu_grid(menu: &ui::BarMenuWindow, russian: bool) {
    let rows = if russian {
        [
            "Микрофон",
            "Звук системы",
            "Вопрос по снимку экрана",
            "Вернуть закрытый тайл",
            "Архив сессий",
            "Итог встречи",
            "База знаний",
            "Настройки",
            "Справка",
            "Свернуть в трей",
            if menu.get_deep_lock() {
                "Загрузить локальный AI"
            } else {
                "Выгрузить локальный AI"
            },
            "Перезапустить",
            "Выйти",
        ]
    } else {
        [
            "Microphone",
            "System audio",
            "Ask about a screenshot",
            "Restore closed tile",
            "Session archive",
            "Meeting summary",
            "Knowledge base",
            "Settings",
            "Help",
            "Hide to tray",
            if menu.get_deep_lock() {
                "Load local AI"
            } else {
                "Unload local AI"
            },
            "Restart",
            "Quit",
        ]
    };
    let headings = if russian {
        ["ЗВУК", "ТАЙЛЫ", "ВСТРЕЧА", "ПРИЛОЖЕНИЕ"]
    } else {
        ["SOUND", "TILES", "MEETING", "APP"]
    };
    // Instantiating the entire conditional tree precedes all position reads.
    assert_eq!(
        ElementHandle::find_by_accessible_label(menu, rows[0]).count(),
        1
    );
    let first = menu_element(menu, rows[0]);
    let name_x = first.absolute_position().x;
    let name_width = first.size().width;
    let key_x = name_x + name_width + 8.0;
    let pin_x = key_x + 96.0 + 8.0;
    assert_eq!(name_x, 36.0);
    assert_eq!(pin_x + 78.0 + 8.0, 408.0 - 6.0);
    let mut previous_bottom = 6.0;
    // Snapshot rectangles: later conditional probes invalidate element handles.
    let mut targets = Vec::new();
    for (index, label) in rows.iter().enumerate() {
        let heading = match index {
            0 => Some(headings[0]),
            2 => Some(headings[1]),
            4 => Some(headings[2]),
            7 => Some(headings[3]),
            _ => None,
        };
        if let Some(heading) = heading {
            let item = menu_element(menu, heading);
            assert_eq!(
                item.absolute_position().y,
                previous_bottom,
                "heading: {heading}"
            );
            assert_eq!(item.size().height, 22.0);
            previous_bottom = item.absolute_position().y + item.size().height;
            targets.push((heading.to_owned(), item.absolute_position(), item.size()));
        }
        if (index == 9 && !menu.get_tray_available())
            || (index == 10 && !menu.get_lock_menu_managed())
        {
            assert_eq!(
                ElementHandle::find_by_accessible_label(menu, label).count(),
                0
            );
            continue;
        }
        let name = menu_element(menu, label);
        let pos = name.absolute_position();
        let size = name.size();
        assert_eq!(pos.y, previous_bottom, "row order: {label}");
        assert_eq!(size.height, 28.0, "row height: {label}");
        assert_eq!(pos.x, name_x, "name column: {label}");
        assert_eq!(size.width, name_width, "name column width: {label}");
        // Empty cells are unlabelled. In this inert fixture, temporarily put
        // Yes/No in this row to measure the actual key/pin cells, then restore
        // the valid state. Do not dispatch actions in the synthetic probe.
        let saved_confirmation = menu.get_confirmation();
        let id = [
            "microphone",
            "system_audio",
            "capture",
            "restore",
            "archive",
            "summary",
            "kb",
            "settings",
            "help",
            "tray",
            "unload_ai",
            "restart",
            "quit",
        ][index];
        menu.set_confirmation(id.into());
        assert!(
            ElementHandle::find_by_accessible_label(menu, if russian { "Да" } else { "Yes" })
                .count()
                > 0,
        );
        let key_cell = menu_element(menu, if russian { "Да" } else { "Yes" });
        let pin_cell = menu_element(menu, if russian { "Нет" } else { "No" });
        assert_eq!(
            key_cell.absolute_position().x,
            key_x,
            "actual key cell: {label}"
        );
        assert_eq!(
            pin_cell.absolute_position().x,
            pin_x,
            "actual pin cell: {label}"
        );
        assert_eq!(key_cell.absolute_position().y, pos.y + 2.0);
        assert_eq!(pin_cell.absolute_position().y, pos.y + 2.0);
        assert_eq!(key_cell.size(), LogicalSize::new(96.0, 24.0));
        assert_eq!(pin_cell.size(), LogicalSize::new(78.0, 24.0));
        assert!(pos.x + size.width < key_cell.absolute_position().x);
        assert!(
            key_cell.absolute_position().x + key_cell.size().width < pin_cell.absolute_position().x
        );
        menu.set_confirmation(saved_confirmation);
        assert_eq!(
            ElementHandle::find_by_accessible_label(menu, label).count(),
            1
        );
        previous_bottom = pos.y + size.height;
        let confirmation = match index {
            11 => menu.get_confirmation() == "restart",
            12 => menu.get_confirmation() == "quit",
            _ => false,
        };
        let text_label = if confirmation {
            format!("{label}?.")
        } else {
            format!("{label}.")
        };
        let text = menu_element(menu, &text_label);
        assert_eq!(text.absolute_position(), pos);
        assert_eq!(text.size(), size);
        if (2..10).contains(&index) {
            let pinned = menu.get_pins().row_count() != 0;
            // The fixture uses either all pinnable IDs or none, in row order.
            let suffix = match (russian, pinned) {
                (true, true) => "на панели",
                (true, false) => "закрепить",
                (false, true) => "on the bar",
                (false, false) => "pin",
            };
            let pin_label = format!("{label}. {suffix}");
            let pin = menu_element(menu, &pin_label);
            assert_eq!(pin.absolute_position().x, pin_x, "pin column: {label}");
            assert_eq!(pin.absolute_position().y, pos.y + 2.0);
            assert_eq!(pin.size(), LogicalSize::new(78.0, 24.0));
            assert!(pos.x + size.width < pin.absolute_position().x);
            targets.push((pin_label, pin.absolute_position(), pin.size()));
        }
        if confirmation {
            for (button, x, width) in [
                (if russian { "Да" } else { "Yes" }, key_x, 96.0),
                (if russian { "Нет" } else { "No" }, pin_x, 78.0),
            ] {
                let item = menu_element(menu, button);
                assert_eq!(
                    item.absolute_position().x,
                    x,
                    "confirmation column: {button}"
                );
                assert_eq!(item.absolute_position().y, pos.y + 2.0);
                assert_eq!(item.size(), LogicalSize::new(width, 24.0));
                targets.push((button.to_owned(), item.absolute_position(), item.size()));
            }
        }
        if index < 2 {
            let enabled = if index == 0 {
                !menu.get_mic_muted()
            } else {
                menu.get_sys_active()
            };
            let word = match (russian, enabled) {
                (true, true) => "вкл",
                (true, false) => "выкл",
                (false, true) => "on",
                (false, false) => "off",
            };
            let chip = ElementHandle::find_by_accessible_label(menu, word)
                .find(|item| item.absolute_position().y == pos.y + 4.0)
                .expect("word chip in the sound row");
            let chip_pos = chip.absolute_position();
            let chip_size = chip.size();
            assert_eq!(chip_pos.x + chip_size.width, key_x + 96.0);
            assert!(chip_pos.x >= key_x && chip_size.width > 0.0);
            assert_eq!(chip_size.height, 20.0);
            targets.push((format!("{label}: {word}"), chip_pos, chip_size));
        }
        let key = match index {
            2 => Some("F8"),
            4 => Some("F7"),
            6 => Some("F4"),
            8 => Some("F1"),
            _ => None,
        };
        if let Some(key) = key {
            let item = menu_element(menu, key);
            let kpos = item.absolute_position();
            let ksize = item.size();
            assert!(
                kpos.x >= key_x && ksize.width > 0.0,
                "key inside column: {key}"
            );
            assert_eq!(kpos.x + ksize.width, key_x + 96.0, "key right edge: {key}");
            assert_eq!(kpos.y, pos.y + 4.0);
            assert_eq!(ksize.height, 20.0);
            assert!(kpos.x > pos.x + size.width && kpos.x + ksize.width < pin_x);
            targets.push((key.to_owned(), item.absolute_position(), item.size()));
        }
        targets.push((label.to_string(), pos, size));
    }
    assert!(previous_bottom <= menu.window().size().height as f32 - 6.0);
    for (index, (label, pos, size)) in targets.iter().enumerate() {
        assert!(
            size.width > 0.0 && size.height > 0.0,
            "empty menu target: {label}"
        );
        assert!(
            pos.x >= 6.0 && pos.x + size.width <= 402.0,
            "menu bounds: {label}"
        );
        for (other_label, other_pos, other_size) in &targets[index + 1..] {
            assert!(
                pos.x + size.width <= other_pos.x
                    || other_pos.x + other_size.width <= pos.x
                    || pos.y + size.height <= other_pos.y
                    || other_pos.y + other_size.height <= pos.y,
                "menu cells overlap: {label} and {other_label}"
            );
        }
    }
}

fn more_menu_rows_and_columns_do_not_overlap_in_english_and_russian() {
    for language in ["en", "ru"] {
        slint::select_bundled_translation(language).expect("select grid language");
        let menu = ui::BarMenuWindow::new().expect("create menu grid fixture");
        menu.set_can_restore_tile(true);
        for pinned in [false, true] {
            let ids = if pinned {
                vec![
                    "capture", "restore", "archive", "summary", "kb", "settings", "help", "tray",
                ]
            } else {
                Vec::new()
            };
            menu.set_pins(ModelRc::new(VecModel::from(
                ids.into_iter().map(SharedString::from).collect::<Vec<_>>(),
            )));
            for tray in [false, true] {
                menu.set_tray_available(tray);
                for managed in [false, true] {
                    menu.set_lock_menu_managed(managed);
                    for deep in [false, true] {
                        menu.set_deep_lock(deep);
                        let height = 408.0
                            + if tray { 28.0 } else { 0.0 }
                            + if managed { 28.0 } else { 0.0 };
                        menu.window().set_size(LogicalSize::new(408.0, height));
                        menu.show().expect("show grid fixture");
                        for confirmation in ["", "restart", "quit"] {
                            menu.set_confirmation(confirmation.into());
                            assert_menu_grid(&menu, language == "ru");
                        }
                    }
                }
            }
        }
    }
    slint::select_bundled_translation("en").expect("restore English translation");
}

fn pin_save_failure_footer_is_bilingual_temporary_and_does_not_dispatch() {
    for (language, error) in [
        ("en", "Could not save. Try again."),
        ("ru", "Не удалось сохранить. Повторите."),
    ] {
        slint::select_bundled_translation(language).expect("select menu language");
        let menu = ui::BarMenuWindow::new().expect("create menu error fixture");
        menu.set_can_restore_tile(true);
        menu.set_pins(ModelRc::new(VecModel::from(vec![SharedString::from(
            "archive",
        )])));
        let weak = menu.as_weak();
        menu.on_footer_resized(move || {
            let menu = weak.upgrade().expect("live fixture");
            menu.window()
                .set_size(LogicalSize::new(408.0, 408.0 + menu.get_footer_height()));
        });
        let actions = Rc::new(Cell::new(0));
        let count = actions.clone();
        menu.on_action(move |_| count.set(count.get() + 1));
        let weak = menu.as_weak();
        menu.on_pin_toggled(move |_| weak.upgrade().unwrap().invoke_pin_save_failed());
        menu.window().set_size(LogicalSize::new(408.0, 408.0));
        menu.show().expect("show footer fixture");
        assert_eq!(
            ElementHandle::find_by_accessible_label(&menu, error).count(),
            0
        );
        assert_eq!(menu.get_footer_height(), 0.0);
        let pin_label = if language == "ru" {
            "Настройки. закрепить"
        } else {
            "Settings. pin"
        };
        activate(&menu, pin_label);
        slint::platform::update_timers_and_animations();
        assert_eq!(
            ElementHandle::find_by_accessible_label(&menu, error).count(),
            1
        );
        assert_eq!(menu.get_footer_height(), 24.0);
        let footer = ElementHandle::find_by_accessible_label(&menu, error)
            .next()
            .unwrap();
        assert_eq!(footer.size().height, 24.0);
        assert!(footer.absolute_position().y + footer.size().height <= 432.0);
        assert_eq!(actions.get(), 0, "failure is not an action");
        assert_eq!(
            menu.get_pins().row_count(),
            1,
            "failed save keeps the old model"
        );
        assert!(menu.window().is_visible(), "save failure stays in the menu");
        let pin = ElementHandle::find_by_accessible_label(&menu, pin_label)
            .next()
            .unwrap();
        assert_eq!(pin.size().height, 24.0);
        assert_eq!(pin.size().width, 78.0);
        let quit = if language == "ru" {
            "Выйти"
        } else {
            "Quit"
        };
        let yes_label = if language == "ru" { "Да" } else { "Yes" };
        let no_label = if language == "ru" { "Нет" } else { "No" };
        activate(&menu, quit);
        assert!(ElementHandle::find_by_accessible_label(&menu, yes_label).count() > 0);
        let yes = ElementHandle::find_by_accessible_label(&menu, yes_label)
            .next()
            .unwrap();
        let no = ElementHandle::find_by_accessible_label(&menu, no_label)
            .next()
            .unwrap();
        assert_eq!(yes.size().height, 24.0);
        assert_eq!(no.size().height, 24.0);
        assert_eq!(yes.size().width, 96.0);
        assert_eq!(no.size().width, 78.0);
        assert_eq!(yes.absolute_position().y, no.absolute_position().y);
        activate(&menu, no_label);
        assert_eq!(menu.get_confirmation().as_str(), "");
        // Duration/gating/expiry bindings are guarded in bar_slop_guard.
        // Real timer expiry and native resizing remain consumer acceptance.
        assert_eq!(actions.get(), 0);
    }
    slint::select_bundled_translation("en").expect("restore English");
}

#[test]
fn bar_actions_dismiss_more_and_compact_changes_cannot_keep_it_open() {
    i_slint_backend_testing::init_no_event_loop();
    let bar = ui::OverlayBarWindow::new().expect("create bar");
    let menu = Rc::new(ui::BarMenuWindow::new().expect("create More menu"));
    bar.set_lock_a11y("Deep AI hint".into());
    bar.set_deep_lock(true);
    bar.set_tray_available(true);
    bar.set_can_restore_tile(true);
    bar.set_autotile_mode(0);
    bar.set_timer_active(true);
    bar.set_open_tiles(1);
    bar.set_pins(ModelRc::new(VecModel::from(vec![SharedString::from(
        "archive",
    )])));
    // Inert host fixture: the source guard checks production wiring to close_menu.
    // Native focus/owner/reveal behaviour is intentionally not simulated here.
    let dismissals = Rc::new(Cell::new(0));
    let dismissals_for_callback = dismissals.clone();
    let weak = bar.as_weak();
    let menu_for_dismiss = menu.as_weak();
    bar.on_menu_dismissed(move || {
        dismissals_for_callback.set(dismissals_for_callback.get() + 1);
        let menu = menu_for_dismiss.upgrade().unwrap();
        menu.set_confirmation(SharedString::default());
        menu.hide()
            .expect("hide from shared bar dismissal callback");
        weak.upgrade().unwrap().set_menu_open(false);
    });
    let opening_state = Rc::new(RefCell::new(String::from(
        "More, language=en, fixture setup",
    )));
    let state_for_open = opening_state.clone();
    let weak = bar.as_weak();
    let menu_for_open = menu.as_weak();
    bar.on_menu_opened(move |_, _| {
        let menu = menu_for_open.upgrade().unwrap();
        let bar = weak.upgrade().unwrap();
        if menu.window().is_visible() {
            bar.invoke_menu_dismissed();
        } else {
            menu.show().expect("show from more chip");
            assert!(
                menu.window().is_visible(),
                "show() returned with More hidden inside opening callback: {}",
                state_for_open.borrow()
            );
            bar.set_menu_open(true);
        }
    });
    let pinned = Rc::new(RefCell::new(Vec::<String>::new()));
    let pinned_for_callback = pinned.clone();
    let menu_for_pin = menu.as_weak();
    let state_for_pin = opening_state.clone();
    bar.on_pin_activated(move |id| {
        assert!(
            !menu_for_pin.upgrade().unwrap().window().is_visible(),
            "pin id={id} must dismiss before dispatch: {}",
            state_for_pin.borrow()
        );
        pinned_for_callback.borrow_mut().push(id.to_string());
    });

    for compact in [false, true] {
        bar.set_compact_bar(compact);
        bar.set_bar_form(i32::from(compact));
        assert_eq!(
            ElementHandle::find_by_accessible_label(&bar, "More").count(),
            1
        );
        let spec = bar.global::<ui::BarSpec>();
        let width = if compact {
            spec.get_compact_width()
        } else {
            spec.get_full_width()
        };
        bar.window()
            .set_size(LogicalSize::new(width, spec.get_height()));
        // Slint queues `changed compact-bar`; init_no_event_loop does not drain it.
        // Settle the form BEFORE opening More; a pending form dismissal must
        // not race the next action while the headless fixture has no loop.
        slint::platform::update_timers_and_animations();
        assert!(
            !menu.window().is_visible(),
            "label=More, form setup must leave menu hidden: language=en, compact={compact}"
        );
        assert!(
            !bar.get_menu_open(),
            "label=More, form setup must clear active chip: language=en, compact={compact}"
        );
        let mut labels = vec![
            "Stealth (hide from capture)",
            "Silent: no automatic tiles",
            "Deep AI hint",
            "Stop the session",
            if compact {
                "Expand the bar"
            } else {
                "Compact reader mode"
            },
        ];
        if compact {
            labels.push("Hide to tray");
        } else {
            labels.extend([
                "Mic live (click to mute)",
                "System audio capture",
                "Pause the session",
                "New tile, 1 open",
                "close all",
                "Write a question",
                "Restore closed tile",
                "Session archive",
            ]);
        }
        for (iteration, label) in labels.into_iter().enumerate() {
            let state =
                format!("label={label}, language=en, compact={compact}, iteration={iteration}");
            *opening_state.borrow_mut() = state.clone();
            assert!(
                !menu.window().is_visible(),
                "previous action left More visible: {state}"
            );
            assert!(
                !bar.get_menu_open(),
                "previous action left More chip active: {state}"
            );
            let targets: Vec<_> = ElementHandle::find_by_accessible_label(&bar, "More").collect();
            assert_eq!(
                targets.len(),
                1,
                "More must have one target before action: {state}"
            );
            let dismissals_before = dismissals.get();
            targets[0].invoke_accessible_default_action();
            assert_eq!(
                dismissals.get(),
                dismissals_before,
                "opening More must not run a pending form dismissal: {state}"
            );
            assert!(
                menu.window().is_visible(),
                "More must show before bar action: {state}"
            );
            assert!(
                bar.get_menu_open(),
                "More must activate chip before bar action: {state}"
            );
            menu.set_confirmation("quit".into());
            let targets: Vec<_> = ElementHandle::find_by_accessible_label(&bar, label).collect();
            assert_eq!(targets.len(), 1, "bar action must have one target: {state}");
            targets[0].invoke_accessible_default_action();
            assert!(
                !menu.window().is_visible(),
                "bar action must hide More: {state}"
            );
            assert!(
                !bar.get_menu_open(),
                "bar action must clear active chip: {state}"
            );
            assert_eq!(
                menu.get_confirmation().as_str(),
                "",
                "bar action must clear confirmation: {state}"
            );
        }
        // Clicking More itself toggles rather than running the generic dismiss first.
        *opening_state.borrow_mut() =
            format!("label=More toggle, language=en, compact={compact}, iteration=1");
        let more = ElementHandle::find_by_accessible_label(&bar, "More")
            .next()
            .unwrap();
        more.invoke_accessible_default_action();
        assert!(
            menu.window().is_visible(),
            "More toggle first click: language=en, compact={compact}, iteration=1"
        );
        assert!(
            bar.get_menu_open(),
            "More toggle first click activates chip: language=en, compact={compact}, iteration=1"
        );
        more.invoke_accessible_default_action();
        assert!(
            !menu.window().is_visible(),
            "More toggle second click: language=en, compact={compact}, iteration=2"
        );
        assert!(
            !bar.get_menu_open(),
            "More toggle second click clears chip: language=en, compact={compact}, iteration=2"
        );
    }
    assert_eq!(
        &*pinned.borrow(),
        &["archive"],
        "Session archive pin dispatch: language=en, full/compact action iterations complete"
    );
    for (iteration, compact) in [false, true].into_iter().enumerate() {
        let state =
            format!("label=More, language=en, target compact={compact}, iteration={iteration}");
        *opening_state.borrow_mut() = state.clone();
        ElementHandle::find_by_accessible_label(&bar, "More")
            .next()
            .unwrap()
            .invoke_accessible_default_action();
        assert_ne!(
            bar.get_compact_bar(),
            compact,
            "external form test must actually change compact-bar: {state}"
        );
        assert!(
            menu.window().is_visible(),
            "More visible before external form change: {state}"
        );
        assert!(
            bar.get_menu_open(),
            "More chip active before external form change: {state}"
        );
        menu.set_confirmation("quit".into());
        let dismissals_before = dismissals.get();
        bar.set_compact_bar(compact);
        assert_eq!(
            ElementHandle::find_by_accessible_label(&bar, "More").count(),
            1
        );
        slint::platform::update_timers_and_animations();
        assert_eq!(
            dismissals.get(),
            dismissals_before + 1,
            "external form change must run exactly one dismissal: {state}"
        );
        assert!(
            !bar.get_menu_open(),
            "property change must clear menu-open: {state}"
        );
        assert!(
            !menu.window().is_visible(),
            "external compact/full change must dismiss: {state}"
        );
        assert_eq!(
            menu.get_confirmation().as_str(),
            "",
            "external form change must clear confirmation: {state}"
        );
    }
    for form in [0, 2, 1] {
        ElementHandle::find_by_accessible_label(&bar, "More")
            .next()
            .unwrap()
            .invoke_accessible_default_action();
        assert!(menu.window().is_visible());
        assert_ne!(bar.get_bar_form(), form);
        menu.set_confirmation("quit".into());
        let dismissals_before = dismissals.get();
        bar.set_bar_form(form);
        assert_eq!(
            ElementHandle::find_by_accessible_label(&bar, "More").count(),
            1
        );
        slint::platform::update_timers_and_animations();
        assert_eq!(
            dismissals.get(),
            dismissals_before + 1,
            "effective form={form} dismissal"
        );
        assert!(!bar.get_menu_open());
        assert!(!menu.window().is_visible());
        assert_eq!(menu.get_confirmation().as_str(), "");
    }
}
