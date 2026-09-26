---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_204c21d0e9ad"
source_path: "slint-experiment/src/bin/overlay_host/tile_ptt.rs"
batch_id: "B03"
total_lines: 510
symbols_count: 4
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_ptt.rs`

- **Batch:** B03
- **Physical Lines:** 510
- **Coverage:** 510/510 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `would` | L82 | private |

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `spawn_ptt_watchdog` | L26 | `fn spawn_ptt_watchdog(stop: Arc<AtomicBool>) -> ()` |
| function | `ptt_initial_copy` | L33 | `fn ptt_initial_copy(is_ru: bool) -> (&'static str, &'static str, &'static str, &'static str)` |
| function | `ptt_tile_error` | L54 | `fn ptt_tile_error(weak: slint::Weak<TileWindow>, msg: &str, is_ru: bool) -> ()` |
| function | `ptt_initial_copy_has_both_ui_languages` | L490 | `fn ptt_initial_copy_has_both_ui_languages() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L27
