---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ff0eac62bb61"
source_path: "slint-experiment/src/bin/overlay_host/settings_ai.rs"
batch_id: "B02"
total_lines: 1547
symbols_count: 25
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_ai.rs`

- **Batch:** B02
- **Physical Lines:** 1547
- **Coverage:** 1547/1547 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `CodexCopyResult` | L126 | private |
| enum | `ModelTarget` | L435 | pub(crate) |

## Symbols & Routines (25)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `codex_model_label` | L59 | `fn codex_model_label(model: &overlay_backend::codex_subscription::CodexModel) -> SharedString` |
| function | `preferred_codex_model_index` | L63 | `fn preferred_codex_model_index(models: &[overlay_backend::codex_subscription::CodexModel], saved: &str, image_only: bool,) -> Option<usize>` |
| function | `reasoning_label` | L87 | `fn reasoning_label(effort: &str, is_ru: bool) -> String` |
| function | `catalog_is_authoritative` | L100 | `fn catalog_is_authoritative(state: &overlay_backend::codex_subscription::AccountState) -> bool` |
| function | `reasoning_normalization_notice` | L107 | `fn reasoning_normalization_notice(saved: &str, normalized: &str, is_ru: bool,) -> Option<&'static str>` |
| function | `copy_codex_user_code` | L132 | `fn copy_codex_user_code(code: &str, write: impl FnOnce(&str) -> Result<(), E>, ) -> CodexCopyResult` |
| function | `codex_copy_status` | L146 | `fn codex_copy_status(result: CodexCopyResult, is_ru: bool) -> &'static str` |
| function | `invalidate_codex_login_ui` | L156 | `fn invalidate_codex_login_ui() -> u64` |
| function | `codex_ui_is_current` | L162 | `fn codex_ui_is_current(generation: u64) -> bool` |
| function | `invalidate_codex_snapshot_ui` | L166 | `fn invalidate_codex_snapshot_ui() -> u64` |
| function | `codex_snapshot_ui_is_current` | L170 | `fn codex_snapshot_ui_is_current(generation: u64) -> bool` |
| function | `codex_account_label` | L174 | `fn codex_account_label(state: &overlay_backend::codex_subscription::AccountState, is_ru: bool,) -> String` |
| function | `refresh_codex_account_status` | L203 | `fn refresh_codex_account_status(weak: slint::Weak<SettingsWindow>, cfg: overlay_backend::config::SharedConfig,) -> ()` |
| function | `fetch_models` | L447 | `fn fetch_models(weak: slint::Weak<SettingsWindow>, cfg: overlay_backend::config::SharedConfig, target: ModelTarget,) -> ()` |
| function | `refresh_local_model_resource_warning` | L508 | `fn refresh_local_model_resource_warning(win: &SettingsWindow, root: std::path::PathBuf, base_url: String, model: String,) -> ()` |
| function | `wire_ai_settings` | L538 | `fn wire_ai_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig, overlay_weak: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `model` | L1415 | `fn model(id: &str, is_default: bool, image: bool,) -> overlay_backend::codex_subscription::CodexModel` |
| function | `fresh_codex_selection_prefers_luna_but_preserves_explicit_choice` | L1435 | `fn fresh_codex_selection_prefers_luna_but_preserves_explicit_choice() -> ()` |
| function | `codex_vision_selection_filters_text_only_models` | L1453 | `fn codex_vision_selection_filters_text_only_models() -> ()` |
| function | `reasoning_labels_do_not_invent_unsupported_off_value` | L1467 | `fn reasoning_labels_do_not_invent_unsupported_off_value() -> ()` |
| function | `catalog_repairs_require_a_signed_in_account` | L1477 | `fn catalog_repairs_require_a_signed_in_account() -> ()` |
| function | `effort_normalization_is_explained_in_the_current_language` | L1489 | `fn effort_normalization_is_explained_in_the_current_language() -> ()` |
| function | `model_picker_label_contains_no_reasoning_metadata` | L1501 | `fn model_picker_label_contains_no_reasoning_metadata() -> ()` |
| function | `codex_copy_writes_exact_displayed_code_and_skips_blank` | L1508 | `fn codex_copy_writes_exact_displayed_code_and_skips_blank() -> ()` |
| function | `codex_copy_feedback_is_short_generic_and_localized` | L1529 | `fn codex_copy_feedback_is_short_generic_and_localized() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L211
- Spawns asynchronous thread/task at L452
- Spawns asynchronous thread/task at L517
- Spawns asynchronous thread/task at L897
- Spawns asynchronous thread/task at L1091
- Spawns asynchronous thread/task at L1321
- Spawns asynchronous thread/task at L1385

## Configuration Access

- Configuration read at L561: `eprintln!("[overlay-host] ai_bearer saved to config.json");`
