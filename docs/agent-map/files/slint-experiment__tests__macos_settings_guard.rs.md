---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a575472c41c6"
source_path: "slint-experiment/tests/macos_settings_guard.rs"
batch_id: "B05"
total_lines: 227
symbols_count: 8
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "13aa3da31fd8e4f390668f18db5a115a68b644bd4809b96b51a3176681de4f4a"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/macos_settings_guard.rs`

- **Batch:** B05
- **Physical Lines:** 227
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

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

## Heuristic configuration access matches

- Configuration read at L183: `assert!(config.contains("if cfg!(target_os = \"macos\") {\n        \"gigaam\".in`
