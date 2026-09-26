---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_f63cdf80faf8"
source_path: "overlay-backend/examples/ai_probe.rs"
batch_id: "B09"
total_lines: 58
symbols_count: 1
review_state: validated
---

# File Map: `overlay-backend/examples/ai_probe.rs`

- **Batch:** B09
- **Physical Lines:** 58
- **Coverage:** 58/58 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `main` | L19 | `fn main() -> ()` |

## Configuration Access

- Configuration read at L27: `let ep = cfg.ai_endpoint(false);`
- Configuration read at L30: `ai::set_local_no_think(ep.is_local && !cfg.ai_local_thinking);`
