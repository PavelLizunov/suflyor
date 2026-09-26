---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_05fd21ee5cf8"
source_path: "slint-experiment/src/main.rs"
batch_id: "B01"
total_lines: 469
symbols_count: 12
review_state: validated
---

# File Map: `slint-experiment/src/main.rs`

- **Batch:** B01
- **Physical Lines:** 469
- **Coverage:** 469/469 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `PilotState` | L42 | private |
| struct | `can` | L62 | private |

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L52 | `fn new() -> Self` |
| function | `accent_for_kind` | L67 | `fn accent_for_kind(kind: &str) -> slint::Color` |
| function | `fmt_bytes` | L81 | `fn fmt_bytes(n: u64) -> String` |
| function | `fmt_modified` | L91 | `fn fmt_modified(unix: u64) -> String` |
| function | `session_label` | L102 | `fn session_label(s: &SessionInfo) -> SharedString` |
| function | `build_chips` | L113 | `fn build_chips(state: &PilotState) -> Vec<FilterChip>` |
| function | `build_visible_events` | L136 | `fn build_visible_events(state: &PilotState) -> Vec<ReplayEvent>` |
| function | `sync_window` | L169 | `fn sync_window(window: &MainWindow, state: &mut PilotState) -> ()` |
| function | `load_session_into_state` | L193 | `fn load_session_into_state(state: &mut PilotState, path: &str) -> ()` |
| function | `main` | L207 | `fn main() -> Result<(), slint::PlatformError>` |
| function | `ev` | L318 | `fn ev(kind: &str, unix_ms: u64) -> serde_json::Value` |
| function | `slint_pilot_scenarios` | L324 | `fn slint_pilot_scenarios() -> ()` |
