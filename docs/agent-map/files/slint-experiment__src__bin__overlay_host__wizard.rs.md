---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_196e40a174ac"
source_path: "slint-experiment/src/bin/overlay_host/wizard.rs"
batch_id: "B01"
total_lines: 488
symbols_count: 1
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/wizard.rs`

- **Batch:** B01
- **Physical Lines:** 488
- **Coverage:** 488/488 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `refill_wizard_summary` | L42 | `fn refill_wizard_summary(w: &WizardWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L164
- Spawns asynchronous thread/task at L206
- Spawns asynchronous thread/task at L241
- Spawns asynchronous thread/task at L285
