---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_65222f59b797"
source_path: "slint-experiment/src/native/windows/clipboard.rs"
batch_id: "B04"
total_lines: 23
symbols_count: 4
review_state: validated
---

# File Map: `slint-experiment/src/native/windows/clipboard.rs`

- **Batch:** B04
- **Physical Lines:** 23
- **Coverage:** 23/23 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read_text` | L4 | `fn read_text() -> Option<String>` |
| function | `set_text` | L11 | `fn set_text(text: &str) -> Result<(), String>` |
| function | `write_text` | L16 | `fn write_text(text: &str) -> ()` |
| function | `clear` | L21 | `fn clear() -> ()` |
