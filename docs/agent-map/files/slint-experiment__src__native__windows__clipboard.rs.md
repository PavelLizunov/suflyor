---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_65222f59b797"
source_path: "slint-experiment/src/native/windows/clipboard.rs"
batch_id: "B04"
total_lines: 23
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "e4566e0c54c5641b825a32c1b233c6df11fd7b307b9e8fe1ba05f4d59a1e28a3"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/native/windows/clipboard.rs`

- **Batch:** B04
- **Physical Lines:** 23
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read_text` | L4 | `fn read_text() -> Option<String>` |
| function | `set_text` | L11 | `fn set_text(text: &str) -> Result<(), String>` |
| function | `write_text` | L16 | `fn write_text(text: &str) -> ()` |
| function | `clear` | L21 | `fn clear() -> ()` |
