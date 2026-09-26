---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_77a24268cab1"
source_path: "overlay-backend/src/nemotron_diar.rs"
batch_id: "B07"
total_lines: 182
symbols_count: 5
review_state: validated
---

# File Map: `overlay-backend/src/nemotron_diar.rs`

- **Batch:** B07
- **Physical Lines:** 182
- **Coverage:** 182/182 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `ChildGuard` | L64 | private |

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `diarize` | L23 | `fn diarize(wav: &Path, duration_ms: i64) -> Result<(Vec<DiarSegment>, i64, String)>` |
| function | `drop` | L66 | `fn drop(&mut self) -> ()` |
| function | `parse_rttm` | L93 | `fn parse_rttm(reader: impl BufRead, duration_ms: i64) -> Result<(Vec<DiarSegment>, i64)>` |
| function | `keeps_overlap_and_first_arrival_ids` | L151 | `fn keeps_overlap_and_first_arrival_ids() -> ()` |
| function | `rejects_nan_bounds_and_unexpected_file` | L173 | `fn rejects_nan_bounds_and_unexpected_file() -> ()` |
