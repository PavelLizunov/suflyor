---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_aad4fcb85e74"
source_path: "slint-experiment/src/native/macos/screen.rs"
batch_id: "B04"
total_lines: 267
symbols_count: 18
review_state: validated
---

# File Map: `slint-experiment/src/native/macos/screen.rs`

- **Batch:** B04
- **Physical Lines:** 267
- **Coverage:** 267/267 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `ScreenCaptureAccess` | L57 | pub |
| struct | `MacDisplayRect` | L34 | private |
| struct | `DisplayRect` | L44 | pub |

## Symbols & Routines (18)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `suflyor_macos_copy_active_displays` | L4 | `fn suflyor_macos_copy_active_displays(out_displays: *mut MacDisplayRect, capacity: usize,) -> usize` |
| function | `suflyor_macos_cursor_position` | L8 | `fn suflyor_macos_cursor_position(out_x: *mut i32, out_y: *mut i32) -> i32` |
| function | `suflyor_macos_screen_capture_preflight` | L9 | `fn suflyor_macos_screen_capture_preflight() -> i32` |
| function | `suflyor_macos_screen_capture_request` | L10 | `fn suflyor_macos_screen_capture_request() -> i32` |
| function | `suflyor_macos_capture_display_bgra` | L12 | `fn suflyor_macos_capture_display_bgra(out_width: *mut u32, out_height: *mut u32, out_display: *mut MacDisplayRect, out_bytes: *mut *mut u8, out_len: *mut usize,) -> i32` |
| function | `suflyor_macos_free_screenshot_buffer` | L20 | `fn suflyor_macos_free_screenshot_buffer(ptr: *mut u8) -> ()` |
| function | `suflyor_macos_ocr_bgra` | L22 | `fn suflyor_macos_ocr_bgra(bgra: *const u8, width: u32, height: u32, out_text: *mut *mut std::ffi::c_char,) -> i32` |
| function | `suflyor_macos_free_string` | L29 | `fn suflyor_macos_free_string(ptr: *mut std::ffi::c_char) -> ()` |
| function | `from` | L64 | `fn from(value: MacDisplayRect) -> Self` |
| function | `active_displays` | L77 | `fn active_displays() -> Vec<DisplayRect>` |
| function | `cursor_position` | L93 | `fn cursor_position() -> Option<(i32, i32)>` |
| function | `request_screen_capture_access` | L103 | `fn request_screen_capture_access() -> ScreenCaptureAccess` |
| function | `display_union` | L115 | `fn display_union(displays: &[DisplayRect]) -> Option<DisplayRect>` |
| function | `capture_rect_bgra` | L135 | `fn capture_rect_bgra(_x: i32, _y: i32, _w: i32, _h: i32,) -> Result<Vec<u8>, Box<dyn std::error::Error>>` |
| function | `capture_display_bgra_with_dimensions` | L151 | `fn capture_display_bgra_with_dimensions() -> Result<DisplayCapture, Box<dyn std::error::Error>>` |
| function | `recognize_text_from_bgra` | L196 | `fn recognize_text_from_bgra(bgra: &[u8], width: u32, height: u32,) -> Result<String, Box<dyn std::error::Error>>` |
| function | `display_union_preserves_negative_origins` | L234 | `fn display_union_preserves_negative_origins() -> ()` |
| function | `ocr_rejects_a_mismatched_bgra_buffer_before_ffi` | L264 | `fn ocr_rejects_a_mismatched_bgra_buffer_before_ffi() -> ()` |
