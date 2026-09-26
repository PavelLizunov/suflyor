---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_037b5dbfefd2"
source_path: "slint-experiment/src/win32.rs"
batch_id: "B04"
total_lines: 1430
symbols_count: 86
review_state: validated
---

# File Map: `slint-experiment/src/win32.rs`

- **Batch:** B04
- **Physical Lines:** 1430
- **Coverage:** 1430/1430 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `MonitorRect` | L440 | pub |
| struct | `HWND` | L1034 | pub |
| struct | `MonitorRect` | L1037 | pub |
| trait | `secondary` | L515 | private |

## Symbols & Routines (86)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `restore_arrow_after_message` | L48 | `fn restore_arrow_after_message(message: u32) -> bool` |
| function | `force_arrow_if_stealthed` | L52 | `fn force_arrow_if_stealthed(hwnd: HWND) -> bool` |
| function | `stealth_cursor_subclass_proc` | L70 | `fn stealth_cursor_subclass_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _subclass_id: usize, _ref_data: usize,) -> LRESULT` |
| function | `install_stealth_cursor_guard` | L98 | `fn install_stealth_cursor_guard(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>>` |
| function | `grab_hwnd` | L127 | `fn grab_hwnd(window: &slint::Window) -> Result<HWND, Box<dyn std::error::Error>>` |
| function | `force_hide` | L143 | `fn force_hide(window: &slint::Window) -> ()` |
| function | `make_transparent_overlay` | L164 | `fn make_transparent_overlay(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>>` |
| function | `make_transparent_tile` | L175 | `fn make_transparent_tile(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>>` |
| function | `composition_enabled` | L188 | `fn composition_enabled() -> bool` |
| function | `apply_transparency` | L196 | `fn apply_transparency(hwnd: HWND, click_through: bool,) -> Result<(), Box<dyn std::error::Error>>` |
| function | `transparency_exstyle` | L286 | `fn transparency_exstyle(before: isize, click_through: bool) -> isize` |
| function | `set_always_on_top` | L301 | `fn set_always_on_top(hwnd: HWND, on: bool) -> Result<(), Box<dyn std::error::Error>>` |
| function | `stealth_supported` | L329 | `fn stealth_supported() -> bool` |
| function | `set_stealth` | L333 | `fn set_stealth(hwnd: HWND, on: bool) -> Result<(), Box<dyn std::error::Error>>` |
| function | `read_display_affinity` | L357 | `fn read_display_affinity(hwnd: HWND) -> Result<u32, Box<dyn std::error::Error>>` |
| function | `set_skip_taskbar` | L389 | `fn set_skip_taskbar(hwnd: HWND, skip: bool) -> Result<(), Box<dyn std::error::Error>>` |
| function | `set_window_owner` | L416 | `fn set_window_owner(popup: HWND, owner: HWND) -> ()` |
| function | `skip_taskbar_exstyle` | L431 | `fn skip_taskbar_exstyle(before: isize, skip: bool) -> isize` |
| function | `width` | L450 | `fn width(&self) -> i32` |
| function | `height` | L454 | `fn height(&self) -> i32` |
| function | `is_landscape` | L458 | `fn is_landscape(&self) -> bool` |
| function | `enum_monitors` | L468 | `fn enum_monitors() -> Vec<MonitorRect>` |
| function | `callback` | L484 | `fn callback(hmonitor: HMONITOR, _hdc: HDC, _lprect: *mut RECT, _lparam: LPARAM,) -> BOOL` |
| function | `virtual_screen_bounds` | L518 | `fn virtual_screen_bounds() -> (i32, i32, i32, i32)` |
| function | `cursor_pos` | L535 | `fn cursor_pos() -> (i32, i32)` |
| function | `hide_own_windows` | L551 | `fn hide_own_windows() -> Vec<(isize, bool)>` |
| function | `cb` | L561 | `fn cb(hwnd: HWND, _l: LPARAM) -> BOOL` |
| function | `focus_window` | L618 | `fn focus_window(hwnd: HWND) -> ()` |
| function | `is_foreground_window` | L626 | `fn is_foreground_window(hwnd: HWND) -> bool` |
| function | `reveal_window` | L639 | `fn reveal_window(hwnd: HWND) -> ()` |
| function | `move_window` | L650 | `fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32,) -> Result<(), Box<dyn std::error::Error>>` |
| function | `move_window_pos_only` | L677 | `fn move_window_pos_only(hwnd: HWND, x: i32, y: i32,) -> Result<(), Box<dyn std::error::Error>>` |
| function | `get_window_rect` | L694 | `fn get_window_rect(hwnd: HWND) -> Result<(i32, i32, i32, i32), Box<dyn std::error::Error>>` |
| function | `work_area_for_window` | L709 | `fn work_area_for_window(hwnd: HWND) -> Option<MonitorRect>` |
| function | `work_area_for_point` | L737 | `fn work_area_for_point(x: i32, y: i32) -> Option<MonitorRect>` |
| function | `drag_begin` | L791 | `fn drag_begin(hwnd: HWND) -> ()` |
| function | `drag_update` | L811 | `fn drag_update(hwnd: HWND) -> ()` |
| function | `drag_end` | L830 | `fn drag_end() -> ()` |
| function | `set_round_corners` | L841 | `fn set_round_corners(hwnd: HWND) -> ()` |
| function | `pick_monitor` | L867 | `fn pick_monitor(monitors: &[MonitorRect]) -> Option<MonitorRect>` |
| function | `send_ctrl_c` | L895 | `fn send_ctrl_c() -> bool` |
| function | `read_aloud_hotkey_modifiers_released` | L934 | `fn read_aloud_hotkey_modifiers_released() -> bool` |
| function | `stealth_cursor_guard_defers_mouse_move_restore` | L949 | `fn stealth_cursor_guard_defers_mouse_move_restore() -> ()` |
| function | `presentable_stealth_requires_a_verified_exclusion` | L957 | `fn presentable_stealth_requires_a_verified_exclusion() -> ()` |
| function | `skip_taskbar_keeps_toolwindow_baseline` | L973 | `fn skip_taskbar_keeps_toolwindow_baseline() -> ()` |
| function | `transparent_windows_clear_appwindow_without_losing_other_flags` | L1008 | `fn transparent_windows_clear_appwindow_without_losing_other_flags() -> ()` |
| function | `width` | L1047 | `fn width(&self) -> i32` |
| function | `height` | L1051 | `fn height(&self) -> i32` |
| function | `is_landscape` | L1055 | `fn is_landscape(&self) -> bool` |
| function | `grab_hwnd` | L1060 | `fn grab_hwnd(window: &slint::Window) -> Result<HWND, Box<dyn std::error::Error>>` |
| function | `force_hide` | L1072 | `fn force_hide(_window: &slint::Window) -> ()` |
| function | `make_transparent_overlay` | L1074 | `fn make_transparent_overlay(_hwnd: HWND) -> Result<(), Box<dyn std::error::Error>>` |
| function | `make_transparent_tile` | L1078 | `fn make_transparent_tile(_hwnd: HWND) -> Result<(), Box<dyn std::error::Error>>` |
| function | `composition_enabled` | L1082 | `fn composition_enabled() -> bool` |
| function | `set_always_on_top` | L1086 | `fn set_always_on_top(hwnd: HWND, on: bool) -> Result<(), Box<dyn std::error::Error>>` |
| function | `stealth_supported` | L1099 | `fn stealth_supported() -> bool` |
| function | `set_stealth` | L1103 | `fn set_stealth(_hwnd: HWND, on: bool) -> Result<(), Box<dyn std::error::Error>>` |
| function | `read_display_affinity` | L1114 | `fn read_display_affinity(_hwnd: HWND) -> Result<u32, Box<dyn std::error::Error>>` |
| function | `set_skip_taskbar` | L1125 | `fn set_skip_taskbar(_hwnd: HWND, _skip: bool) -> Result<(), Box<dyn std::error::Error>>` |
| function | `set_window_owner` | L1129 | `fn set_window_owner(_popup: HWND, _owner: HWND) -> ()` |
| function | `skip_taskbar_exstyle` | L1131 | `fn skip_taskbar_exstyle(before: isize, _skip: bool) -> isize` |
| function | `enum_monitors` | L1135 | `fn enum_monitors() -> Vec<MonitorRect>` |
| function | `virtual_screen_bounds` | L1161 | `fn virtual_screen_bounds() -> (i32, i32, i32, i32)` |
| function | `cursor_pos` | L1182 | `fn cursor_pos() -> (i32, i32)` |
| function | `hide_own_windows` | L1193 | `fn hide_own_windows() -> Vec<(isize, bool)>` |
| function | `focus_window` | L1199 | `fn focus_window(hwnd: HWND) -> ()` |
| function | `is_foreground_window` | L1206 | `fn is_foreground_window(_hwnd: HWND) -> bool` |
| function | `reveal_window` | L1210 | `fn reveal_window(hwnd: HWND) -> ()` |
| function | `set_window_rect` | L1219 | `fn set_window_rect(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> ()` |
| function | `move_window` | L1226 | `fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32,) -> Result<(), Box<dyn std::error::Error>>` |
| function | `move_window_pos_only` | L1237 | `fn move_window_pos_only(hwnd: HWND, x: i32, y: i32,) -> Result<(), Box<dyn std::error::Error>>` |
| function | `get_window_rect` | L1253 | `fn get_window_rect(hwnd: HWND) -> Result<(i32, i32, i32, i32), Box<dyn std::error::Error>>` |
| function | `monitor_for_point` | L1272 | `fn monitor_for_point(monitors: &[MonitorRect], x: i32, y: i32) -> Option<MonitorRect>` |
| function | `work_area_for_window` | L1283 | `fn work_area_for_window(hwnd: HWND) -> Option<MonitorRect>` |
| function | `work_area_for_point` | L1307 | `fn work_area_for_point(x: i32, y: i32) -> Option<MonitorRect>` |
| function | `drag_begin` | L1325 | `fn drag_begin(_hwnd: HWND) -> ()` |
| function | `drag_update` | L1327 | `fn drag_update(_hwnd: HWND) -> ()` |
| function | `drag_end` | L1329 | `fn drag_end() -> ()` |
| function | `set_round_corners` | L1331 | `fn set_round_corners(_hwnd: HWND) -> ()` |
| function | `pick_monitor` | L1333 | `fn pick_monitor(monitors: &[MonitorRect]) -> Option<MonitorRect>` |
| function | `send_ctrl_c` | L1345 | `fn send_ctrl_c() -> bool` |
| function | `read_aloud_hotkey_modifiers_released` | L1356 | `fn read_aloud_hotkey_modifiers_released() -> bool` |
| function | `clipboard_read_text` | L1367 | `fn clipboard_read_text() -> Option<String>` |
| function | `clipboard_write_text` | L1378 | `fn clipboard_write_text(text: &str) -> ()` |
| function | `clipboard_clear` | L1389 | `fn clipboard_clear() -> ()` |
| function | `monitor_selection_prefers_cursor_then_primary` | L1401 | `fn monitor_selection_prefers_cursor_then_primary() -> ()` |
