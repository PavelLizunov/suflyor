---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7f8a5c712d11"
source_path: "slint-experiment/src/bin/overlay_host/capture_watchdog.rs"
batch_id: "B01"
total_lines: 155
symbols_count: 9
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/capture_watchdog.rs`

- **Batch:** B01
- **Physical Lines:** 155
- **Coverage:** 155/155 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Decision` | L8 | pub(super) |
| struct | `StreamWatch` | L14 | private |
| struct | `CaptureWatchdog` | L47 | pub(super) |

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `observe` | L21 | `fn observe(&mut self, emitted: u64) -> ()` |
| function | `stagnant` | L37 | `fn stagnant(self) -> bool` |
| function | `request_stop` | L83 | `fn request_stop(&mut self) -> Decision` |
| function | `reset` | L88 | `fn reset(&mut self) -> ()` |
| function | `flowing_stream_then_stall_requests_one_stop` | L111 | `fn flowing_stream_then_stall_requests_one_stop() -> ()` |
| function | `disappeared_flowing_capture_requests_one_stop` | L119 | `fn disappeared_flowing_capture_requests_one_stop() -> ()` |
| function | `never_flowed_streams_do_not_stop` | L127 | `fn never_flowed_streams_do_not_stop() -> ()` |
| function | `progress_clears_a_partial_stall` | L136 | `fn progress_clears_a_partial_stall() -> ()` |
| function | `intentional_stop_rearms_the_next_session` | L147 | `fn intentional_stop_rearms_the_next_session() -> ()` |
