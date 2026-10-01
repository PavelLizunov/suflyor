---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_46c13bb943cc"
source_path: "slint-experiment/src/native/macos/status.rs"
batch_id: "B04"
total_lines: 56
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "c1a258d36b6f93b56395e0f5e8ea61c354fb2f030a899cf8db1751a4bab3591e"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/native/macos/status.rs`

- **Batch:** B04
- **Physical Lines:** 56
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `StatusItemGuard` | L31 | pub |

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `suflyor_macos_status_remove` | L14 | `fn suflyor_macos_status_remove() -> ()` |
| function | `quit_through_event_loop` | L17 | `fn quit_through_event_loop() -> ()` |
| function | `appkit_view` | L21 | `fn appkit_view(window: &slint::Window) -> Result<*mut c_void, Box<dyn std::error::Error>>` |
| function | `drop` | L36 | `fn drop(&mut self) -> ()` |
