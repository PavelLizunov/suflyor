---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_cc5e026d3800"
source_path: "experiments/macos-gate0b/src/main.rs"
batch_id: "B15"
total_lines: 281
symbols_count: 20
review_state: validated
---

# File Map: `experiments/macos-gate0b/src/main.rs`

- **Batch:** B15
- **Physical Lines:** 281
- **Coverage:** 281/281 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `NativeSnapshot` | L33 | private |

## Symbols & Routines (20)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `main` | L16 | `fn main() -> Result<(), Box<dyn Error>>` |
| function | `suflyor_gate0b_initialize` | L49 | `fn suflyor_gate0b_initialize() -> ()` |
| function | `suflyor_gate0b_refresh` | L50 | `fn suflyor_gate0b_refresh() -> ()` |
| function | `suflyor_gate0b_request_microphone` | L51 | `fn suflyor_gate0b_request_microphone() -> ()` |
| function | `suflyor_gate0b_start_microphone` | L52 | `fn suflyor_gate0b_start_microphone() -> ()` |
| function | `suflyor_gate0b_stop_microphone` | L53 | `fn suflyor_gate0b_stop_microphone() -> ()` |
| function | `suflyor_gate0b_start_system_audio` | L54 | `fn suflyor_gate0b_start_system_audio() -> ()` |
| function | `suflyor_gate0b_stop_system_audio` | L55 | `fn suflyor_gate0b_stop_system_audio() -> ()` |
| function | `suflyor_gate0b_capture_screen` | L56 | `fn suflyor_gate0b_capture_screen() -> ()` |
| function | `suflyor_gate0b_open_privacy_settings` | L57 | `fn suflyor_gate0b_open_privacy_settings(section: u32) -> ()` |
| function | `suflyor_gate0b_snapshot` | L58 | `fn suflyor_gate0b_snapshot(snapshot: *mut NativeSnapshot) -> ()` |
| function | `suflyor_gate0b_copy_message` | L59 | `fn suflyor_gate0b_copy_message(buffer: *mut c_char, capacity: usize) -> ()` |
| function | `suflyor_gate0b_shutdown` | L60 | `fn suflyor_gate0b_shutdown() -> ()` |
| function | `run` | L63 | `fn run() -> Result<(), Box<dyn std::error::Error>>` |
| function | `seed` | L144 | `fn seed(window: &GateCapture) -> ()` |
| function | `wire_actions` | L163 | `fn wire_actions(window: &GateCapture) -> ()` |
| function | `refresh_window` | L200 | `fn refresh_window(window: &GateCapture) -> ()` |
| function | `native_message` | L258 | `fn native_message(last_error: i32) -> String` |
| function | `yes_no` | L269 | `fn yes_no(value: u32) -> &'static str` |
| function | `run_macos` | L279 | `fn run_macos() -> Result<(), Box<dyn Error>>` |
