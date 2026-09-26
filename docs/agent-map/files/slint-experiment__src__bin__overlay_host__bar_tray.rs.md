---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_bc554ea2702a"
source_path: "slint-experiment/src/bin/overlay_host/bar_tray.rs"
batch_id: "B01"
total_lines: 480
symbols_count: 15
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/bar_tray.rs`

- **Batch:** B01
- **Physical Lines:** 480
- **Coverage:** 480/480 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `sync_bar_status_visibility` | L31 | `fn sync_bar_status_visibility(visible: bool) -> ()` |
| function | `refresh_status` | L43 | `fn refresh_status(overlay: &OverlayBarWindow, mic: bool, sys: bool) -> ()` |
| function | `get_mic_active` | L53 | `fn get_mic_active(state: &slint_replay::app_state::SharedState) -> bool` |
| function | `get_sys_active` | L57 | `fn get_sys_active(state: &slint_replay::app_state::SharedState) -> bool` |
| function | `spawn_relaunch` | L74 | `fn spawn_relaunch() -> bool` |
| function | `apply_bar_size` | L118 | `fn apply_bar_size(overlay: &OverlayBarWindow, compact: bool) -> ()` |
| function | `recenter_when_sized` | L142 | `fn recenter_when_sized(weak: slint::Weak<OverlayBarWindow>, target_w_logical: f32, attempt: u32) -> ()` |
| function | `tray_menu_action` | L171 | `fn tray_menu_action(index: i32) -> Option<slint_replay::tray::TrayAction>` |
| function | `dismiss_tray_menu` | L183 | `fn dismiss_tray_menu(menu: &TrayMenuWindow, focus_armed: &RefCell<bool>) -> ()` |
| function | `dismiss_tray_menu` | L190 | `fn dismiss_tray_menu(menu: &TrayMenuWindow, focus_armed: &RefCell<bool>) -> ()` |
| function | `open_tray_menu` | L196 | `fn open_tray_menu(menu: &Rc<TrayMenuWindow>, anchor_x: i32, anchor_y: i32, state: &slint_replay::app_state::SharedState, cfg: &config::SharedConfig, focus_armed: &Rc<RefCell<bool>>,) -> ()` |
| function | `hide_bar_to_tray` | L280 | `fn hide_bar_to_tray(weak: &slint::Weak<OverlayBarWindow>) -> ()` |
| function | `restore_bar_from_tray` | L298 | `fn restore_bar_from_tray(weak: &slint::Weak<OverlayBarWindow>) -> ()` |
| function | `tray_action_dispatch` | L322 | `fn tray_action_dispatch(action: slint_replay::tray::TrayAction, weak: &slint::Weak<OverlayBarWindow>, state: &slint_replay::app_state::SharedState, cfg: &config::SharedConfig, menu: &Rc<TrayMenuWindow>, focus_armed: &Rc<RefCell<bool>>,) -> ()` |
| function | `apply_overlay_hwnd` | L371 | `fn apply_overlay_hwnd(overlay: &OverlayBarWindow, state: &slint_replay::app_state::SharedState) -> ()` |
