---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_6c0d4874f031"
source_path: "slint-experiment/ui/replay.slint"
batch_id: "B05"
total_lines: 225
symbols_count: 13
review_state: validated
---

# File Map: `slint-experiment/ui/replay.slint`

- **Batch:** B05
- **Physical Lines:** 225
- **Coverage:** 225/225 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `MainWindow` | L29 | - |

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `MainWindow` | L29 | `-` |
| slint_property | `session-labels` | L41 | `[string]` |
| slint_property | `selected-session-index` | L44 | `int` |
| slint_property | `filter-chips` | L47 | `[FilterChip]` |
| slint_property | `any-hidden` | L51 | `bool` |
| slint_property | `events` | L54 | `[ReplayEvent]` |
| slint_property | `total-events` | L57 | `int` |
| slint_property | `ai-response-count` | L58 | `int` |
| slint_property | `total-cost-display` | L59 | `string` |
| slint_callback | `session-changed` | L62 | `-` |
| slint_callback | `chip-clicked` | L63 | `-` |
| slint_callback | `reset-filter` | L64 | `-` |
| slint_callback | `back-clicked` | L65 | `-` |

## Key Behaviors & Concurrency

- UI Callback `session-changed` declared at L62
- UI Callback `chip-clicked` declared at L63
- UI Callback `reset-filter` declared at L64
- UI Callback `back-clicked` declared at L65
