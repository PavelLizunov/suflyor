---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_486c3d8cfe9e"
source_path: "slint-experiment/ui/palette.slint"
batch_id: "B05"
total_lines: 268
symbols_count: 12
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "bc29207e78e04a8ebc7cc7c4d6d1c15ecfb5814c84a949b90dfd8c473c475c69"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/palette.slint`

- **Batch:** B05
- **Physical Lines:** 268
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `PaletteWindow` | L21 | - |

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `PaletteWindow` | L21 | `-` |
| slint_property | `widgets-light` | L25 | `bool` |
| slint_property | `query` | L41 | `string` |
| slint_property | `recent-chips` | L48 | `[string]` |
| slint_property | `results` | L49 | `[PaletteResult]` |
| slint_property | `selected-index` | L50 | `int` |
| slint_callback | `query-changed` | L51 | `-` |
| slint_callback | `chip-clicked` | L53 | `-` |
| slint_callback | `result-activated` | L54 | `-` |
| slint_callback | `close-requested` | L55 | `-` |
| slint_callback | `drag-start-requested` | L56 | `-` |
| slint_callback | `drag-moved` | L57 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `query-changed` declared at L51
- UI Callback `chip-clicked` declared at L53
- UI Callback `result-activated` declared at L54
- UI Callback `close-requested` declared at L55
- UI Callback `drag-start-requested` declared at L56
- UI Callback `drag-moved` declared at L57
