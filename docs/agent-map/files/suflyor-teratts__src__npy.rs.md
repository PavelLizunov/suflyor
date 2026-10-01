---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_426d54bada46"
source_path: "suflyor-teratts/src/npy.rs"
batch_id: "B12"
total_lines: 211
symbols_count: 10
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "0abecd0dd66841513b2cf75289af6c6fac5fef06aa09ecddacbd86291477782f"
source_matches_reconciliation_baseline: true
---

# File Map: `suflyor-teratts/src/npy.rs`

- **Batch:** B12
- **Physical Lines:** 211
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `NpyArray` | L11 | pub |

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `load_f32` | L16 | `fn load_f32(path: &Path) -> Result<NpyArray>` |
| function | `parse_f32` | L21 | `fn parse_f32(bytes: &[u8]) -> Result<NpyArray>` |
| function | `dict_string_field` | L83 | `fn dict_string_field(header: &str, key: &str) -> Result<String>` |
| function | `dict_bool_field` | L102 | `fn dict_bool_field(header: &str, key: &str) -> Result<bool>` |
| function | `dict_shape_field` | L118 | `fn dict_shape_field(header: &str) -> Result<Vec<usize>>` |
| function | `build_npy` | L153 | `fn build_npy(descr: &str, shape: &str, payload: &[u8]) -> Vec<u8>` |
| function | `parses_f4_array` | L168 | `fn parses_f4_array() -> ()` |
| function | `parses_f8_array_into_f32` | L180 | `fn parses_f8_array_into_f32() -> ()` |
| function | `rejects_wrong_dtype_and_order` | L192 | `fn rejects_wrong_dtype_and_order() -> ()` |
| function | `rejects_bad_magic` | L207 | `fn rejects_bad_magic() -> ()` |
