---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c2ce4fae88c9"
source_path: "slint-experiment/src/native/windows/lifecycle.rs"
batch_id: "B04"
total_lines: 56
symbols_count: 2
review_state: validated
---

# File Map: `slint-experiment/src/native/windows/lifecycle.rs`

- **Batch:** B04
- **Physical Lines:** 56
- **Coverage:** 56/56 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SingletonGuard` | L10 | pub |

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `drop` | L15 | `fn drop(&mut self) -> ()` |
| function | `acquire_singleton` | L34 | `fn acquire_singleton(wait_ms: u32) -> Result<SingletonGuard, Box<dyn std::error::Error>>` |
