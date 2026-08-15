//! Compile-time selected native UI and capture adapters.
//!
//! Clipboard implementations preserve four operations: empty-filtered
//! `read_text`, result-aware `set_text`, and best-effort `write_text` / `clear`.

#[cfg(windows)]
#[path = "windows/clipboard.rs"]
pub mod clipboard;

#[cfg(windows)]
#[path = "windows/screen.rs"]
pub mod screen;
