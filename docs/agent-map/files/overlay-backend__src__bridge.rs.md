---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4337aa9fc85f"
source_path: "overlay-backend/src/bridge.rs"
batch_id: "B06"
total_lines: 750
symbols_count: 21
review_state: validated
---

# File Map: `overlay-backend/src/bridge.rs`

- **Batch:** B06
- **Physical Lines:** 750
- **Coverage:** 750/750 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `BridgeHandle` | L74 | pub |

## Symbols & Routines (21)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `constant_time_eq` | L36 | `fn constant_time_eq(a: &[u8], b: &[u8]) -> bool` |
| function | `verify_authorization` | L50 | `fn verify_authorization(supplied: &[u8], expected: &[u8]) -> bool` |
| function | `bind_host` | L81 | `fn bind_host(configured: &str) -> String` |
| function | `is_loopback_host` | L93 | `fn is_loopback_host(host: &str) -> bool` |
| function | `generate_token` | L102 | `fn generate_token() -> String` |
| function | `stop` | L117 | `fn stop(mut self) -> ()` |
| function | `start` | L128 | `fn start(cfg: SharedConfig) -> Result<BridgeHandle, String>` |
| function | `handle_request` | L173 | `fn handle_request(mut req: tiny_http::Request, cfg: &SharedConfig, token: &str) -> ()` |
| function | `split_query` | L229 | `fn split_query(url: &str) -> (&str, Vec<(String, String)>)` |
| function | `percent_decode` | L245 | `fn percent_decode(s: &str) -> String` |
| function | `hex_val` | L273 | `fn hex_val(b: u8) -> Option<u8>` |
| function | `scratch` | L557 | `fn scratch() -> (Store, SharedConfig)` |
| function | `call` | L564 | `fn call(store: &mut Store, cfg: &SharedConfig, method: &str, url: &str, body: serde_json::Value,) -> (u16, serde_json::Value, bool)` |
| function | `health_and_unknown_routes` | L576 | `fn health_and_unknown_routes() -> ()` |
| function | `sessions_list_and_missing_session` | L589 | `fn sessions_list_and_missing_session() -> ()` |
| function | `search_requires_query` | L620 | `fn search_requires_query() -> ()` |
| function | `suggest_lands_in_candidate_queue_not_memory` | L629 | `fn suggest_lands_in_candidate_queue_not_memory() -> ()` |
| function | `profile_upsert_create_update_activate` | L668 | `fn profile_upsert_create_update_activate() -> ()` |
| function | `query_decoding_cyrillic` | L714 | `fn query_decoding_cyrillic() -> ()` |
| function | `constant_time_eq_behavior` | L721 | `fn constant_time_eq_behavior() -> ()` |
| function | `verify_authorization_behavior` | L731 | `fn verify_authorization_behavior() -> ()` |
