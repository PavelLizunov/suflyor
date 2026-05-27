//! Shared modules for the slint-experiment crate's multiple binaries.
//!
//! Each binary (`slint-replay`, `overlay-spike`, `markdown-spike`,
//! `overlay-host`) is its own compilation unit but reuses code from
//! this library — primarily the `win32` HWND helpers and the
//! `app_state` shared-state module for the multi-window plumbing.

pub mod app_state;
pub mod win32;
