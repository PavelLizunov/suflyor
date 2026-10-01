---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c2434e16b2bc"
source_path: "slint-experiment/tests/macos_capture_watchdog_guard.rs"
batch_id: "B05"
total_lines: 134
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "8d19ff4a487ed78f29b314359bf60f106e699520ca737134d93abee2b75c8888"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/macos_capture_watchdog_guard.rs`

- **Batch:** B05
- **Physical Lines:** 134
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read` | L7 | `fn read(relative: &str) -> String` |
| function | `canonical_runtime_owns_the_macos_watchdog_timer` | L13 | `fn canonical_runtime_owns_the_macos_watchdog_timer() -> ()` |
| function | `manual_session_tasks_drop_stale_macos_intents` | L82 | `fn manual_session_tasks_drop_stale_macos_intents() -> ()` |
