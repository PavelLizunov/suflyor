---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_46e998ee916d"
source_path: "slint-experiment/src/native/macos/window.rs"
batch_id: "B04"
total_lines: 104
symbols_count: 13
review_state: validated
---

# File Map: `slint-experiment/src/native/macos/window.rs`

- **Batch:** B04
- **Physical Lines:** 104
- **Coverage:** 104/104 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `suflyor_macos_configure_floating_window` | L7 | `fn suflyor_macos_configure_floating_window(view: *mut c_void) -> c_int` |
| function | `suflyor_macos_begin_window_drag` | L8 | `fn suflyor_macos_begin_window_drag(view: *mut c_void) -> c_int` |
| function | `suflyor_macos_raise_window_key_front` | L9 | `fn suflyor_macos_raise_window_key_front(view: *mut c_void) -> c_int` |
| function | `suflyor_macos_get_window_rect` | L10 | `fn suflyor_macos_get_window_rect(view: *mut c_void, out_x: *mut i32, out_y: *mut i32, out_width: *mut i32, out_height: *mut i32,) -> c_int` |
| function | `appkit_view` | L19 | `fn appkit_view(window: &slint::Window) -> Result<*mut c_void, Box<dyn std::error::Error>>` |
| function | `view_from_id` | L28 | `fn view_from_id(view_id: isize) -> Result<*mut c_void, Box<dyn std::error::Error>>` |
| function | `view_id` | L37 | `fn view_id(window: &slint::Window) -> Result<isize, Box<dyn std::error::Error>>` |
| function | `configure_floating` | L42 | `fn configure_floating(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>>` |
| function | `configure_floating_by_id` | L47 | `fn configure_floating_by_id(view_id: isize) -> Result<(), Box<dyn std::error::Error>>` |
| function | `begin_drag` | L57 | `fn begin_drag(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>>` |
| function | `raise_key_front` | L68 | `fn raise_key_front(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>>` |
| function | `raise_key_front_by_id` | L73 | `fn raise_key_front_by_id(view_id: isize) -> Result<(), Box<dyn std::error::Error>>` |
| function | `window_rect_by_id` | L83 | `fn window_rect_by_id(view_id: isize,) -> Result<(i32, i32, i32, i32), Box<dyn std::error::Error>>` |
