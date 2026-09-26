---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a575472c41c6"
source_path: "slint-experiment/tests/macos_settings_guard.rs"
batch_id: "B05"
total_lines: 227
symbols_count: 8
review_state: validated
---

# File Map: `slint-experiment/tests/macos_settings_guard.rs`

- **Batch:** B05
- **Physical Lines:** 227
- **Coverage:** 227/227 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `read` | L7 | `fn read(relative: &str) -> String` |
| function | `canonical_runtime_reaches_the_shared_settings_controller` | L13 | `fn canonical_runtime_reaches_the_shared_settings_controller() -> ()` |
| function | `macos_reports_builtin_apple_vision_without_tesseract_install_actions` | L26 | `fn macos_reports_builtin_apple_vision_without_tesseract_install_actions() -> ()` |
| function | `macos_settings_hide_windows_only_setup_and_managed_local_actions` | L42 | `fn macos_settings_hide_windows_only_setup_and_managed_local_actions() -> ()` |
| function | `macos_provider_catalogs_and_components_keep_platform_indices_honest` | L63 | `fn macos_provider_catalogs_and_components_keep_platform_indices_honest() -> ()` |
| function | `model_dropdowns_keep_selected_indices_across_provider_switches` | L163 | `fn model_dropdowns_keep_selected_indices_across_provider_switches() -> ()` |
| function | `macos_gigaam_is_coreml_backed_and_primary_on_fresh_config` | L175 | `fn macos_gigaam_is_coreml_backed_and_primary_on_fresh_config() -> ()` |
| function | `macos_mlx_models_are_opt_in_downloads_with_honest_state` | L200 | `fn macos_mlx_models_are_opt_in_downloads_with_honest_state() -> ()` |

## Configuration Access

- Configuration read at L183: `assert!(config.contains("if cfg!(target_os = \"macos\") {\n        \"gigaam\".in`
