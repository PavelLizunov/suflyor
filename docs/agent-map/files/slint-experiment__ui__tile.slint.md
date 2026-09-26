---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ca55ef29ec2a"
source_path: "slint-experiment/ui/tile.slint"
batch_id: "B05"
total_lines: 1116
symbols_count: 79
review_state: validated
---

# File Map: `slint-experiment/ui/tile.slint`

- **Batch:** B05
- **Physical Lines:** 1116
- **Coverage:** 1116/1116 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `StarMark` | L28 | - |
| slint_component | `TileMenuRow` | L60 | - |
| slint_component | `TilePlayerButton` | L94 | - |
| slint_component | `TileWindow` | L137 | - |

## Symbols & Routines (79)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `StarMark` | L28 | `-` |
| ui_component | `TileMenuRow` | L60 | `-` |
| ui_component | `TilePlayerButton` | L94 | `-` |
| ui_component | `TileWindow` | L137 | `-` |
| slint_property | `marked` | L29 | `bool` |
| slint_property | `shift-latched` | L36 | `bool` |
| slint_property | `icon` | L61 | `image` |
| slint_property | `label` | L62 | `string` |
| slint_property | `icon` | L96 | `image` |
| slint_property | `label` | L97 | `string` |
| slint_property | `a11y` | L98 | `string` |
| slint_property | `primary` | L99 | `bool` |
| slint_property | `quiet` | L100 | `bool` |
| slint_property | `icon-color` | L101 | `color` |
| slint_property | `widgets-light` | L141 | `bool` |
| slint_property | `tile-title` | L155 | `string` |
| slint_property | `sequence` | L157 | `int` |
| slint_property | `blocks` | L158 | `[MarkdownBlock]` |
| slint_property | `source-label` | L160 | `string` |
| slint_property | `trigger-label` | L162 | `string` |
| slint_property | `trigger-color` | L163 | `color` |
| slint_property | `pinned` | L166 | `bool` |
| slint_property | `tile-maximized` | L167 | `bool` |
| slint_property | `body-opacity` | L173 | `float` |
| slint_property | `convo-id` | L177 | `int` |
| slint_property | `tile-id` | L182 | `int` |
| slint_property | `followup-busy` | L184 | `bool` |
| slint_property | `voice-recording` | L187 | `bool` |
| slint_property | `can-regenerate` | L190 | `bool` |
| slint_property | `can-escalate` | L194 | `bool` |
| slint_property | `can-copy` | L197 | `bool` |
| slint_property | `copied` | L198 | `bool` |
| slint_property | `copied-block-index` | L202 | `int` |
| slint_property | `marked-count` | L207 | `int` |
| slint_property | `mark-anchor` | L211 | `int` |
| slint_property | `capture-text` | L212 | `string` |
| slint_property | `capture-block-index` | L216 | `int` |
| slint_property | `capture-pending` | L229 | `bool` |
| slint_property | `select-mode` | L240 | `bool` |
| slint_property | `select-text` | L241 | `string` |
| slint_property | `can-retry` | L250 | `bool` |
| slint_property | `can-speak` | L254 | `bool` |
| slint_property | `speak-active` | L255 | `bool` |
| slint_property | `speak-paused` | L256 | `bool` |
| slint_property | `speak-loading` | L257 | `bool` |
| slint_property | `speak-error` | L258 | `bool` |
| slint_property | `speak-speed-label` | L260 | `string` |
| slint_property | `followup-text` | L262 | `string` |
| slint_property | `v-locked` | L1034 | `bool` |
| slint_property | `v-press-y` | L1035 | `length` |
| slint_property | `v-swallow` | L1036 | `bool` |
| slint_property | `v-rec` | L1037 | `bool` |
| slint_callback | `clicked` | L33 | `-` |
| slint_callback | `clicked` | L63 | `-` |
| slint_callback | `clicked` | L102 | `-` |
| slint_callback | `toggle-block-marked` | L220 | `-` |
| slint_callback | `save-marked` | L221 | `-` |
| slint_callback | `clear-marks` | L222 | `-` |
| slint_callback | `copy-marked` | L224 | `-` |
| slint_callback | `capture-selection` | L231 | `-` |
| slint_callback | `save-capture` | L233 | `-` |
| slint_callback | `cancel-capture` | L234 | `-` |
| slint_callback | `toggle-select-mode` | L242 | `-` |
| slint_callback | `close-clicked` | L263 | `-` |
| slint_callback | `pin-clicked` | L265 | `-` |
| slint_callback | `maximize-clicked` | L266 | `-` |
| slint_callback | `regenerate-clicked` | L268 | `-` |
| slint_callback | `escalate-clicked` | L270 | `-` |
| slint_callback | `copy-clicked` | L272 | `-` |
| slint_callback | `copy-block-clicked` | L274 | `-` |
| slint_callback | `retry-clicked` | L276 | `-` |
| slint_callback | `speak-clicked` | L278 | `-` |
| slint_callback | `speak-pause-clicked` | L279 | `-` |
| slint_callback | `speak-seek-clicked` | L280 | `-` |
| slint_callback | `speak-speed-next-clicked` | L281 | `-` |
| slint_callback | `drag-start-requested` | L286 | `-` |
| slint_callback | `drag-moved` | L287 | `-` |
| slint_callback | `followup-submitted` | L289 | `-` |
| slint_callback | `followup-voice-toggled` | L292 | `-` |

## Key Behaviors & Concurrency

- UI Callback `clicked` declared at L33
- UI Callback `clicked` declared at L63
- UI Callback `clicked` declared at L102
- UI Callback `toggle-block-marked` declared at L220
- UI Callback `save-marked` declared at L221
- UI Callback `clear-marks` declared at L222
- UI Callback `copy-marked` declared at L224
- UI Callback `capture-selection` declared at L231
- UI Callback `save-capture` declared at L233
- UI Callback `cancel-capture` declared at L234
- UI Callback `toggle-select-mode` declared at L242
- UI Callback `close-clicked` declared at L263
- UI Callback `pin-clicked` declared at L265
- UI Callback `maximize-clicked` declared at L266
- UI Callback `regenerate-clicked` declared at L268
- UI Callback `escalate-clicked` declared at L270
- UI Callback `copy-clicked` declared at L272
- UI Callback `copy-block-clicked` declared at L274
- UI Callback `retry-clicked` declared at L276
- UI Callback `speak-clicked` declared at L278
- UI Callback `speak-pause-clicked` declared at L279
- UI Callback `speak-seek-clicked` declared at L280
- UI Callback `speak-speed-next-clicked` declared at L281
- UI Callback `drag-start-requested` declared at L286
- UI Callback `drag-moved` declared at L287
- UI Callback `followup-submitted` declared at L289
- UI Callback `followup-voice-toggled` declared at L292
