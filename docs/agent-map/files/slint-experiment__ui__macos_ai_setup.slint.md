---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ade0a904bf80"
source_path: "slint-experiment/ui/macos_ai_setup.slint"
batch_id: "B05"
total_lines: 148
symbols_count: 8
review_state: validated
---

# File Map: `slint-experiment/ui/macos_ai_setup.slint`

- **Batch:** B05
- **Physical Lines:** 148
- **Coverage:** 148/148 lines (100%)

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

## Key Behaviors & Concurrency

- UI Callback `save-clicked` declared at L31
- UI Callback `cancel-clicked` declared at L33
- UI Callback `drag-start-requested` declared at L35
