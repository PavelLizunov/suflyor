---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_cd36018ac53a"
source_path: "overlay-backend/src/http_log.rs"
batch_id: "B09"
total_lines: 47
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "24332ecc983ef4e9a4b1ac4d6b154e1be3ff4c13a693ffc6ff67a265b6546ec0"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/http_log.rs`

- **Batch:** B09
- **Physical Lines:** 47
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `http_error_line` | L16 | `fn http_error_line(op: &str, status: u16, body_len: usize) -> String` |
| function | `includes_op_status_and_length` | L25 | `fn includes_op_status_and_length() -> ()` |
| function | `never_carries_body_text` | L33 | `fn never_carries_body_text() -> ()` |
