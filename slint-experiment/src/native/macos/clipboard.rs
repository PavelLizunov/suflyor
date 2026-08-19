//! AppKit pasteboard text write for the macOS copy affordances.
//!
//! Only the result-aware `set_text` exists here — the overlay copies answers
//! and code blocks locally; it never reads or clears the pasteboard.

use std::ffi::c_int;

extern "C" {
    fn suflyor_macos_clipboard_set_text(bytes: *const u8, length: usize) -> c_int;
}

/// Replace the general pasteboard's text. The bytes + explicit length reach
/// AppKit as one NSString, so an embedded NUL cannot truncate the payload.
pub fn set_text(text: &str) -> Result<(), String> {
    let written = unsafe { suflyor_macos_clipboard_set_text(text.as_ptr(), text.len()) };
    if written == 1 {
        Ok(())
    } else {
        // Category only — the payload must never surface in an error string.
        Err("AppKit pasteboard write failed".into())
    }
}
