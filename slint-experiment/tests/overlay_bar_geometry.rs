//! Headless geometry regression for the full 1280px bar with live tiles.
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

fn assert_open_tile_geometry(bar: &ui::OverlayBarWindow, close_all_label: &str, tile_label: &str) {
    bar.set_deep_lock(true);
    bar.set_suppress_tiles(true);
    bar.set_timer_active(true);
    bar.set_timer_label("88:88".into());
    bar.set_stealth_fault(true);
    bar.set_lock_a11y("Lock mode".into());

    for count in [1, 2, 99] {
        bar.set_open_tiles(count);
        bar.window().set_size(LogicalSize::new(1600.0, 64.0));
        let natural_close_width = element(bar, close_all_label).size().width;
        let natural_tile_width = element(bar, &format!("{tile_label} ({count})"))
            .size()
            .width;

        bar.window().set_size(LogicalSize::new(1280.0, 64.0));

        let lock = element(bar, "Lock mode");
        let archive = element(bar, "Session archive");
        let close_all = element(bar, close_all_label);
        let write = element(bar, "Write a question");
        let tile = element(bar, &format!("{tile_label} ({count})"));
        let camera = element(bar, "Screenshot to vision");
        let quit = element(bar, "Quit");

        assert!(
            close_all.size().width >= natural_close_width,
            "close all was compressed: natural={natural_close_width}, actual={}",
            close_all.size().width
        );
        assert!(
            tile.size().width >= natural_tile_width,
            "tile label was compressed: natural={natural_tile_width}, actual={}",
            tile.size().width
        );

        for (name, item) in [
            ("lock", &lock),
            ("archive", &archive),
            ("close all", &close_all),
            ("write", &write),
            ("tile", &tile),
            ("camera", &camera),
            ("quit", &quit),
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

        for (left_name, left, right_name, right) in [
            ("lock", &lock, "archive", &archive),
            ("archive", &archive, "close all", &close_all),
            ("close all", &close_all, "write", &write),
            ("write", &write, "tile", &tile),
            ("tile", &tile, "camera", &camera),
            ("camera", &camera, "quit", &quit),
        ] {
            let left_pos = left.absolute_position();
            let right_pos = right.absolute_position();
            assert!(
                left_pos.x + left.size().width <= right_pos.x,
                "{left_name} overlaps {right_name}: left edge={}, left width={}, right edge={}",
                left_pos.x,
                left.size().width,
                right_pos.x
            );
        }
    }
}

fn assert_compact_geometry(bar: &ui::OverlayBarWindow) {
    bar.set_compact_bar(true);
    bar.set_open_tiles(99);
    bar.set_deep_lock(true);
    bar.set_suppress_tiles(true);
    bar.set_timer_active(true);
    bar.set_timer_label("88:88".into());
    bar.set_stealth_fault(true);
    bar.set_lock_a11y("Lock mode".into());

    bar.window().set_size(LogicalSize::new(900.0, 64.0));
    let natural_lock_width = element(bar, "Lock mode").size().width;
    bar.window().set_size(LogicalSize::new(680.0, 64.0));
    let compact_lock_width = element(bar, "Lock mode").size().width;
    assert!(
        compact_lock_width >= natural_lock_width,
        "compact lock label was compressed: natural={natural_lock_width}, actual={compact_lock_width}"
    );

    for (name, item) in [
        ("lock", element(bar, "Lock mode")),
        ("timer", element(bar, "88:88")),
        ("expand", element(bar, "Expand the bar")),
    ] {
        let pos = item.absolute_position();
        let size = item.size();
        assert!(pos.y + size.height <= 34.0, "{name} entered the status row");
        assert!(
            pos.x >= 0.0 && pos.x + size.width <= 680.0,
            "{name} escaped the compact bar: x={}, width={}",
            pos.x,
            size.width
        );
    }

    bar.set_compact_bar(false);
}

fn assert_memory_footer_geometry(bar: &ui::OverlayBarWindow, app_ram_label: &str) {
    bar.set_compact_bar(false);
    bar.set_active_stack("MODEL".into());
    bar.set_tok_per_sec("".into());
    bar.window().set_size(LogicalSize::new(1280.0, 64.0));

    for (app_memory, mlx_memory) in [
        ("", ""),
        ("123 MB", ""),
        ("", "456 MB"),
        ("123 MB", "456 MB"),
    ] {
        bar.set_app_memory(app_memory.into());
        bar.set_mlx_memory(mlx_memory.into());

        let mut items = vec![("model", element(bar, "MODEL"))];
        if !app_memory.is_empty() {
            items.push((
                "app memory",
                element(bar, &format!("{app_ram_label} {app_memory}")),
            ));
        }
        if !mlx_memory.is_empty() {
            items.push((
                "MLX memory",
                element(bar, &format!("MLX {mlx_memory}")),
            ));
        }
        for (name, item) in items {
            let pos = item.absolute_position();
            let size = item.size();
            assert!(pos.y >= 34.0, "{name} entered the action row: y={}", pos.y);
            assert!(
                pos.y + size.height <= 64.0,
                "{name} escaped the status row: y={}, height={}",
                pos.y,
                size.height
            );
        }
    }
}

fn assert_confirming_geometry(bar: &ui::OverlayBarWindow, yes_label: &str, no_label: &str) {
    bar.window().set_size(LogicalSize::new(1280.0, 64.0));
    bar.set_open_tiles(99);
    bar.set_deep_lock(true);
    bar.set_suppress_tiles(true);
    bar.set_timer_active(true);
    bar.set_timer_label("88:88".into());
    bar.set_stealth_fault(true);
    bar.set_quit_armed(true);

    for (name, item) in [
        ("confirm", element(bar, yes_label)),
        ("cancel", element(bar, no_label)),
    ] {
        let pos = item.absolute_position();
        let size = item.size();
        assert!(pos.y + size.height <= 34.0, "{name} entered the status row");
        assert!(
            pos.x >= 0.0 && pos.x + size.width <= 1280.0,
            "{name} escaped the 1280px bar: x={}, width={}",
            pos.x,
            size.width
        );
    }

    bar.set_quit_armed(false);
}

#[test]
fn open_tile_controls_stay_inside_1280_in_english_and_russian() {
    i_slint_backend_testing::init_no_event_loop();

    let english = ui::OverlayBarWindow::new().expect("create English bar");
    assert_open_tile_geometry(&english, "close all", "+ tile");
    assert_compact_geometry(&english);
    assert_memory_footer_geometry(&english, "App RAM");
    assert_confirming_geometry(&english, "Yes", "No");

    slint::select_bundled_translation("ru").expect("select Russian translation");
    let russian = ui::OverlayBarWindow::new().expect("create Russian bar");
    assert_open_tile_geometry(&russian, "закрыть все", "+ тайл");
    assert_compact_geometry(&russian);
    assert_memory_footer_geometry(&russian, "RAM приложения");
    assert_confirming_geometry(&russian, "Да", "Нет");
}

#[test]
fn open_tile_controls_keep_their_text_labels() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/overlay_bar.slint"))
            .expect("read overlay bar source");

    assert!(source.contains("label: @tr(\"close all\");"));
    assert!(source.contains("label: @tr(\"+ tile ({})\", root.open-tiles);"));
    assert!(!source.contains("@image-url(\"../assets/icons/trash.svg\")"));
    assert!(!source.contains("@image-url(\"../assets/icons/tiles.svg\")"));
}
