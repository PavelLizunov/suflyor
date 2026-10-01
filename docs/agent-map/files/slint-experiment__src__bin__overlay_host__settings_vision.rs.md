---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ecf89aa70c98"
source_path: "slint-experiment/src/bin/overlay_host/settings_vision.rs"
batch_id: "B02"
total_lines: 395
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "0600b3ea6db781f8667c5198b3dfbc843e6e3f5311c47eb01cf382b3036499ab"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_vision.rs`

- **Batch:** B02
- **Physical Lines:** 395
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `vision_provider_index_from_id` | L25 | `fn vision_provider_index_from_id(provider: &str) -> i32` |
| function | `wire_vision_settings` | L45 | `fn wire_vision_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `vision_provider_ids_match_the_shared_settings_catalog` | L382 | `fn vision_provider_ids_match_the_shared_settings_catalog() -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L228
- Spawns asynchronous thread/task at L273
- Spawns asynchronous thread/task at L332
