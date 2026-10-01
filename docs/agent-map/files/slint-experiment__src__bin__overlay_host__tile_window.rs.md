---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_20e4ab8c9b94"
source_path: "slint-experiment/src/bin/overlay_host/tile_window.rs"
batch_id: "B03"
total_lines: 450
symbols_count: 10
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "c254a31f89c412d96305225c3ca5250ec4e9d0fabbccb30906a694f88098d17a"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_window.rs`

- **Batch:** B03
- **Physical Lines:** 450
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| trait | `secondary` | L396 | private |

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `next_tile_id` | L64 | `fn next_tile_id() -> i32` |
| function | `register_tile_stream` | L89 | `fn register_tile_stream(tile_id: i32, handle: tokio::task::AbortHandle) -> ()` |
| function | `abort_tile_stream` | L102 | `fn abort_tile_stream(tile_id: i32) -> ()` |
| function | `abort_all_tile_streams` | L116 | `fn abort_all_tile_streams() -> ()` |
| function | `cascade_cycle` | L130 | `fn cascade_cycle(raw_seq: usize, total_slots: usize, x_base: i32, y_base: i32, mon_left: i32, mon_bottom: i32, real_h: i32, cascade_dx: i32, cascade_dy: i32,) -> usize` |
| function | `toggle_tile_maximize` | L152 | `fn toggle_tile_maximize(hwnd: slint_replay::win32::HWND, tile: &TileWindow) -> ()` |
| function | `wire_tile_drag` | L212 | `fn wire_tile_drag(tile: &TileWindow) -> ()` |
| function | `present_tile_window` | L259 | `fn present_tile_window(tile: &TileWindow) -> ()` |
| function | `apply_tile_hwnd_with_monitor` | L276 | `fn apply_tile_hwnd_with_monitor(tile: &TileWindow) -> ()` |
| function | `cascade_clamp_pins_in_band` | L435 | `fn cascade_clamp_pins_in_band() -> ()` |
