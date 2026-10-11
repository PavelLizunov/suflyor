//! Headless action/pin/confirmation regression; native focus is audited separately.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize, ModelRc, SharedString, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

mod ui {
    slint::include_modules!();
}

fn activate(menu: &ui::BarMenuWindow, label: &str) {
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
    menu.window().set_size(LogicalSize::new(408.0, 386.0));
    menu.show().expect("show menu");
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
    assert!(actions.borrow().is_empty(), "disabled restore is inert");
    assert_eq!(pins.borrow().len(), 2, "disabled restore pin is inert");
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
                "Restore last tile",
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
}
