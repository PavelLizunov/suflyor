---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_233562d7f96b"
source_path: "slint-experiment/src/native/macos/lifecycle.rs"
batch_id: "B04"
total_lines: 67
symbols_count: 3
review_state: validated
---

# File Map: `slint-experiment/src/native/macos/lifecycle.rs`

- **Batch:** B04
- **Physical Lines:** 67
- **Coverage:** 67/67 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SingletonGuard` | L14 | pub |

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `try_acquire` | L18 | `fn try_acquire(path: &Path) -> io::Result<Option<SingletonGuard>>` |
| function | `acquire_singleton` | L32 | `fn acquire_singleton(wait_ms: u32) -> Result<SingletonGuard, Box<dyn std::error::Error>>` |
| function | `lock_is_exclusive_and_released_on_drop` | L55 | `fn lock_is_exclusive_and_released_on_drop() -> ()` |
