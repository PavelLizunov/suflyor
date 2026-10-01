---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7710d4bc1a32"
source_path: "slint-experiment/tests/native_macos_status_guard.rs"
batch_id: "B05"
total_lines: 95
symbols_count: 6
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "c1b1bd878122f3a71d764eea18731e6a01acaa2df7a992722e4928759b516d2e"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/native_macos_status_guard.rs`

- **Batch:** B05
- **Physical Lines:** 95
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `StatusItemGuard` | L52 | pub |

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `source` | L7 | `fn source(root: &Path, relative: &str) -> String` |
| function | `status_bridge_installs_once_and_removes_symmetrically` | L14 | `fn status_bridge_installs_once_and_removes_symmetrically() -> ()` |
| function | `status_menu_has_one_toggle_and_quit_with_plain_labels` | L30 | `fn status_menu_has_one_toggle_and_quit_with_plain_labels() -> ()` |
| function | `status_guard_is_synchronous_main_thread_owned_and_removes_on_drop` | L48 | `fn status_guard_is_synchronous_main_thread_owned_and_removes_on_drop() -> ()` |
| function | `production_host_owns_one_status_guard_through_the_event_loop` | L62 | `fn production_host_owns_one_status_guard_through_the_event_loop() -> ()` |
| function | `status_bridge_has_no_disposable_gate_branding` | L88 | `fn status_bridge_has_no_disposable_gate_branding() -> ()` |
