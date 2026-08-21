//! Windows clipboard implementation shared by the UI host.

/// Read UTF-8 text, treating an empty clipboard as no selection.
pub fn read_text() -> Option<String> {
    clipboard_win::get_clipboard_string()
        .ok()
        .filter(|text| !text.is_empty())
}

/// Replace the clipboard text and preserve failure for result-aware callers.
pub fn set_text(text: &str) -> Result<(), String> {
    clipboard_win::set_clipboard_string(text).map_err(|error| error.to_string())
}

/// Best-effort write used when restoring a previously saved clipboard.
pub fn write_text(text: &str) {
    let _ = set_text(text);
}

/// Best-effort empty sentinel used before a synthetic copy.
pub fn clear() {
    let _ = set_text("");
}
