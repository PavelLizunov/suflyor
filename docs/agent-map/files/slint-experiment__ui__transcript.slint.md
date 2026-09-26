---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_5d4daaad7255"
source_path: "slint-experiment/ui/transcript.slint"
batch_id: "B05"
total_lines: 1083
symbols_count: 67
review_state: validated
---

# File Map: `slint-experiment/ui/transcript.slint`

- **Batch:** B05
- **Physical Lines:** 1083
- **Coverage:** 1083/1083 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `Tick` | L38 | - |
| slint_component | `TranscriptWindow` | L69 | - |

## Symbols & Routines (67)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `Tick` | L38 | `-` |
| ui_component | `TranscriptWindow` | L69 | `-` |
| slint_property | `checked` | L39 | `bool` |
| slint_property | `widgets-light` | L71 | `bool` |
| slint_property | `heading` | L88 | `string` |
| slint_property | `lines` | L89 | `[TranscriptLine]` |
| slint_property | `empty` | L90 | `bool` |
| slint_property | `overflow-count` | L94 | `int` |
| slint_property | `with-timecodes` | L98 | `bool` |
| slint_property | `copied` | L99 | `bool` |
| slint_property | `all-selected` | L100 | `bool` |
| slint_property | `has-audio` | L103 | `bool` |
| slint_property | `playing` | L104 | `bool` |
| slint_property | `progress` | L105 | `float` |
| slint_property | `time-text` | L106 | `string` |
| slint_property | `active-line` | L107 | `int` |
| slint_property | `speed-index` | L111 | `int` |
| slint_property | `volume` | L112 | `float` |
| slint_property | `marked-count` | L118 | `int` |
| slint_property | `mark-anchor` | L121 | `int` |
| slint_property | `capture-text` | L122 | `string` |
| slint_property | `capture-pending` | L123 | `bool` |
| slint_property | `capture-line-index` | L127 | `int` |
| slint_property | `by-voice` | L131 | `bool` |
| slint_property | `has-diarization` | L133 | `bool` |
| slint_property | `can-diarize` | L135 | `bool` |
| slint_property | `diarizing` | L137 | `bool` |
| slint_property | `needs-diar-models` | L141 | `bool` |
| slint_property | `installing-diar-models` | L143 | `bool` |
| slint_property | `use-nemotron` | L145 | `bool` |
| slint_property | `nemotron-available` | L146 | `bool` |
| slint_property | `timeline-unreliable` | L149 | `bool` |
| slint_property | `has-speaker-names` | L152 | `bool` |
| slint_property | `confirm-rediar` | L153 | `bool` |
| slint_property | `diar-status` | L155 | `string` |
| slint_property | `speaker-count` | L158 | `int` |
| slint_property | `speakers` | L160 | `[SpeakerRow]` |
| slint_property | `search-query` | L163 | `string` |
| slint_property | `search-count` | L164 | `int` |
| slint_property | `search-pos` | L165 | `int` |
| slint_property | `shift-latched` | L817 | `bool` |
| slint_callback | `clicked` | L40 | `-` |
| slint_callback | `toggle-play` | L166 | `-` |
| slint_callback | `play-line` | L168 | `-` |
| slint_callback | `seek-fraction` | L169 | `-` |
| slint_callback | `set-speed` | L172 | `-` |
| slint_callback | `set-volume` | L173 | `-` |
| slint_callback | `copy-all-requested` | L174 | `-` |
| slint_callback | `copy-selected-requested` | L175 | `-` |
| slint_callback | `toggle-line` | L176 | `-` |
| slint_callback | `toggle-all` | L177 | `-` |
| slint_callback | `close-requested` | L178 | `-` |
| slint_callback | `drag-start-requested` | L179 | `-` |
| slint_callback | `drag-moved` | L180 | `-` |
| slint_callback | `toggle-line-marked` | L184 | `-` |
| slint_callback | `save-marked` | L185 | `-` |
| slint_callback | `clear-marks` | L186 | `-` |
| slint_callback | `save-capture` | L188 | `-` |
| slint_callback | `cancel-capture` | L189 | `-` |
| slint_callback | `capture-line-selection` | L193 | `-` |
| slint_callback | `toggle-by-voice` | L197 | `-` |
| slint_callback | `run-diarization` | L198 | `-` |
| slint_callback | `install-diar-models` | L201 | `-` |
| slint_callback | `select-diar-engine` | L202 | `-` |
| slint_callback | `rename-speaker` | L203 | `-` |
| slint_callback | `search-edited` | L206 | `-` |
| slint_callback | `search-jump` | L207 | `-` |

## Key Behaviors & Concurrency

- UI Callback `clicked` declared at L40
- UI Callback `toggle-play` declared at L166
- UI Callback `play-line` declared at L168
- UI Callback `seek-fraction` declared at L169
- UI Callback `set-speed` declared at L172
- UI Callback `set-volume` declared at L173
- UI Callback `copy-all-requested` declared at L174
- UI Callback `copy-selected-requested` declared at L175
- UI Callback `toggle-line` declared at L176
- UI Callback `toggle-all` declared at L177
- UI Callback `close-requested` declared at L178
- UI Callback `drag-start-requested` declared at L179
- UI Callback `drag-moved` declared at L180
- UI Callback `toggle-line-marked` declared at L184
- UI Callback `save-marked` declared at L185
- UI Callback `clear-marks` declared at L186
- UI Callback `save-capture` declared at L188
- UI Callback `cancel-capture` declared at L189
- UI Callback `capture-line-selection` declared at L193
- UI Callback `toggle-by-voice` declared at L197
- UI Callback `run-diarization` declared at L198
- UI Callback `install-diar-models` declared at L201
- UI Callback `select-diar-engine` declared at L202
- UI Callback `rename-speaker` declared at L203
- UI Callback `search-edited` declared at L206
- UI Callback `search-jump` declared at L207
