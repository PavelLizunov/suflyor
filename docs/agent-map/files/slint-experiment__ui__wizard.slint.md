---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e1ae3f03b7c9"
source_path: "slint-experiment/ui/wizard.slint"
batch_id: "B05"
total_lines: 453
symbols_count: 36
review_state: validated
---

# File Map: `slint-experiment/ui/wizard.slint`

- **Batch:** B05
- **Physical Lines:** 453
- **Coverage:** 453/453 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `RailStep` | L24 | - |
| slint_component | `ModeOption` | L87 | - |
| slint_component | `WizardWindow` | L118 | - |

## Symbols & Routines (36)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `RailStep` | L24 | `-` |
| ui_component | `ModeOption` | L87 | `-` |
| ui_component | `WizardWindow` | L118 | `-` |
| slint_property | `num` | L25 | `string` |
| slint_property | `label` | L26 | `string` |
| slint_property | `state` | L27 | `int` |
| slint_property | `title` | L88 | `string` |
| slint_property | `desc` | L89 | `string` |
| slint_property | `selected` | L90 | `bool` |
| slint_property | `step` | L127 | `int` |
| slint_property | `mode` | L129 | `int` |
| slint_property | `ai-level` | L130 | `int` |
| slint_property | `stt-level` | L131 | `int` |
| slint_property | `mic-level` | L132 | `int` |
| slint_property | `sys-level` | L133 | `int` |
| slint_property | `stealth-on` | L134 | `bool` |
| slint_property | `summary-ai` | L135 | `string` |
| slint_property | `summary-stt` | L136 | `string` |
| slint_property | `summary-mic` | L137 | `string` |
| slint_property | `summary-sys` | L138 | `string` |
| slint_callback | `chosen` | L91 | `-` |
| slint_callback | `mode-selected` | L139 | `-` |
| slint_callback | `ai-test-clicked` | L141 | `-` |
| slint_callback | `stt-test-clicked` | L142 | `-` |
| slint_callback | `mic-test-clicked` | L143 | `-` |
| slint_callback | `sys-test-clicked` | L144 | `-` |
| slint_callback | `install-local-clicked` | L145 | `-` |
| slint_callback | `stealth-toggled` | L146 | `-` |
| slint_callback | `open-diagnostics` | L147 | `-` |
| slint_callback | `nav-back` | L148 | `-` |
| slint_callback | `nav-next` | L149 | `-` |
| slint_callback | `nav-skip` | L150 | `-` |
| slint_callback | `finished` | L151 | `-` |
| slint_callback | `cancelled` | L152 | `-` |
| slint_callback | `drag-start-requested` | L154 | `-` |
| slint_callback | `drag-moved` | L155 | `-` |

## Key Behaviors & Concurrency

- UI Callback `chosen` declared at L91
- UI Callback `mode-selected` declared at L139
- UI Callback `ai-test-clicked` declared at L141
- UI Callback `stt-test-clicked` declared at L142
- UI Callback `mic-test-clicked` declared at L143
- UI Callback `sys-test-clicked` declared at L144
- UI Callback `install-local-clicked` declared at L145
- UI Callback `stealth-toggled` declared at L146
- UI Callback `open-diagnostics` declared at L147
- UI Callback `nav-back` declared at L148
- UI Callback `nav-next` declared at L149
- UI Callback `nav-skip` declared at L150
- UI Callback `finished` declared at L151
- UI Callback `cancelled` declared at L152
- UI Callback `drag-start-requested` declared at L154
- UI Callback `drag-moved` declared at L155
