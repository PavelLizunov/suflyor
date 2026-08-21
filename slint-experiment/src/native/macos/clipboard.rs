//! AppKit pasteboard and selection-copy helpers for macOS.

use std::ffi::c_int;

extern "C" {
    fn suflyor_macos_clipboard_set_text(bytes: *const u8, length: usize) -> c_int;
    fn suflyor_macos_clipboard_read_text(bytes: *mut u8, capacity: usize) -> usize;
    fn suflyor_macos_clipboard_clear();
    fn suflyor_macos_copy_modifiers_released() -> c_int;
    fn suflyor_macos_send_command_c() -> c_int;
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

/// Read UTF-8 text without relying on a NUL-terminated C string.
pub fn read_text() -> Option<String> {
    let length = unsafe { suflyor_macos_clipboard_read_text(std::ptr::null_mut(), 0) };
    if length == 0 {
        return None;
    }
    let mut bytes = vec![0_u8; length];
    let copied = unsafe { suflyor_macos_clipboard_read_text(bytes.as_mut_ptr(), bytes.len()) };
    if copied != bytes.len() {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// Best-effort restore used after reading a copied selection.
pub fn write_text(text: &str) {
    let _ = set_text(text);
}

/// Clear the general pasteboard before requesting a fresh selection copy.
pub fn clear() {
    unsafe { suflyor_macos_clipboard_clear() };
}

#[must_use]
pub fn copy_modifiers_released() -> bool {
    unsafe { suflyor_macos_copy_modifiers_released() == 1 }
}

/// Synthesize Command+C only when Accessibility permission is already granted.
#[must_use]
pub fn send_command_c() -> bool {
    unsafe { suflyor_macos_send_command_c() == 1 }
}
