---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_b1c47c4a0995"
source_path: "slint-experiment/src/bin/overlay_host/settings_updates.rs"
batch_id: "B02"
total_lines: 151
symbols_count: 1
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "6e0fb5cf5ad5dd0db60d204411086c75bd3f83519e50d3f2b2c4579106ea7b0d"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_updates.rs`

- **Batch:** B02
- **Physical Lines:** 151
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `wire_updates` | L35 | `fn wire_updates(win: &SettingsWindow) -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L50
- Spawns asynchronous thread/task at L104
