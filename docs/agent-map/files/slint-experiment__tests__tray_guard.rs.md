---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_fb771d5a5ee4"
source_path: "slint-experiment/tests/tray_guard.rs"
batch_id: "B05"
total_lines: 313
symbols_count: 9
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "3736ee23982432d022aaad96f0bb15b8ce11ef2d24a39416eb4b4f7f862e9704"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/tray_guard.rs`

- **Batch:** B05
- **Physical Lines:** 313
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read` | L21 | `fn read(root: &Path, rel: &str) -> String` |
| function | `hide_action_present_in_both_bar_layouts` | L26 | `fn hide_action_present_in_both_bar_layouts() -> ()` |
| function | `tray_actions_route_through_existing_bar_callbacks` | L47 | `fn tray_actions_route_through_existing_bar_callbacks() -> ()` |
| function | `startup_is_always_visible_and_hide_is_explicit_only` | L88 | `fn startup_is_always_visible_and_hide_is_explicit_only() -> ()` |
| function | `restore_keeps_compact_mode_and_icon_lifecycle_is_clean` | L132 | `fn restore_keeps_compact_mode_and_icon_lifecycle_is_clean() -> ()` |
| function | `tray_module_never_persists_state` | L187 | `fn tray_module_never_persists_state() -> ()` |
| function | `every_non_open_tray_action_closes_the_styled_menu_first` | L273 | `fn every_non_open_tray_action_closes_the_styled_menu_first() -> ()` |
| function | `themed_menu_stays_above_and_clear_of_the_taskbar` | L293 | `fn themed_menu_stays_above_and_clear_of_the_taskbar() -> ()` |
| function | `tray_icon_asset_exists` | L307 | `fn tray_icon_asset_exists() -> ()` |
