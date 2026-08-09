//! Headless geometry regression for the full 1280px bar with live tiles.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize};

mod ui {
    slint::include_modules!();
}

fn element(bar: &ui::OverlayBarWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(bar, label)
        .next()
        .unwrap_or_else(|| panic!("find element with accessible label: {label}"))
}

fn assert_open_tile_geometry(bar: &ui::OverlayBarWindow, close_all_label: &str, tile_label: &str) {
    bar.window().set_size(LogicalSize::new(1280.0, 64.0));
    bar.set_deep_lock(true);
    bar.set_suppress_tiles(true);
    bar.set_timer_active(true);
    bar.set_timer_label("88:88".into());
    bar.set_stealth_fault(true);
    bar.set_lock_a11y("Lock mode".into());

    for count in [1, 2, 99] {
        bar.set_open_tiles(count);

        let lock = element(bar, "Lock mode");
        let archive = element(bar, "Session archive");
        let close_all = element(bar, close_all_label);
        let write = element(bar, "Write a question");
        let tile = element(bar, &format!("{tile_label} ({count})"));
        let camera = element(bar, "Screenshot to vision");
        let quit = element(bar, "Quit");

        for (name, item) in [
            ("lock", lock),
            ("archive", archive),
            ("close all", close_all),
            ("write", write),
            ("tile", tile),
            ("camera", camera),
            ("quit", quit),
        ] {
            let pos = item.absolute_position();
            let size = item.size();
            assert!(
                pos.y + size.height <= 34.0,
                "{name} dropped into the status row: y={}, height={}",
                pos.y,
                size.height
            );
            assert!(
                pos.x >= 0.0 && pos.x + size.width <= 1280.0,
                "{name} escaped the 1280px bar: x={}, width={}",
                pos.x,
                size.width
            );
        }
    }
}

#[test]
fn open_tile_controls_stay_inside_1280_in_english_and_russian() {
    i_slint_backend_testing::init_no_event_loop();

    let english = ui::OverlayBarWindow::new().expect("create English bar");
    assert_open_tile_geometry(&english, "close all", "+ tile");

    slint::select_bundled_translation("ru").expect("select Russian translation");
    let russian = ui::OverlayBarWindow::new().expect("create Russian bar");
    assert_open_tile_geometry(&russian, "закрыть все", "+ тайл");
}
