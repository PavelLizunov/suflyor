use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use std::cell::Cell;
use std::error::Error;
use std::ffi::c_void;
use std::rc::Rc;
use std::time::Duration;

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use ui::{GateOverlay, GateSettings, GateTile};

#[cfg(target_os = "macos")]
extern "C" {
    fn suflyor_gate0a_configure_overlay(view: *mut c_void);
    fn suflyor_gate0a_configure_tile(view: *mut c_void);
    fn suflyor_gate0a_configure_settings(view: *mut c_void);
    fn suflyor_gate0a_drag_window(view: *mut c_void);
    fn suflyor_gate0a_log_displays();
}

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(not(target_os = "macos"))]
    return Err("Gate 0A is a macOS-only feasibility prototype".into());

    #[cfg(target_os = "macos")]
    run_macos()
}

#[cfg(target_os = "macos")]
fn run_macos() -> Result<(), Box<dyn Error>> {
    let overlay = GateOverlay::new()?;
    let tile = GateTile::new()?;
    let settings = GateSettings::new()?;

    seed_overlay(&overlay);
    seed_tile(&tile);
    seed_settings(&settings);

    let tile_visible = Rc::new(Cell::new(true));

    overlay.on_drag_start_requested({
        let weak = overlay.as_weak();
        move || drag_component(&weak)
    });
    tile.on_drag_start_requested({
        let weak = tile.as_weak();
        move || drag_component(&weak)
    });

    overlay.on_chip_clicked({
        let weak = overlay.as_weak();
        move || {
            if let Some(window) = weak.upgrade() {
                let active = window.get_chip_active();
                window.set_state_text(if active {
                    "Chip click received - interactive overlay is live".into()
                } else {
                    "Gate 0A - external capture exclusion is unsupported".into()
                });
            }
        }
    });

    overlay.on_toggle_tile({
        let weak = tile.as_weak();
        let visible = tile_visible.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                if visible.get() {
                    let _ = window.hide();
                    visible.set(false);
                } else {
                    let _ = window.show();
                    configure_tile_component(&window);
                    visible.set(true);
                }
            }
        }
    });

    tile.on_hide_tile({
        let weak = tile.as_weak();
        let visible = tile_visible.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                let _ = window.hide();
                visible.set(false);
            }
        }
    });

    overlay.on_open_settings({
        let weak = settings.as_weak();
        move || {
            if let Some(window) = weak.upgrade() {
                let _ = window.show();
                configure_settings_component(&window);
            }
        }
    });

    settings.on_hide_settings({
        let weak = settings.as_weak();
        move || {
            if let Some(window) = weak.upgrade() {
                let _ = window.hide();
            }
        }
    });

    overlay.on_hide_overlay({
        let weak = overlay.as_weak();
        move || {
            if let Some(window) = weak.upgrade() {
                let _ = window.hide();
            }
        }
    });

    tile.show()?;

    let overlay_weak = overlay.as_weak();
    let tile_weak = tile.as_weak();
    slint::Timer::single_shot(Duration::from_millis(250), move || {
        if let Some(window) = overlay_weak.upgrade() {
            configure_overlay_component(&window);
        }
        if let Some(window) = tile_weak.upgrade() {
            configure_tile_component(&window);
        }
        unsafe { suflyor_gate0a_log_displays() };
    });

    overlay.run()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn seed_overlay(window: &GateOverlay) {
    window.set_title_text("Suflyor macOS Gate 0A".into());
    window.set_state_text("Gate 0A - external capture exclusion is unsupported".into());
    window.set_test_button_text("Test chip".into());
    window.set_tile_button_text("Tile".into());
    window.set_settings_button_text("Settings".into());
    window.set_hide_button_text("Hide".into());
}

#[cfg(target_os = "macos")]
fn seed_tile(window: &GateTile) {
    window.set_title_text("Movable test tile".into());
    window.set_body_text(
        "This isolated surface has no AI, STT, audio, persistence, or production backend.".into(),
    );
    window.set_move_hint_text("Drag the title area to verify native AppKit movement.".into());
    window.set_hide_button_text("Hide".into());
}

#[cfg(target_os = "macos")]
fn seed_settings(window: &GateSettings) {
    window.set_title_text("Gate 0A Settings focus test".into());
    window.set_explanation_text(
        "This is a normal AppKit window. Click the field and type to verify keyboard focus.".into(),
    );
    window.set_input_placeholder_text("Type a short focus test".into());
    window.set_close_button_text("Close".into());
}

#[cfg(target_os = "macos")]
fn appkit_view<C: ComponentHandle>(component: &C) -> Result<*mut c_void, Box<dyn Error>> {
    let slint_handle = component.window().window_handle();
    let raw = slint_handle.window_handle()?;
    match raw.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr()),
        other => Err(format!("expected an AppKit window handle, got {other:?}").into()),
    }
}

#[cfg(target_os = "macos")]
fn configure_overlay_component<C: ComponentHandle>(component: &C) {
    match appkit_view(component) {
        Ok(view) => unsafe { suflyor_gate0a_configure_overlay(view) },
        Err(error) => eprintln!("[gate0a] overlay configuration failed: {error}"),
    }
}

#[cfg(target_os = "macos")]
fn configure_settings_component<C: ComponentHandle>(component: &C) {
    match appkit_view(component) {
        Ok(view) => unsafe { suflyor_gate0a_configure_settings(view) },
        Err(error) => eprintln!("[gate0a] settings configuration failed: {error}"),
    }
}

#[cfg(target_os = "macos")]
fn configure_tile_component<C: ComponentHandle>(component: &C) {
    match appkit_view(component) {
        Ok(view) => unsafe { suflyor_gate0a_configure_tile(view) },
        Err(error) => eprintln!("[gate0a] tile configuration failed: {error}"),
    }
}

#[cfg(target_os = "macos")]
fn drag_component<C: ComponentHandle>(component: &slint::Weak<C>) {
    if let Some(window) = component.upgrade() {
        match appkit_view(&window) {
            Ok(view) => unsafe { suflyor_gate0a_drag_window(view) },
            Err(error) => eprintln!("[gate0a] native drag failed: {error}"),
        }
    }
}
