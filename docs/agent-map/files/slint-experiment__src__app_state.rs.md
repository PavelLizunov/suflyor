---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_46113cce7e0d"
source_path: "slint-experiment/src/app_state.rs"
batch_id: "B01"
total_lines: 504
symbols_count: 14
review_state: validated
---

# File Map: `slint-experiment/src/app_state.rs`

- **Batch:** B01
- **Physical Lines:** 504
- **Coverage:** 504/504 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `only` | L8 | private |
| struct | `AppState` | L16 | pub |
| struct | `LocalAiBusyGuard` | L108 | pub |
| struct | `PaletteRow` | L197 | pub |
| struct | `a` | L281 | private |

## Symbols & Routines (14)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `default` | L85 | `fn default() -> Self` |
| function | `try_acquire` | L116 | `fn try_acquire(flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Option<Self>` |
| function | `drop` | L132 | `fn drop(&mut self) -> ()` |
| function | `new_shared_state` | L141 | `fn new_shared_state() -> SharedState` |
| function | `next_model` | L156 | `fn next_model(current: &str) -> &'static str` |
| function | `format_timer` | L167 | `fn format_timer(secs: u64) -> String` |
| function | `tile_title_line` | L188 | `fn tile_title_line(s: &str) -> String` |
| function | `palette_row_from` | L209 | `fn palette_row_from(entry: &overlay_backend::kb::KBEntry) -> PaletteRow` |
| function | `classify_ai_error` | L239 | `fn classify_ai_error(msg: &str) -> &'static str` |
| function | `local_ai_busy_guard_is_a_single_latch` | L284 | `fn local_ai_busy_guard_is_a_single_latch() -> ()` |
| function | `tile_title_line_collapses_to_one_line` | L314 | `fn tile_title_line_collapses_to_one_line() -> ()` |
| function | `next_model_outputs_are_canonical_ids` | L339 | `fn next_model_outputs_are_canonical_ids() -> ()` |
| function | `palette_row_preview_slicing` | L394 | `fn palette_row_preview_slicing() -> ()` |
| function | `classify_ai_error_table` | L447 | `fn classify_ai_error_table() -> ()` |
