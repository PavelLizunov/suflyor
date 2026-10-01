---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_204c21d0e9ad"
source_path: "slint-experiment/src/bin/overlay_host/tile_ptt.rs"
batch_id: "B03"
total_lines: 510
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "012709580bb6190c40511f852ede4f2517885fd38288566bed9e290d868fe4b2"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_ptt.rs`

- **Batch:** B03
- **Physical Lines:** 510
- **Semantic coverage:** not measured; no full-line review evidence.

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

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L27
