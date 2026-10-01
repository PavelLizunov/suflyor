---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c2ce4fae88c9"
source_path: "slint-experiment/src/native/windows/lifecycle.rs"
batch_id: "B04"
total_lines: 56
symbols_count: 2
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "cf3af28540aadec179db9d6969750aa29e355773d8808f697cddc0e56dda9abe"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/native/windows/lifecycle.rs`

- **Batch:** B04
- **Physical Lines:** 56
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SingletonGuard` | L10 | pub |

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `drop` | L15 | `fn drop(&mut self) -> ()` |
| function | `acquire_singleton` | L34 | `fn acquire_singleton(wait_ms: u32) -> Result<SingletonGuard, Box<dyn std::error::Error>>` |
