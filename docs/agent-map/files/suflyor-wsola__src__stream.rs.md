---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e8e426e7f8dc"
source_path: "suflyor-wsola/src/stream.rs"
batch_id: "B13"
total_lines: 105
symbols_count: 6
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "11f440a929388bbda2417b5cc53f5a679bbc7f784efdcf92f4442083350c0098"
source_matches_reconciliation_baseline: true
---

# File Map: `suflyor-wsola/src/stream.rs`

- **Batch:** B13
- **Physical Lines:** 105
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `StreamingWsola` | L14 | pub |

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L25 | `fn new(sample_rate: u32, speed: f32) -> Self` |
| function | `process` | L41 | `fn process(&mut self, fresh: &[f32]) -> Result<Vec<f32>, WsolaError>` |
| function | `finish` | L69 | `fn finish(&mut self) -> Vec<f32>` |
| function | `consecutive_chunks_and_finish_produce_finite_audio` | L82 | `fn consecutive_chunks_and_finish_produce_finite_audio() -> ()` |
| function | `empty_input_is_a_noop` | L94 | `fn empty_input_is_a_noop() -> ()` |
| function | `boundary_crossfade_keeps_the_same_duration_at_tera_sample_rate` | L101 | `fn boundary_crossfade_keeps_the_same_duration_at_tera_sample_rate() -> ()` |
