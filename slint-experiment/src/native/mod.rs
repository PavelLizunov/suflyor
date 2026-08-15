//! Compile-time selected native UI adapters.
//!
//! Clipboard implementations preserve four operations: empty-filtered
//! `read_text`, result-aware `set_text`, and best-effort `write_text` / `clear`.

#[cfg(windows)]
#[path = "windows/clipboard.rs"]
pub mod clipboard;
