---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_de95fa0cd432"
source_path: "slint-experiment/ui/lock_mode_menu.slint"
batch_id: "B05"
total_lines: 58
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "4c7865b5a76fe83839f96eda3681087256c12b530884472ea46ddcc569a71ea3"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/lock_mode_menu.slint`

- **Batch:** B05
- **Physical Lines:** 58
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `LockModeMenuWindow` | L7 | - |

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `LockModeMenuWindow` | L7 | `-` |
| slint_property | `managed` | L14 | `bool` |
| slint_property | `mode` | L15 | `int` |
| slint_callback | `mode-selected` | L16 | `-` |
| slint_callback | `dismissed` | L17 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `mode-selected` declared at L16
- UI Callback `dismissed` declared at L17
