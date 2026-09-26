---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_29d8564bf478"
source_path: "slint-experiment/src/bin/overlay_host/window_lifecycle.rs"
batch_id: "B04"
total_lines: 705
symbols_count: 34
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/window_lifecycle.rs`

- **Batch:** B04
- **Physical Lines:** 705
- **Coverage:** 705/705 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `is` | L494 | private |
| struct | `WindowRegistry` | L503 | pub(crate) |
| trait | `at` | L694 | private |

## Symbols & Routines (34)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `set_platform_window_position` | L37 | `fn set_platform_window_position(window: &slint::Window, x: i32, y: i32) -> ()` |
| function | `set_global_tile_opacity` | L45 | `fn set_global_tile_opacity(value: f32) -> ()` |
| function | `global_tile_opacity` | L51 | `fn global_tile_opacity() -> f32` |
| function | `set_global_stealth` | L66 | `fn set_global_stealth(on: bool) -> ()` |
| function | `global_stealth` | L74 | `fn global_stealth() -> bool` |
| function | `set_global_stealth_effective` | L93 | `fn set_global_stealth_effective(on: bool) -> ()` |
| function | `global_stealth_effective` | L98 | `fn global_stealth_effective() -> bool` |
| function | `surface_stealth_unavailable` | L109 | `fn surface_stealth_unavailable(bar: &OverlayBarWindow) -> ()` |
| function | `apply_bar_stealth` | L122 | `fn apply_bar_stealth(bar: &OverlayBarWindow, state: &slint_replay::app_state::SharedState, on: bool,) -> bool` |
| function | `apply_stealth_one` | L176 | `fn apply_stealth_one(hwnd: slint_replay::win32::HWND, on: bool) -> ()` |
| function | `global_tile_monitor` | L205 | `fn global_tile_monitor() -> Option<(i32, i32)>` |
| function | `parse_tile_monitor_pin` | L217 | `fn parse_tile_monitor_pin(s: &str) -> Option<(i32, i32)>` |
| function | `set_global_scheme` | L232 | `fn set_global_scheme(scheme: i32) -> ()` |
| function | `global_scheme` | L237 | `fn global_scheme() -> i32` |
| function | `clamp_scheme` | L245 | `fn clamp_scheme(n: i32) -> i32` |
| function | `present_window_stealth_aware` | L265 | `fn present_window_stealth_aware(win: &W, decorate: F) -> ()` |
| function | `realize_with_retries` | L395 | `fn realize_with_retries(win: &W, attempt: Rc<dyn Fn(&W) -> bool>, fallback: Rc<dyn Fn(&W)>, ) where W: slint::ComponentHandle + 'static,` |
| function | `refresh_open_tiles` | L439 | `fn refresh_open_tiles(weak: &slint::Weak<OverlayBarWindow>, tiles: &TileWindows) -> ()` |
| function | `apply_scheme_bar` | L458 | `fn apply_scheme_bar(w: &OverlayBarWindow, scheme: i32) -> ()` |
| function | `apply_scheme_lock_menu` | L461 | `fn apply_scheme_lock_menu(w: &LockModeMenuWindow, scheme: i32) -> ()` |
| function | `apply_scheme_tile` | L464 | `fn apply_scheme_tile(w: &TileWindow, scheme: i32) -> ()` |
| function | `apply_scheme_settings` | L467 | `fn apply_scheme_settings(w: &SettingsWindow, scheme: i32) -> ()` |
| function | `apply_scheme_palette` | L470 | `fn apply_scheme_palette(w: &PaletteWindow, scheme: i32) -> ()` |
| function | `apply_scheme_text_ask` | L473 | `fn apply_scheme_text_ask(w: &TextAskWindow, scheme: i32) -> ()` |
| function | `apply_scheme_wizard` | L476 | `fn apply_scheme_wizard(w: &WizardWindow, scheme: i32) -> ()` |
| function | `apply_scheme_help` | L479 | `fn apply_scheme_help(w: &HelpWindow, scheme: i32) -> ()` |
| function | `apply_scheme_recover_offer` | L482 | `fn apply_scheme_recover_offer(w: &RecoverOfferWindow, scheme: i32) -> ()` |
| function | `apply_scheme_transcript` | L485 | `fn apply_scheme_transcript(w: &TranscriptWindow, scheme: i32) -> ()` |
| function | `apply_scheme_archive` | L488 | `fn apply_scheme_archive(w: &ArchiveWindow, scheme: i32) -> ()` |
| function | `apply_stealth` | L538 | `fn apply_stealth(&self, on: bool) -> ()` |
| function | `apply_scheme` | L623 | `fn apply_scheme(&self, scheme: i32) -> ()` |
| function | `refresh_tiles_chip` | L659 | `fn refresh_tiles_chip(&self, overlay: &OverlayBarWindow) -> ()` |
| function | `owner_monitors` | L671 | `fn owner_monitors() -> Vec<MonitorRect>` |
| function | `saved_pos_validation_covers_owner_layout` | L691 | `fn saved_pos_validation_covers_owner_layout() -> ()` |
