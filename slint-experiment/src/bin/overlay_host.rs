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

include!("overlay_host_windows.rs");

#[cfg(not(any(windows, target_os = "macos")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("overlay-host is not supported on this platform".into())
}
