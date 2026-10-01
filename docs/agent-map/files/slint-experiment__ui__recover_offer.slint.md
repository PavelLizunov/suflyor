---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_be0296d2a422"
source_path: "slint-experiment/ui/recover_offer.slint"
batch_id: "B05"
total_lines: 183
symbols_count: 9
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "14d738de63876d736c4523a65bf09ee93b947c770e47f552b1bb9cb0a1c0534c"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/recover_offer.slint`

- **Batch:** B05
- **Physical Lines:** 183
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `RecoverOfferWindow` | L25 | - |

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `RecoverOfferWindow` | L25 | `-` |
| slint_property | `last-question` | L37 | `string` |
| slint_property | `last-answer` | L38 | `string` |
| slint_property | `transcript-preview` | L41 | `string` |
| slint_property | `has-qa` | L43 | `bool` |
| slint_callback | `recover-accepted` | L47 | `-` |
| slint_callback | `dismissed` | L49 | `-` |
| slint_callback | `drag-start-requested` | L50 | `-` |
| slint_callback | `drag-moved` | L51 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `recover-accepted` declared at L47
- UI Callback `dismissed` declared at L49
- UI Callback `drag-start-requested` declared at L50
- UI Callback `drag-moved` declared at L51
