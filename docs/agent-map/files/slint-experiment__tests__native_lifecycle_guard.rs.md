---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_f5516d01f2f2"
source_path: "slint-experiment/tests/native_lifecycle_guard.rs"
batch_id: "B05"
total_lines: 55
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "fd50c710789112d2f92f22e9b2688a402624876b8f93f808deedf1a1c6d14286"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/native_lifecycle_guard.rs`

- **Batch:** B05
- **Physical Lines:** 55
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `assert_create_mutex_isolated_to` | L7 | `fn assert_create_mutex_isolated_to(path: &Path, allowed: &Path) -> ()` |
| function | `process_singleton_is_owned_by_the_windows_lifecycle_adapter` | L24 | `fn process_singleton_is_owned_by_the_windows_lifecycle_adapter() -> ()` |
| function | `macos_lifecycle_uses_a_native_file_lock` | L49 | `fn macos_lifecycle_uses_a_native_file_lock() -> ()` |
