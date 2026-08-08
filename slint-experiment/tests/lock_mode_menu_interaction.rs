//! Headless interaction regression for the lock chip and its native menu.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use slint::ComponentHandle;
use std::cell::Cell;
use std::rc::Rc;

mod ui {
    slint::include_modules!();
}

#[test]
fn lock_chip_opens_visible_menu_and_unlocked_row_selects() {
    i_slint_backend_testing::init_no_event_loop();

    let bar = ui::OverlayBarWindow::new().expect("create bar");
    let menu = Rc::new(ui::LockModeMenuWindow::new().expect("create lock menu"));
    bar.set_lock_a11y("Lock mode".into());
    assert!(bar.get_lock_icon_unlocked());

    let menu_for_open = menu.clone();
    bar.on_lock_menu_opened(move |_, _| {
        menu_for_open.show().expect("show lock menu");
    });

    let lock_chip = ElementHandle::find_by_accessible_label(&bar, "Lock mode")
        .next()
        .expect("find lock chip by accessibility label");
    lock_chip.invoke_accessible_default_action();
    assert!(menu.window().is_visible(), "lock click must show the menu");

    bar.set_suppress_tiles(true);
    assert!(
        !bar.get_lock_icon_unlocked(),
        "the runtime property driving the image must switch away from unlock.svg"
    );

    let selected = Rc::new(Cell::new(-1));
    let selected_for_callback = selected.clone();
    let menu_for_callback = menu.clone();
    menu.on_mode_selected(move |mode| {
        selected_for_callback.set(mode);
        menu_for_callback.hide().expect("hide lock menu");
    });
    let unlocked = ElementHandle::find_by_accessible_label(menu.as_ref(), "Unlocked")
        .next()
        .expect("find unlocked menu row");
    unlocked.invoke_accessible_default_action();

    assert_eq!(selected.get(), 0);
    assert!(
        !menu.window().is_visible(),
        "selection must dismiss the menu"
    );
}
