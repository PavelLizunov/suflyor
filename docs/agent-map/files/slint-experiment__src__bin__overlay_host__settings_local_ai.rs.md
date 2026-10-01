---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7887c16d9193"
source_path: "slint-experiment/src/bin/overlay_host/settings_local_ai.rs"
batch_id: "B02"
total_lines: 1524
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "612496fa6da4fd08ec6941aeb6132b71bfaee3dbf34244d36e29fdd4cfb9b7b4"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_local_ai.rs`

- **Batch:** B02
- **Physical Lines:** 1524
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `refresh_local_context_controls` | L37 | `fn refresh_local_context_controls(win: &SettingsWindow, cfg: &overlay_backend::config::Config,) -> ()` |
| function | `refresh_local_context_preview` | L69 | `fn refresh_local_context_preview(win: &SettingsWindow, cfg: &overlay_backend::config::Config, model: overlay_backend::local_ai::ManagedModel, profile: overlay_backend::local_ai::HardwareModelProfile, preset: overlay_backend::local_ai::LocalContextPreset, custom_active: bool,) -> ()` |
| function | `wire_local_ai` | L112 | `fn wire_local_ai(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig, state: &slint_replay::app_state::SharedState, overlay_weak: &slint::Weak<OverlayBarWindow>,) -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L173
- Spawns asynchronous thread/task at L607
- Spawns asynchronous thread/task at L919
- Spawns asynchronous thread/task at L1070
- Spawns asynchronous thread/task at L1186
- Spawns asynchronous thread/task at L1359

## Heuristic configuration access matches

- Configuration read at L43: `&cfg.ai_local_model,`
- Configuration read at L44: `cfg.ai_local_quality,`
- Configuration read at L47: `overlay_backend::local_ai::custom_gguf_display_name(&cfg.ai_local_custom_gguf);`
- Configuration read at L55: `let preset = overlay_backend::local_ai::LocalContextPreset::from_config(&cfg.ai_`
- Configuration read at L687: `// cfg.ai_local_model; the request "model" field is ignored by`
