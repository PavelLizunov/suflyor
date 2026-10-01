---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7c4efcd05886"
source_path: "slint-experiment/src/bin/overlay_host/hotkeys.rs"
batch_id: "B04"
total_lines: 231
symbols_count: 2
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "b2006b11e37d5024b8d3c8103d6b3c216e0856136ba3ff022634757bbb739f7f"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/hotkeys.rs`

- **Batch:** B04
- **Physical Lines:** 231
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `HotkeyDiag` | L34 | pub(crate) |
| struct | `RegisteredHotkeys` | L69 | pub(crate) |

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `hotkey_diag_row` | L51 | `fn hotkey_diag_row() -> (i32, String, String)` |
| function | `register_hotkeys` | L104 | `fn register_hotkeys() -> RegisteredHotkeys` |

## Heuristic hotkey matches

- Hotkey binding/dispatch at L5: `//! This module owns the one-time global-hotkey REGISTRATION ([`register_hotkeys`
- Hotkey binding/dispatch at L6: `//! — it builds the process-wide `GlobalHotKeyManager`, registers every F-key`
- Hotkey binding/dispatch at L15: `//! (the `GlobalHotKeyEvent::receiver()` poll loop), because it captures a dozen`
- Hotkey binding/dispatch at L18: `//! [`register_hotkeys`] returns a [`RegisteredHotkeys`] carrying the manager`
- Hotkey binding/dispatch at L62: `/// Result of [`register_hotkeys`]: the live `GlobalHotKeyManager` (which the`
- Hotkey binding/dispatch at L65: `/// `GlobalHotKeyEvent` against the right action. `manager` is `None` when the`
- Hotkey binding/dispatch at L71: `pub manager: Option<global_hotkey::GlobalHotKeyManager>,`
- Hotkey binding/dispatch at L104: `pub(crate) fn register_hotkeys() -> RegisteredHotkeys {`
- Hotkey binding/dispatch at L105: `let hotkey_manager = match global_hotkey::GlobalHotKeyManager::new() {`
- Hotkey binding/dispatch at L108: `eprintln!("[overlay-host] GlobalHotKeyManager init failed: {e}. Hotkeys disabled`
