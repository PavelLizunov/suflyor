---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_dd657258773b"
source_path: "slint-experiment/src/runtime_state.rs"
batch_id: "B01"
total_lines: 340
symbols_count: 8
review_state: validated
---

# File Map: `slint-experiment/src/runtime_state.rs`

- **Batch:** B01
- **Physical Lines:** 340
- **Coverage:** 340/340 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SlintRuntime` | L35 | pub |
| struct | `a` | L194 | private |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `shared_runtime` | L196 | `fn shared_runtime() -> SharedSlintRuntime` |
| function | `lock` | L205 | `fn lock(rt: &SharedSlintRuntime) -> std::sync::MutexGuard<'_, SlintRuntime>` |
| function | `push_transcript_line` | L228 | `fn push_transcript_line(rt: &mut SlintRuntime, line: TranscriptLine) -> ()` |
| function | `cost_cap_reason` | L243 | `fn cost_cap_reason(cap_usd: f64, current_microcents: u64) -> Option<String>` |
| function | `shared_runtime_starts_empty` | L269 | `fn shared_runtime_starts_empty() -> ()` |
| function | `push_transcript_line_caps_at_max` | L282 | `fn push_transcript_line_caps_at_max() -> ()` |
| function | `full_transcript_caps_at_max_and_flags_truncation` | L311 | `fn full_transcript_caps_at_max_and_flags_truncation() -> ()` |
| function | `cost_cap_reason_covers_boundaries_and_ui_text` | L332 | `fn cost_cap_reason_covers_boundaries_and_ui_text() -> ()` |
