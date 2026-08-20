//! macOS ScreenCaptureKit screenshot and Apple Vision OCR adapter.

extern "C" {
    fn suflyor_macos_copy_active_displays(
        out_displays: *mut MacDisplayRect,
        capacity: usize,
    ) -> usize;
    fn suflyor_macos_cursor_position(out_x: *mut i32, out_y: *mut i32) -> i32;
    fn suflyor_macos_screen_capture_preflight() -> i32;
    fn suflyor_macos_screen_capture_request() -> i32;

    fn suflyor_macos_capture_display_bgra(
        out_width: *mut u32,
        out_height: *mut u32,
        out_display: *mut MacDisplayRect,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
struct MacDisplayRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    is_primary: u32,
}

/// Bounds of an active display in CoreGraphics global screen coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub is_primary: bool,
}

/// Captured BGRA bytes, pixel dimensions, and global display bounds.
pub type DisplayCapture = (Vec<u8>, u32, u32, DisplayRect);

/// Result of checking Screen Recording access after an explicit user action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScreenCaptureAccess {
    Allowed,
    RestartRequired,
    Denied,
}

impl From<MacDisplayRect> for DisplayRect {
    fn from(value: MacDisplayRect) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            is_primary: value.is_primary != 0,
        }
    }
}

/// Enumerate active displays afresh so hot-plug changes cannot leave stale geometry.
#[must_use]
pub fn active_displays() -> Vec<DisplayRect> {
    let mut capacity = unsafe { suflyor_macos_copy_active_displays(std::ptr::null_mut(), 0) };
    while capacity != 0 {
        let mut raw = vec![MacDisplayRect::default(); capacity];
        let count = unsafe { suflyor_macos_copy_active_displays(raw.as_mut_ptr(), raw.len()) };
        if count <= raw.len() {
            raw.truncate(count);
            return raw.into_iter().map(DisplayRect::from).collect();
        }
        capacity = count;
    }
    Vec::new()
}

/// Current cursor position in CoreGraphics global screen coordinates.
#[must_use]
pub fn cursor_position() -> Option<(i32, i32)> {
    let mut x = 0;
    let mut y = 0;
    (unsafe { suflyor_macos_cursor_position(&mut x, &mut y) } == 1).then_some((x, y))
}

/// Check Screen Recording access and, only because the user explicitly asked
/// for a capture, request it when needed. macOS requires a full app restart
/// after a newly accepted request.
#[must_use]
pub fn request_screen_capture_access() -> ScreenCaptureAccess {
    if unsafe { suflyor_macos_screen_capture_preflight() } == 1 {
        ScreenCaptureAccess::Allowed
    } else if unsafe { suflyor_macos_screen_capture_request() } == 1 {
        ScreenCaptureAccess::RestartRequired
    } else {
        ScreenCaptureAccess::Denied
    }
}

/// Smallest rectangle containing every active display.
#[must_use]
pub fn display_union(displays: &[DisplayRect]) -> Option<DisplayRect> {
    let first = *displays.first()?;
    Some(
        displays
            .iter()
            .skip(1)
            .fold(first, |union, display| DisplayRect {
                left: union.left.min(display.left),
                top: union.top.min(display.top),
                right: union.right.max(display.right),
                bottom: union.bottom.max(display.bottom),
                is_primary: union.is_primary || display.is_primary,
            }),
    )
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
    let (vec, _width, _height, _display) = capture_display_bgra_with_dimensions()?;
    Ok(vec)
}

/// Capture the display under the cursor, returning its BGRA pixels, pixel
/// dimensions, and global CoreGraphics bounds. The primary display is the
/// fallback when the cursor cannot be resolved.
///
/// # Errors
/// Returns an error if ScreenCaptureKit screenshot fails.
pub fn capture_display_bgra_with_dimensions() -> Result<DisplayCapture, Box<dyn std::error::Error>>
{
    let mut width = 0u32;
    let mut height = 0u32;
    let mut display = MacDisplayRect::default();
    let mut bytes_ptr = std::ptr::null_mut::<u8>();
    let mut len = 0usize;

    let status = unsafe {
        suflyor_macos_capture_display_bgra(
            &mut width,
            &mut height,
            &mut display,
            &mut bytes_ptr,
            &mut len,
        )
    };

    if status != 0 {
        return Err(format!("ScreenCaptureKit screenshot failed with code {status}").into());
    }

    if bytes_ptr.is_null() || len == 0 {
        return Err("ScreenCaptureKit returned empty buffer".into());
    }

    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    if expected != Some(len) {
        unsafe { suflyor_macos_free_screenshot_buffer(bytes_ptr) };
        return Err("ScreenCaptureKit returned an invalid buffer length".into());
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, len) };
    let vec = slice.to_vec();
    unsafe { suflyor_macos_free_screenshot_buffer(bytes_ptr) };

    Ok((vec, width, height, display.into()))
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
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("OCR image dimensions overflow")?;
    if expected == 0 || bgra.len() != expected {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::{display_union, recognize_text_from_bgra, DisplayRect};

    #[test]
    fn display_union_preserves_negative_origins() {
        let displays = [
            DisplayRect {
                left: 0,
                top: 0,
                right: 1512,
                bottom: 982,
                is_primary: true,
            },
            DisplayRect {
                left: -1080,
                top: -300,
                right: 0,
                bottom: 1620,
                is_primary: false,
            },
        ];
        assert_eq!(
            display_union(&displays),
            Some(DisplayRect {
                left: -1080,
                top: -300,
                right: 1512,
                bottom: 1620,
                is_primary: true,
            })
        );
    }

    #[test]
    fn ocr_rejects_a_mismatched_bgra_buffer_before_ffi() {
        assert!(recognize_text_from_bgra(&[0, 0, 0, 0], 2, 2).is_err());
    }
}
