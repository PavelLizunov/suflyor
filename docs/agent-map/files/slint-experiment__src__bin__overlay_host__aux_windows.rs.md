---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7250e02f5f92"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows.rs"
batch_id: "B03"
total_lines: 83
symbols_count: 3
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows.rs`

- **Batch:** B03
- **Physical Lines:** 83
- **Coverage:** 83/83 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `BusyGuard` | L71 | pub(super) |

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `lock_store` | L63 | `fn lock_store(slot: &StoreSlot) -> std::sync::MutexGuard<'_, Option<Store>>` |
| function | `drop` | L74 | `fn drop(&mut self) -> ()` |
| function | `try_acquire_busy` | L79 | `fn try_acquire_busy(busy: &AtomicBool) -> Option<BusyGuard<'_>>` |
