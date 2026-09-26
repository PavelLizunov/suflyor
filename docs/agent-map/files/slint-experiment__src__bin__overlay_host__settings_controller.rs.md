---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_31dda21e86c9"
source_path: "slint-experiment/src/bin/overlay_host/settings_controller.rs"
batch_id: "B02"
total_lines: 1979
symbols_count: 11
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_controller.rs`

- **Batch:** B02
- **Physical Lines:** 1979
- **Coverage:** 1979/1979 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (11)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `ai_provider_index` | L74 | `fn ai_provider_index(provider: &str, is_macos: bool) -> i32` |
| function | `msg_refresh_after_import` | L1300 | `fn msg_refresh_after_import(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> String` |
| function | `component_row_copy` | L1311 | `fn component_row_copy(kind: overlay_backend::components::ComponentKind, ru: bool,) -> (&'static str, &'static str)` |
| function | `populate_component_rows` | L1381 | `fn populate_component_rows(win: &SettingsWindow, snap: &overlay_backend::config::Config,) -> ()` |
| function | `reset_component_install_state` | L1462 | `fn reset_component_install_state(win: &SettingsWindow) -> ()` |
| function | `refresh_profiles` | L1471 | `fn refresh_profiles(win: &SettingsWindow, c: &overlay_backend::config::Config) -> ()` |
| function | `populate_tile_monitors` | L1495 | `fn populate_tile_monitors(win: &SettingsWindow, c: &overlay_backend::config::Config) -> ()` |
| function | `populate_tts_voices` | L1538 | `fn populate_tts_voices(win: &SettingsWindow, c: &overlay_backend::config::Config) -> ()` |
| function | `populate_token_status` | L1604 | `fn populate_token_status(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `component_row_copy_follows_ui_language` | L1938 | `fn component_row_copy_follows_ui_language() -> ()` |
| function | `codex_has_no_fake_cloud_selection_on_macos` | L1973 | `fn codex_has_no_fake_cloud_selection_on_macos() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L224
- Spawns asynchronous thread/task at L317
- Spawns asynchronous thread/task at L360
- Spawns asynchronous thread/task at L1057
- Spawns asynchronous thread/task at L1180
- Spawns asynchronous thread/task at L1793
