//! Platform entry point for the production overlay host.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
include!("overlay_host_windows.rs");

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_replay::logging::init();
    let _singleton = slint_replay::native::lifecycle::acquire_singleton(0)?;
    Err("macOS overlay UI is not implemented yet".into())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("overlay-host is not supported on this platform".into())
}
