---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_22a05989fc18"
source_path: "slint-experiment/ui/archive.slint"
batch_id: "B05"
total_lines: 724
symbols_count: 38
review_state: validated
---

# File Map: `slint-experiment/ui/archive.slint`

- **Batch:** B05
- **Physical Lines:** 724
- **Coverage:** 724/724 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `RowIcon` | L38 | - |
| slint_component | `ArchiveWindow` | L65 | - |

## Symbols & Routines (38)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `RowIcon` | L38 | `-` |
| ui_component | `ArchiveWindow` | L65 | `-` |
| slint_property | `icon` | L39 | `image` |
| slint_property | `a11y` | L40 | `string` |
| slint_property | `widgets-light` | L69 | `bool` |
| slint_property | `query` | L85 | `string` |
| slint_property | `results` | L87 | `[ArchiveRow]` |
| slint_property | `selected-index` | L88 | `int` |
| slint_property | `summary` | L92 | `string` |
| slint_property | `unavailable` | L95 | `bool` |
| slint_property | `retranscribe-busy` | L100 | `bool` |
| slint_property | `retranscribe-status` | L101 | `string` |
| slint_property | `stt-is-cloud` | L105 | `bool` |
| slint_property | `renaming-index` | L110 | `int` |
| slint_property | `rename-text` | L111 | `string` |
| slint_property | `confirm-delete-index` | L133 | `int` |
| slint_property | `confirm-delete-title` | L134 | `string` |
| slint_property | `confirm-resummary-index` | L140 | `int` |
| slint_property | `confirm-resummary-title` | L141 | `string` |
| slint_callback | `clicked` | L41 | `-` |
| slint_callback | `query-changed` | L112 | `-` |
| slint_callback | `result-activated` | L114 | `-` |
| slint_callback | `retranscribe-requested` | L115 | `-` |
| slint_callback | `view-summary-requested` | L118 | `-` |
| slint_callback | `view-debrief-requested` | L120 | `-` |
| slint_callback | `rename-requested` | L124 | `-` |
| slint_callback | `rename-confirmed` | L125 | `-` |
| slint_callback | `rename-cancelled` | L126 | `-` |
| slint_callback | `regen-name-requested` | L127 | `-` |
| slint_callback | `delete-requested` | L130 | `-` |
| slint_callback | `delete-confirmed` | L135 | `-` |
| slint_callback | `delete-cancelled` | L136 | `-` |
| slint_callback | `resummary-confirmed` | L142 | `-` |
| slint_callback | `resummary-cancelled` | L143 | `-` |
| slint_callback | `transcript-requested` | L145 | `-` |
| slint_callback | `close-requested` | L146 | `-` |
| slint_callback | `drag-start-requested` | L149 | `-` |
| slint_callback | `drag-moved` | L150 | `-` |

## Key Behaviors & Concurrency

- UI Callback `clicked` declared at L41
- UI Callback `query-changed` declared at L112
- UI Callback `result-activated` declared at L114
- UI Callback `retranscribe-requested` declared at L115
- UI Callback `view-summary-requested` declared at L118
- UI Callback `view-debrief-requested` declared at L120
- UI Callback `rename-requested` declared at L124
- UI Callback `rename-confirmed` declared at L125
- UI Callback `rename-cancelled` declared at L126
- UI Callback `regen-name-requested` declared at L127
- UI Callback `delete-requested` declared at L130
- UI Callback `delete-confirmed` declared at L135
- UI Callback `delete-cancelled` declared at L136
- UI Callback `resummary-confirmed` declared at L142
- UI Callback `resummary-cancelled` declared at L143
- UI Callback `transcript-requested` declared at L145
- UI Callback `close-requested` declared at L146
- UI Callback `drag-start-requested` declared at L149
- UI Callback `drag-moved` declared at L150
