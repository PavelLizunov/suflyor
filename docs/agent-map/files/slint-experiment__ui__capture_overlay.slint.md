---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c5a2f626bea9"
source_path: "slint-experiment/ui/capture_overlay.slint"
batch_id: "B05"
total_lines: 235
symbols_count: 16
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "eeab61406fc40b727ce99c69b41448213a3b91c8c9dd1cd4499c04431cb15819"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/capture_overlay.slint`

- **Batch:** B05
- **Physical Lines:** 235
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `CaptureOverlay` | L9 | - |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `CaptureOverlay` | L9 | `-` |
| slint_property | `frozen` | L19 | `image` |
| slint_property | `shown` | L24 | `bool` |
| slint_property | `sx` | L27 | `length` |
| slint_property | `sy` | L28 | `length` |
| slint_property | `ex` | L29 | `length` |
| slint_property | `ey` | L30 | `length` |
| slint_property | `dragging` | L31 | `bool` |
| slint_property | `translate-mode` | L36 | `bool` |
| slint_property | `practice-mode` | L39 | `bool` |
| slint_property | `rx` | L42 | `length` |
| slint_property | `ry` | L43 | `length` |
| slint_property | `rw` | L44 | `length` |
| slint_property | `rh` | L45 | `length` |
| slint_callback | `region-selected` | L48 | `-` |
| slint_callback | `cancelled` | L49 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `region-selected` declared at L48
- UI Callback `cancelled` declared at L49
