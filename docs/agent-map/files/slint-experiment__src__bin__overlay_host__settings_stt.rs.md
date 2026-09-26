---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e2e0aa32fe0e"
source_path: "slint-experiment/src/bin/overlay_host/settings_stt.rs"
batch_id: "B02"
total_lines: 404
symbols_count: 13
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_stt.rs`

- **Batch:** B02
- **Physical Lines:** 404
- **Coverage:** 404/404 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `cloud_model_from_index` | L40 | `fn cloud_model_from_index(idx: i32) -> &'static str` |
| function | `cloud_model_index` | L50 | `fn cloud_model_index(model: &str) -> i32` |
| function | `stt_provider_index` | L58 | `fn stt_provider_index(provider: &str) -> i32` |
| function | `stt_provider_from_index` | L67 | `fn stt_provider_from_index(idx: i32) -> &'static str` |
| function | `managed_gigaam_dir` | L75 | `fn managed_gigaam_dir() -> PathBuf` |
| function | `installed_gigaam_dir` | L79 | `fn installed_gigaam_dir(saved: &str) -> Option<PathBuf>` |
| function | `format_mebibytes` | L88 | `fn format_mebibytes(bytes: u64) -> String` |
| function | `update_cloud_model` | L92 | `fn update_cloud_model(config: &mut overlay_backend::config::Config, idx: i32, save: impl FnOnce(&overlay_backend::config::Config) -> Result<(), E>, ) -> Result<(), E>` |
| function | `wire_stt_settings` | L113 | `fn wire_stt_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |
| function | `cloud_model_mapping_is_bounded` | L364 | `fn cloud_model_mapping_is_bounded() -> ()` |
| function | `provider_mapping_is_shared_by_windows_and_macos` | L376 | `fn provider_mapping_is_shared_by_windows_and_macos() -> ()` |
| function | `gigaam_progress_matches_the_windows_megabyte_display` | L387 | `fn gigaam_progress_matches_the_windows_megabyte_display() -> ()` |
| function | `failed_cloud_model_save_rolls_back` | L393 | `fn failed_cloud_model_save_rolls_back() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L129
- Spawns asynchronous thread/task at L214
