---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8061d7929f90"
source_path: "slint-experiment/tests/macos_global_hotkeys_guard.rs"
batch_id: "B05"
total_lines: 41
symbols_count: 2
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "99ce988643489c65055b8ea0441670795d97a8a0046f2a9bbb712b5df78921bc"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/macos_global_hotkeys_guard.rs`

- **Batch:** B05
- **Physical Lines:** 41
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read` | L7 | `fn read(relative: &str) -> String` |
| function | `canonical_runtime_registers_and_polls_all_product_hotkeys` | L13 | `fn canonical_runtime_registers_and_polls_all_product_hotkeys() -> ()` |

## Heuristic hotkey matches

- Hotkey binding/dispatch at L18: `assert!(host.contains("} = register_hotkeys();"));`
- Hotkey binding/dispatch at L21: `assert!(host.contains("global_hotkey::GlobalHotKeyEvent::receiver()"));`
