//! macOS ScreenCaptureKit screenshot and Apple Vision OCR adapter.

extern "C" {
    fn suflyor_macos_capture_display_bgra(
        out_width: *mut u32,
        out_height: *mut u32,
        out_bytes: *mut *mut u8,
        out_len: *mut usize,
    ) -> i32;

    fn suflyor_macos_free_screenshot_buffer(ptr: *mut u8);

    fn suflyor_macos_ocr_bgra(
        bgra: *const u8,
        width: u32,
        height: u32,
        out_text: *mut *mut std::ffi::c_char,
    ) -> i32;

    fn suflyor_macos_free_string(ptr: *mut std::ffi::c_char);
}

/// Capture full display as TOP-DOWN BGRA bytes.
///
/// # Errors
/// Returns an error if ScreenCaptureKit fails to capture or allocate memory.
pub fn capture_rect_bgra(
    _x: i32,
    _y: i32,
    _w: i32,
    _h: i32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (vec, _width, _height) = capture_display_bgra_with_dimensions()?;
    Ok(vec)
}

/// Internal helper returning (BGRA_bytes, width, height).
///
/// # Errors
/// Returns an error if ScreenCaptureKit screenshot fails.
pub fn capture_display_bgra_with_dimensions(
) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bytes_ptr = std::ptr::null_mut::<u8>();
    let mut len = 0usize;

    let status = unsafe {
        suflyor_macos_capture_display_bgra(&mut width, &mut height, &mut bytes_ptr, &mut len)
    };

    if status != 0 {
        return Err(format!("ScreenCaptureKit screenshot failed with code {status}").into());
    }

    if bytes_ptr.is_null() || len == 0 {
        return Err("ScreenCaptureKit returned empty buffer".into());
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
    let vec = slice.to_vec();
    unsafe { suflyor_macos_free_screenshot_buffer(bytes_ptr) };

    Ok((vec, width, height))
}

/// Perform Apple Vision OCR on a TOP-DOWN BGRA buffer.
///
/// # Errors
/// Returns an error if Vision OCR fails or returns invalid text.
pub fn recognize_text_from_bgra(
    bgra: &[u8],
    width: u32,
    height: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    if bgra.is_empty() || width == 0 || height == 0 {
        return Err("invalid image input for OCR".into());
    }

    let mut out_text_ptr = std::ptr::null_mut::<std::ffi::c_char>();
    let status = unsafe { suflyor_macos_ocr_bgra(bgra.as_ptr(), width, height, &mut out_text_ptr) };

    if status != 0 {
        return Err(format!("Apple Vision OCR failed with code {status}").into());
    }

    if out_text_ptr.is_null() {
        return Ok(String::new());
    }

    let c_str = unsafe { std::ffi::CStr::from_ptr(out_text_ptr) };
    let string = c_str.to_string_lossy().into_owned();
    unsafe { suflyor_macos_free_string(out_text_ptr) };

    Ok(string)
}
