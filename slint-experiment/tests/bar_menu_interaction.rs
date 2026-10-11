//! Headless action/pin/confirmation regression; native focus is audited separately.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
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
    menu.window().set_size(LogicalSize::new(408.0, 386.0));
    menu.show().expect("show menu");
    let mut name_x = None;
    let mut pin_x = None;
    for label in [
        "Ask about a screenshot",
        "Auto-tiles on every line",
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
    menu.on_action(move |id| actions_for_callback.borrow_mut().push(id.to_string()));
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
        ("Auto-tiles on every line", "aggressive"),
        ("Restore closed tile", "restore"),
        ("Session archive", "archive"),
        ("Meeting summary", "summary"),
        ("Knowledge base", "kb"),
        ("Settings", "settings"),
        ("Help", "help"),
        ("Hide to tray", "tray"),
    ] {
        activate(&menu, label);
        assert_eq!(actions.borrow().last().map(String::as_str), Some(id));
    }
    menu.set_tray_available(false);
    assert_eq!(
        ElementHandle::find_by_accessible_label(&menu, "Hide to tray").count(),
        0
    );
    for (label, id) in [("Restart", "restart"), ("Quit", "quit")] {
        let before = actions.borrow().len();
        activate(&menu, label);
        assert_eq!(menu.get_confirmation().as_str(), id);
        assert_eq!(
            actions.borrow().len(),
            before,
            "first click must not dispatch {id}"
        );
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
    }
}
