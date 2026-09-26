---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_6f433611bf27"
source_path: "suflyor-teratts/src/rng.rs"
batch_id: "B12"
total_lines: 94
symbols_count: 8
review_state: validated
---

# File Map: `suflyor-teratts/src/rng.rs`

- **Batch:** B12
- **Physical Lines:** 94
- **Coverage:** 94/94 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Rng` | L8 | pub |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L14 | `fn new(seed: u64) -> Self` |
| function | `next_u64` | L21 | `fn next_u64(&mut self) -> u64` |
| function | `next_f64` | L30 | `fn next_f64(&mut self) -> f64` |
| function | `next_normal_f32` | L35 | `fn next_normal_f32(&mut self) -> f32` |
| function | `fill_normal_f32` | L52 | `fn fill_normal_f32(&mut self, out: &mut [f32]) -> ()` |
| function | `same_seed_same_sequence` | L66 | `fn same_seed_same_sequence() -> ()` |
| function | `different_seeds_differ` | L75 | `fn different_seeds_differ() -> ()` |
| function | `distribution_is_sane` | L84 | `fn distribution_is_sane() -> ()` |
