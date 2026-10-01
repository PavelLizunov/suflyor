---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_df8ee98a173c"
source_path: "slint-experiment/ui/tray_menu.slint"
batch_id: "B05"
total_lines: 105
symbols_count: 13
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "f8e554e1bffd3124a157ef551544c49b4927593d471da1a5326dd6ad1f0e5eff"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/tray_menu.slint`

- **Batch:** B05
- **Physical Lines:** 105
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `TrayMenuRow` | L3 | - |
| slint_component | `TrayMenuWindow` | L47 | - |

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `TrayMenuRow` | L3 | `-` |
| ui_component | `TrayMenuWindow` | L47 | `-` |
| slint_property | `label` | L5 | `string` |
| slint_property | `icon` | L6 | `image` |
| slint_property | `row-enabled` | L7 | `bool` |
| slint_property | `show-hide-label` | L54 | `string` |
| slint_property | `pause-resume-label` | L56 | `string` |
| slint_property | `stop-label` | L57 | `string` |
| slint_property | `quit-label` | L58 | `string` |
| slint_property | `session-running` | L59 | `bool` |
| slint_callback | `selected` | L8 | `-` |
| slint_callback | `action-selected` | L60 | `-` |
| slint_callback | `dismissed` | L61 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `selected` declared at L8
- UI Callback `action-selected` declared at L60
- UI Callback `dismissed` declared at L61
