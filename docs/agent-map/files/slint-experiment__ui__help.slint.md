---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c928dd518547"
source_path: "slint-experiment/ui/help.slint"
batch_id: "B05"
total_lines: 370
symbols_count: 19
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "4c05bd143a5201e28c740661c36b1fa17c9488d1d815b63b4bca319fe50e68f0"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/help.slint`

- **Batch:** B05
- **Physical Lines:** 370
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `IconSlot` | L6 | - |
| slint_component | `TableHeader` | L27 | - |
| slint_component | `TableRow` | L69 | - |
| slint_component | `HelpWindow` | L134 | - |

## Symbols & Routines (19)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `IconSlot` | L6 | `-` |
| ui_component | `TableHeader` | L27 | `-` |
| ui_component | `TableRow` | L69 | `-` |
| ui_component | `HelpWindow` | L134 | `-` |
| slint_property | `icon` | L8 | `image` |
| slint_property | `tint` | L9 | `color` |
| slint_property | `glyph-size` | L10 | `length` |
| slint_property | `slot-width` | L11 | `length` |
| slint_property | `slot-height` | L12 | `length` |
| slint_property | `left` | L29 | `string` |
| slint_property | `right` | L30 | `string` |
| slint_property | `icon` | L71 | `image` |
| slint_property | `has-icon` | L72 | `bool` |
| slint_property | `glyph` | L73 | `string` |
| slint_property | `desc` | L74 | `string` |
| slint_property | `key-row` | L75 | `bool` |
| slint_callback | `cancelled` | L143 | `-` |
| slint_callback | `drag-start-requested` | L145 | `-` |
| slint_callback | `drag-moved` | L146 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `cancelled` declared at L143
- UI Callback `drag-start-requested` declared at L145
- UI Callback `drag-moved` declared at L146
