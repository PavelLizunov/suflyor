//! Shared modules for the slint-experiment crate's multiple binaries.
//!
//! Each binary (`slint-replay`, `overlay-spike`, `markdown-spike`,
//! `overlay-host`) is its own compilation unit but reuses code from
//! this library — primarily the `win32` HWND helpers, the
//! `app_state` shared-state module for the multi-window plumbing,
//! and (Phase E1) the `runtime_state` + `slint_events` modules that
//! wire the overlay-host binary to overlay-backend's ported fns.

pub mod app_state;
pub mod markdown;
pub mod runtime_state;
pub mod slint_events;
pub mod slint_session;
pub mod win32;
