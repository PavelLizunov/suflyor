---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8061d7929f90"
source_path: "slint-experiment/tests/macos_global_hotkeys_guard.rs"
batch_id: "B05"
total_lines: 41
symbols_count: 2
review_state: validated
---

# File Map: `slint-experiment/tests/macos_global_hotkeys_guard.rs`

- **Batch:** B05
- **Physical Lines:** 41
- **Coverage:** 41/41 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read` | L7 | `fn read(relative: &str) -> String` |
| function | `canonical_runtime_registers_and_polls_all_product_hotkeys` | L13 | `fn canonical_runtime_registers_and_polls_all_product_hotkeys() -> ()` |

## Hotkey Mappings

- Hotkey binding/dispatch at L18: `assert!(host.contains("} = register_hotkeys();"));`
- Hotkey binding/dispatch at L21: `assert!(host.contains("global_hotkey::GlobalHotKeyEvent::receiver()"));`
