//! Platform entry point for the production overlay host.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

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

#[cfg(windows)]
include!("overlay_host_windows.rs");

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use slint::ComponentHandle;
    use std::time::Duration;

    slint_replay::logging::init();
    let _singleton = slint_replay::native::lifecycle::acquire_singleton(0)?;
    let window = ui::OverlayBarWindow::new()?;
    window.set_bootstrap_mode(true);

    window.on_quit_confirm(|| {
        let _ = slint::quit_event_loop();
    });
    window.on_drag_start_requested({
        let weak = window.as_weak();
        move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if let Err(error) = slint_replay::native::window::begin_drag(window.window()) {
                slint_replay::logging::line(&format!("[macos] native drag failed: {error}"));
            }
        }
    });

    let weak = window.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        if let Err(error) = slint_replay::native::window::configure_floating(window.window()) {
            slint_replay::logging::line(&format!(
                "[macos] floating-window configuration failed: {error}"
            ));
        }
    });

    window.run()?;
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("overlay-host is not supported on this platform".into())
}
