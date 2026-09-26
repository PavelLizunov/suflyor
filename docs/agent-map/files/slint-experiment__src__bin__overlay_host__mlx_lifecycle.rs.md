---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_380a36844183"
source_path: "slint-experiment/src/bin/overlay_host/mlx_lifecycle.rs"
batch_id: "B01"
total_lines: 240
symbols_count: 11
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/mlx_lifecycle.rs`

- **Batch:** B01
- **Physical Lines:** 240
- **Coverage:** 240/240 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (11)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `lifecycle_lock` | L9 | `fn lifecycle_lock() -> &'static std::sync::Mutex<()>` |
| function | `selected_mlx_model` | L14 | `fn selected_mlx_model(route: AskRoute, config: &overlay_backend::config::Config) -> Option<String>` |
| function | `route_needs_mlx` | L32 | `fn route_needs_mlx(route: AskRoute, config: &overlay_backend::config::Config) -> bool` |
| function | `activate_mlx_model` | L40 | `fn activate_mlx_model(model: &str) -> Result<(), ()>` |
| function | `stop_mlx_model` | L60 | `fn stop_mlx_model() -> ()` |
| function | `stop_mlx_model_if_active` | L70 | `fn stop_mlx_model_if_active(model: &str) -> ()` |
| function | `resolve_route_endpoint` | L83 | `fn resolve_route_endpoint(route: AskRoute, config: &overlay_backend::config::SharedConfig,) -> Result<overlay_backend::config::AiEndpoint, ()>` |
| function | `start_mlx_for_unlock` | L113 | `fn start_mlx_for_unlock(config: &overlay_backend::config::SharedConfig,) -> Result<(), ()>` |
| function | `show_mlx_runtime_error` | L141 | `fn show_mlx_runtime_error(weak: slint::Weak<TileWindow>, is_ru: bool) -> ()` |
| function | `spawn_mlx_runtime_error` | L166 | `fn spawn_mlx_runtime_error(events: &Arc<dyn RuntimeEvents>, config: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `selection_covers_text_vision_same_and_cloud_without_fallback` | L213 | `fn selection_covers_text_vision_same_and_cloud_without_fallback() -> ()` |

## Key Behaviors & Concurrency

- Acquires synchronization mutex at L9

## Configuration Access

- Configuration read at L16: `AskRoute::Text if config.ai_provider == "mlx" => Some(config.ai_mlx_model.clone(`
- Configuration read at L22: `&& config.ai_provider == "mlx"`
- Configuration read at L23: `&& overlay_backend::mlx_install::catalog_model(&config.ai_mlx_model)`
- Configuration read at L26: `Some(config.ai_mlx_model.clone())`
- Configuration read at L219: `config.ai_provider = "mlx".into();`
- Configuration read at L234: `config.ai_mlx_model = overlay_backend::mlx_install::DEFAULT_VISION_MODEL.into();`
