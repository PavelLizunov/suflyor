---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8a33c4be728e"
source_path: "slint-experiment/src/tray.rs"
batch_id: "B01"
total_lines: 656
symbols_count: 23
review_state: validated
---

# File Map: `slint-experiment/src/tray.rs`

- **Batch:** B01
- **Physical Lines:** 656
- **Coverage:** 656/656 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `TrayAction` | L50 | pub |
| struct | `TraySnapshot` | L78 | pub |
| struct | `TrayMenuEntry` | L99 | pub |
| struct | `TrayCtx` | L194 | private |
| struct | `TrayHandle` | L202 | pub |

## Symbols & Routines (23)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `from_menu_id` | L65 | `fn from_menu_id(id: u32) -> Option<Self>` |
| function | `startup` | L88 | `fn startup() -> Self` |
| function | `menu_entries` | L109 | `fn menu_entries(snap: &TraySnapshot, ru: bool) -> Vec<TrayMenuEntry>` |
| function | `claim_install_slot` | L182 | `fn claim_install_slot(slot: &AtomicBool) -> Result<(), String>` |
| function | `drop` | L207 | `fn drop(&mut self) -> ()` |
| function | `install_win32` | L256 | `fn install_win32() -> Result<TrayHandle, String>` |
| function | `show_icon` | L310 | `fn show_icon() -> Result<(), String>` |
| function | `hide_icon` | L337 | `fn hide_icon() -> ()` |
| function | `return_focus` | L361 | `fn return_focus() -> ()` |
| function | `add_notify_icon` | L386 | `fn add_notify_icon(hwnd: HWND, module: windows::Win32::Foundation::HMODULE,) -> Result<(), String>` |
| function | `load_tray_icon` | L421 | `fn load_tray_icon(module: windows::Win32::Foundation::HMODULE) -> HICON` |
| function | `fill_tip` | L430 | `fn fill_tip(dst: &mut [u16; 128], text: &str) -> ()` |
| function | `tray_wndproc` | L437 | `fn tray_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,) -> LRESULT` |
| function | `dispatch_from_ctx` | L490 | `fn dispatch_from_ctx(action: TrayAction) -> ()` |
| function | `publish_availability` | L500 | `fn publish_availability(available: bool) -> ()` |
| function | `request_tray_menu` | L510 | `fn request_tray_menu() -> ()` |
| function | `arm_menu_request` | L525 | `fn arm_menu_request(pending: &AtomicBool) -> bool` |
| function | `startup_snapshot_is_visible_and_idle` | L536 | `fn startup_snapshot_is_visible_and_idle() -> ()` |
| function | `duplicate_context_events_arm_only_one_menu` | L544 | `fn duplicate_context_events_arm_only_one_menu() -> ()` |
| function | `menu_ids_are_unique_and_round_trip` | L553 | `fn menu_ids_are_unique_and_round_trip() -> ()` |
| function | `menu_routing_reflects_state` | L578 | `fn menu_routing_reflects_state() -> ()` |
| function | `menu_labels_follow_language` | L616 | `fn menu_labels_follow_language() -> ()` |
| function | `install_slot_rejects_duplicates_and_releases` | L642 | `fn install_slot_rejects_duplicates_and_releases() -> ()` |
