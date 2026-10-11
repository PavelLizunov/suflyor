//! Headless interaction regression for the lock chip and its native menu.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize};
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
    let lock_anchor = Rc::new(Cell::new((0.0_f32, 0.0_f32)));
    let anchor_for_open = lock_anchor.clone();
    bar.on_lock_menu_opened(move |x, y| {
        anchor_for_open.set((x, y));
        menu_for_open.show().expect("show lock menu");
    });
    let more_anchor = Rc::new(Cell::new((0.0_f32, 0.0_f32)));
    let anchor_for_more = more_anchor.clone();
    bar.on_menu_opened(move |x, y| { anchor_for_more.set((x, y)); });

    for compact in [false, true] {
        bar.set_compact_bar(compact);
        let spec = bar.global::<ui::BarSpec>();
        let width = if compact { spec.get_compact_width() } else { spec.get_full_width() };
        bar.window().set_size(LogicalSize::new(width, spec.get_height()));
        for mode in [0, 1, 2] {
            bar.set_suppress_tiles(mode != 0);
            bar.set_deep_lock(mode == 2);
            let lock_chip = ElementHandle::find_by_accessible_label(&bar, "Lock mode")
                .next().expect("find lock chip by accessibility label");
            lock_chip.invoke_accessible_default_action();
            assert!(menu.window().is_visible(), "lock click must show the menu");
            let pos = lock_chip.absolute_position();
            let size = lock_chip.size();
            assert_eq!(lock_anchor.get(), (pos.x + size.width, pos.y + size.height));
            menu.hide().expect("hide lock menu between states");
        }
        let more = ElementHandle::find_by_accessible_label(&bar, "More")
            .next().expect("find more chip");
        more.invoke_accessible_default_action();
        let pos = more.absolute_position();
        let size = more.size();
        assert_eq!(more_anchor.get(), (pos.x + size.width, pos.y + size.height));
    }

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
    menu.show().expect("show menu before selecting unlocked");
    assert!(menu.window().is_visible());
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
