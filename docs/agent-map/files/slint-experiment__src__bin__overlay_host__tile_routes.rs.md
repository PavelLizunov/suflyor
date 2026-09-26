---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3a419a6e9338"
source_path: "slint-experiment/src/bin/overlay_host/tile_routes.rs"
batch_id: "B03"
total_lines: 146
symbols_count: 7
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_routes.rs`

- **Batch:** B03
- **Physical Lines:** 146
- **Coverage:** 146/146 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `AskRoute` | L19 | pub(crate) |

## Symbols & Routines (7)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `has_required_auth` | L33 | `fn has_required_auth(self, c: &overlay_backend::config::Config) -> bool` |
| function | `endpoint` | L43 | `fn endpoint(self, c: &overlay_backend::config::Config,) -> overlay_backend::config::AiEndpoint` |
| function | `max_tokens` | L65 | `fn max_tokens(self) -> u32` |
| function | `attaches_screenshot` | L72 | `fn attaches_screenshot(self) -> bool` |
| function | `live_route` | L84 | `fn live_route(initial: AskRoute) -> LiveRoute` |
| function | `remote_routes_require_bearer_and_local_does_not` | L95 | `fn remote_routes_require_bearer_and_local_does_not() -> ()` |
| function | `missing_vision_route_never_falls_back_to_the_text_model` | L136 | `fn missing_vision_route_never_falls_back_to_the_text_model() -> ()` |

## Configuration Access

- Configuration read at L104: `config.ai_bearer = "token".into();`
- Configuration read at L108: `config.ai_provider = "local".into();`
- Configuration read at L109: `config.ai_local_bearer.clear();`
- Configuration read at L112: `config.ai_provider = "codex".into();`
- Configuration read at L138: `config.ai_provider = "codex".into();`
