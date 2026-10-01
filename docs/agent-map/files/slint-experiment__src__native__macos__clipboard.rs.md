---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2c99d8ec5010"
source_path: "slint-experiment/src/native/macos/clipboard.rs"
batch_id: "B04"
total_lines: 58
symbols_count: 11
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "2bd952cca85d6b92475b2c7bd0f4fca32f7c48b59939146a13279265acbece7f"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/native/macos/clipboard.rs`

- **Batch:** B04
- **Physical Lines:** 58
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (11)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `suflyor_macos_clipboard_set_text` | L6 | `fn suflyor_macos_clipboard_set_text(bytes: *const u8, length: usize) -> c_int` |
| function | `suflyor_macos_clipboard_read_text` | L7 | `fn suflyor_macos_clipboard_read_text(bytes: *mut u8, capacity: usize) -> usize` |
| function | `suflyor_macos_clipboard_clear` | L8 | `fn suflyor_macos_clipboard_clear() -> ()` |
| function | `suflyor_macos_copy_modifiers_released` | L9 | `fn suflyor_macos_copy_modifiers_released() -> c_int` |
| function | `suflyor_macos_send_command_c` | L10 | `fn suflyor_macos_send_command_c() -> c_int` |
| function | `set_text` | L15 | `fn set_text(text: &str) -> Result<(), String>` |
| function | `read_text` | L26 | `fn read_text() -> Option<String>` |
| function | `write_text` | L40 | `fn write_text(text: &str) -> ()` |
| function | `clear` | L45 | `fn clear() -> ()` |
| function | `copy_modifiers_released` | L50 | `fn copy_modifiers_released() -> bool` |
| function | `send_command_c` | L56 | `fn send_command_c() -> bool` |
