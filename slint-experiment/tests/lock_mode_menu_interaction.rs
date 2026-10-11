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
fn auto_tiles_chip_opens_menu_in_every_state_and_each_row_selects() {
    i_slint_backend_testing::init_no_event_loop();

    let bar = ui::OverlayBarWindow::new().expect("create bar");
    let menu = Rc::new(ui::LockModeMenuWindow::new().expect("create lock menu"));
    bar.set_lock_a11y("Deep AI hint".into());
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
    bar.on_menu_opened(move |x, y| {
        anchor_for_more.set((x, y));
    });

    for compact in [false, true] {
        bar.set_compact_bar(compact);
        bar.set_bar_form(i32::from(compact));
        let spec = bar.global::<ui::BarSpec>();
        let width = if compact {
            spec.get_compact_width()
        } else {
            spec.get_full_width()
        };
        bar.window()
            .set_size(LogicalSize::new(width, spec.get_height()));
        for mode in [0, 1, 2] {
            lock_anchor.set((-1.0, -1.0));
            bar.set_autotile_mode(mode);
            bar.set_deep_lock(mode == 2);
            let label = match mode {
                0 => "Silent: no automatic tiles",
                2 => "On every line",
                _ => "On questions",
            };
            let state = format!("label={label}, language=en, compact={compact}, mode={mode}");
            assert_eq!(
                bar.get_suppress_tiles(),
                mode == 0,
                "silent binding: {state}"
            );
            assert_eq!(
                bar.get_aggressive_active(),
                mode == 2,
                "every-line binding: {state}"
            );
            assert_eq!(
                bar.get_lock_icon_unlocked(),
                mode == 1,
                "unlock binding: {state}"
            );
            let lock_chip = ElementHandle::find_by_accessible_label(&bar, label)
                .next()
                .expect("find lock chip by accessibility label");
            lock_chip.invoke_accessible_default_action();
            assert!(
                menu.window().is_visible(),
                "auto-tiles click must show menu: {state}"
            );
            let pos = lock_chip.absolute_position();
            let size = lock_chip.size();
            assert_eq!(
                lock_anchor.get(),
                (pos.x + size.width, pos.y + size.height),
                "auto-tiles anchor callback: {state}"
            );
            menu.hide().expect("hide lock menu between states");
        }
        let more = ElementHandle::find_by_accessible_label(&bar, "More")
            .next()
            .expect("find more chip");
        more_anchor.set((-1.0, -1.0));
        more.invoke_accessible_default_action();
        let pos = more.absolute_position();
        let size = more.size();
        assert_eq!(
            more_anchor.get(),
            (pos.x + size.width, pos.y + size.height),
            "More anchor callback: language=en, compact={compact}"
        );
    }

    bar.set_autotile_mode(0);
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
    for current in [0, 1, 2] {
        menu.set_mode(current);
        for (mode, label) in [
            (0, "Silent: no automatic tiles"),
            (1, "On questions"),
            (2, "On every line"),
        ] {
            selected.set(-1);
            let state = format!("label={label}, language=en, form=auto-tiles menu, current={current}, selected mode={mode}");
            menu.show().expect("show menu before selecting");
            assert!(
                menu.window().is_visible(),
                "menu visible before selection: {state}"
            );
            let row = ElementHandle::find_by_accessible_label(menu.as_ref(), label)
                .next()
                .expect("find auto-tiles row");
            row.invoke_accessible_default_action();
            assert_eq!(
                selected.get(),
                mode,
                "row dispatch must not reuse previous selection: {state}"
            );
            assert!(
                !menu.window().is_visible(),
                "selection must dismiss menu: {state}"
            );
        }
    }
    let loaded = Rc::new(Cell::new(-1));
    let loaded_for_callback = loaded.clone();
    bar.on_local_ai_mode_selected(move |mode| loaded_for_callback.set(mode));
    for compact in [false, true] {
        bar.set_compact_bar(compact);
        bar.set_bar_form(i32::from(compact));
        bar.set_deep_lock(true);
        loaded.set(-1);
        ElementHandle::find_by_accessible_label(&bar, "Deep AI hint")
            .next()
            .expect("AI-off action")
            .invoke_accessible_default_action();
        assert_eq!(loaded.get(), 0, "AI-off indicator must dispatch unlocked lifecycle anew: label=Deep AI hint, language=en, compact={compact}");
        bar.set_deep_lock(false);
        assert_eq!(
            ElementHandle::find_by_accessible_label(&bar, "Deep AI hint").count(),
            0,
            "loaded AI removes indicator: label=Deep AI hint, language=en, compact={compact}"
        );
    }
}
