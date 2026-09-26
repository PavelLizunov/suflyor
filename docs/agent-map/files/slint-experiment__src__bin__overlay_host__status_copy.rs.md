---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_5dc4448057ef"
source_path: "slint-experiment/src/bin/overlay_host/status_copy.rs"
batch_id: "B01"
total_lines: 357
symbols_count: 20
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/status_copy.rs`

- **Batch:** B01
- **Physical Lines:** 357
- **Coverage:** 357/357 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (20)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `short_model_name` | L8 | `fn short_model_name(full: &str) -> String` |
| function | `active_stack_label` | L23 | `fn active_stack_label(c: &overlay_backend::config::Config) -> String` |
| function | `ai_perf_label` | L66 | `fn ai_perf_label(perf: Option<overlay_backend::ai::RequestPerf>, load_ms: Option<u64>, loading: bool, is_ru: bool,) -> String` |
| function | `memory_size_label` | L118 | `fn memory_size_label(bytes: Option<u64>) -> String` |
| function | `gigaam_accelerator_name` | L130 | `fn gigaam_accelerator_name(enabled: bool) -> &'static str` |
| function | `manual_tile_heading` | L145 | `fn manual_tile_heading(has_transcript: bool, is_ru: bool) -> &'static str` |
| function | `summary_empty_copy` | L157 | `fn summary_empty_copy(is_ru: bool) -> (&'static str, &'static str)` |
| function | `capture_stopped_copy` | L172 | `fn capture_stopped_copy(is_ru: bool) -> (&'static str, &'static str, &'static str)` |
| function | `mic_busy_status` | L188 | `fn mic_busy_status(is_ru: bool) -> &'static str` |
| function | `manual_tile_placeholder` | L196 | `fn manual_tile_placeholder(deep_locked: bool, has_transcript: bool, is_ru: bool) -> &'static str` |
| function | `manual_tile_not_configured` | L208 | `fn manual_tile_not_configured(is_ru: bool) -> &'static str` |
| function | `manual_tile_failure` | L216 | `fn manual_tile_failure(heading: &str, category: &str, is_ru: bool) -> String` |
| function | `refresh_lock_chip` | L227 | `fn refresh_lock_chip(o: &OverlayBarWindow, cfg: &config::SharedConfig) -> ()` |
| function | `active_stack_uses_the_selected_direct_provider_model` | L256 | `fn active_stack_uses_the_selected_direct_provider_model() -> ()` |
| function | `manual_tile_heading_carries_no_number` | L270 | `fn manual_tile_heading_carries_no_number() -> ()` |
| function | `manual_tile_copy_follows_ui_language` | L286 | `fn manual_tile_copy_follows_ui_language() -> ()` |
| function | `summary_empty_notice_has_both_ui_languages` | L308 | `fn summary_empty_notice_has_both_ui_languages() -> ()` |
| function | `mic_busy_status_follows_ui_language` | L316 | `fn mic_busy_status_follows_ui_language() -> ()` |
| function | `performance_and_memory_labels_are_honest_and_localized` | L322 | `fn performance_and_memory_labels_are_honest_and_localized() -> ()` |
| function | `capture_stopped_copy_is_generic_localized_and_requests_manual_start` | L342 | `fn capture_stopped_copy_is_generic_localized_and_requests_manual_start() -> ()` |

## Configuration Access

- Configuration read at L259: `cfg.ai_model = "claude-haiku-4-5".into();`
- Configuration read at L260: `cfg.ai_provider = "codex".into();`
