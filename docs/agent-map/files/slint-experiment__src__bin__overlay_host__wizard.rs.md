---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_196e40a174ac"
source_path: "slint-experiment/src/bin/overlay_host/wizard.rs"
batch_id: "B01"
total_lines: 488
symbols_count: 1
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "0a8ffa80c12a7a9ede745bef843e0796a4ad509017d5be8475c6ddcc4011040d"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/wizard.rs`

- **Batch:** B01
- **Physical Lines:** 488
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `refill_wizard_summary` | L42 | `fn refill_wizard_summary(w: &WizardWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L164
- Spawns asynchronous thread/task at L206
- Spawns asynchronous thread/task at L241
- Spawns asynchronous thread/task at L285
