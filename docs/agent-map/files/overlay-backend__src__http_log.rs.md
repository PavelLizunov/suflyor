---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_cd36018ac53a"
source_path: "overlay-backend/src/http_log.rs"
batch_id: "B09"
total_lines: 47
symbols_count: 3
review_state: validated
---

# File Map: `overlay-backend/src/http_log.rs`

- **Batch:** B09
- **Physical Lines:** 47
- **Coverage:** 47/47 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `http_error_line` | L16 | `fn http_error_line(op: &str, status: u16, body_len: usize) -> String` |
| function | `includes_op_status_and_length` | L25 | `fn includes_op_status_and_length() -> ()` |
| function | `never_carries_body_text` | L33 | `fn never_carries_body_text() -> ()` |
