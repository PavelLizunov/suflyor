---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_71435e7c14cf"
source_path: "slint-experiment/tests/overlay_bar_geometry.rs"
batch_id: "B05"
total_lines: 235
symbols_count: 7
review_state: validated
---

# File Map: `slint-experiment/tests/overlay_bar_geometry.rs`

- **Batch:** B05
- **Physical Lines:** 235
- **Coverage:** 235/235 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (7)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `element` | L13 | `fn element(bar: &ui::OverlayBarWindow, label: &str) -> ElementHandle` |
| function | `assert_open_tile_geometry` | L19 | `fn assert_open_tile_geometry(bar: &ui::OverlayBarWindow, close_all_label: &str, tile_label: &str) -> ()` |
| function | `assert_compact_geometry` | L102 | `fn assert_compact_geometry(bar: &ui::OverlayBarWindow) -> ()` |
| function | `assert_memory_footer_geometry` | L140 | `fn assert_memory_footer_geometry(bar: &ui::OverlayBarWindow, app_ram_label: &str) -> ()` |
| function | `assert_confirming_geometry` | L179 | `fn assert_confirming_geometry(bar: &ui::OverlayBarWindow, yes_label: &str, no_label: &str) -> ()` |
| function | `open_tile_controls_stay_inside_1280_in_english_and_russian` | L208 | `fn open_tile_controls_stay_inside_1280_in_english_and_russian() -> ()` |
| function | `open_tile_controls_keep_their_text_labels` | L226 | `fn open_tile_controls_keep_their_text_labels() -> ()` |
