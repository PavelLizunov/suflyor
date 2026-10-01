---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_d3ad2723196c"
source_path: "overlay-backend/examples/audio_capture_probe.rs"
batch_id: "B09"
total_lines: 259
symbols_count: 9
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "ec8268f02da312b50055e514b24b284aaa35a6b4dabc55a1b20689c386274de8"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/examples/audio_capture_probe.rs`

- **Batch:** B09
- **Physical Lines:** 259
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SourceStats` | L95 | private |

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `probe_duration` | L42 | `fn probe_duration() -> Duration` |
| function | `ensure_microphone_permission` | L63 | `fn ensure_microphone_permission() -> ()` |
| function | `new` | L106 | `fn new() -> Self` |
| function | `observe` | L116 | `fn observe(&mut self, pcm: &[i16]) -> ()` |
| function | `report` | L126 | `fn report(&self) -> String` |
| function | `main` | L135 | `fn main() -> ()` |
| function | `main` | L230 | `fn main() -> ()` |
| function | `observe_accumulates_safe_aggregates_only` | L240 | `fn observe_accumulates_safe_aggregates_only() -> ()` |
| function | `pure_silence_keeps_negative_infinity_rms` | L253 | `fn pure_silence_keeps_negative_infinity_rms() -> ()` |
