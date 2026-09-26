---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ecf89aa70c98"
source_path: "slint-experiment/src/bin/overlay_host/settings_vision.rs"
batch_id: "B02"
total_lines: 395
symbols_count: 3
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_vision.rs`

- **Batch:** B02
- **Physical Lines:** 395
- **Coverage:** 395/395 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `vision_provider_index_from_id` | L25 | `fn vision_provider_index_from_id(provider: &str) -> i32` |
| function | `wire_vision_settings` | L45 | `fn wire_vision_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `vision_provider_ids_match_the_shared_settings_catalog` | L382 | `fn vision_provider_ids_match_the_shared_settings_catalog() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L228
- Spawns asynchronous thread/task at L273
- Spawns asynchronous thread/task at L332
