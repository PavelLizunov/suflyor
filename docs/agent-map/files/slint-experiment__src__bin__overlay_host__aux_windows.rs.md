---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7250e02f5f92"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows.rs"
batch_id: "B03"
total_lines: 83
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "30cc101fbd7bd2b2f6ee8e5e7171bbfad686bb6beefbfc9b856aa43f55689974"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows.rs`

- **Batch:** B03
- **Physical Lines:** 83
- **Semantic coverage:** not measured; no full-line review evidence.

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
