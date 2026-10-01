---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_f63cdf80faf8"
source_path: "overlay-backend/examples/ai_probe.rs"
batch_id: "B09"
total_lines: 58
symbols_count: 1
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "a3c1749aef6420e02c35026758d41bd193c6fd7b9b55947d82e0b68beb46b890"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/examples/ai_probe.rs`

- **Batch:** B09
- **Physical Lines:** 58
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `main` | L19 | `fn main() -> ()` |

## Heuristic configuration access matches

- Configuration read at L27: `let ep = cfg.ai_endpoint(false);`
- Configuration read at L30: `ai::set_local_no_think(ep.is_local && !cfg.ai_local_thinking);`
