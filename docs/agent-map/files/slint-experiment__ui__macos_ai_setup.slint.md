---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ade0a904bf80"
source_path: "slint-experiment/ui/macos_ai_setup.slint"
batch_id: "B05"
total_lines: 148
symbols_count: 8
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "594d0ab2edf5ba8edadf23f820632e4b45b2da21b320191e4f677cdaca7fe9b9"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/macos_ai_setup.slint`

- **Batch:** B05
- **Physical Lines:** 148
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `MacAiSetupWindow` | L13 | - |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `MacAiSetupWindow` | L13 | `-` |
| slint_property | `bridge-url` | L22 | `string` |
| slint_property | `token-input` | L25 | `string` |
| slint_property | `token-stored` | L27 | `bool` |
| slint_property | `status-kind` | L30 | `int` |
| slint_callback | `save-clicked` | L31 | `-` |
| slint_callback | `cancel-clicked` | L33 | `-` |
| slint_callback | `drag-start-requested` | L35 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `save-clicked` declared at L31
- UI Callback `cancel-clicked` declared at L33
- UI Callback `drag-start-requested` declared at L35
