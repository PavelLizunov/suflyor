---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_daf89e00fa35"
source_path: "suflyor-tts/src/playback_unavailable.rs"
batch_id: "B11"
total_lines: 52
symbols_count: 8
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "f8982721ae4cbe8fa5ae6411ccd0d6ba55dd5c573c5d2b1b33ded56564fd6b58"
source_matches_reconciliation_baseline: true
---

# File Map: `suflyor-tts/src/playback_unavailable.rs`

- **Batch:** B11
- **Physical Lines:** 52
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Playback` | L8 | pub |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `feed` | L17 | `fn feed(&self, _samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L19 | `fn end_of_stream(&self) -> ()` |
| function | `pause` | L21 | `fn pause(&self) -> ()` |
| function | `resume` | L23 | `fn resume(&self) -> ()` |
| function | `seek_seconds` | L25 | `fn seek_seconds(&self, _seconds: i32) -> ()` |
| function | `set_speed` | L27 | `fn set_speed(&self, _speed: f32) -> ()` |
| function | `stop` | L29 | `fn stop(self) -> ()` |
| function | `start_fails_without_invoking_exit_callback` | L39 | `fn start_fails_without_invoking_exit_callback() -> ()` |
