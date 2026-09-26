---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3bf96515833b"
source_path: "overlay-backend/src/mlx_runtime.rs"
batch_id: "B06"
total_lines: 749
symbols_count: 41
review_state: validated
---

# File Map: `overlay-backend/src/mlx_runtime.rs`

- **Batch:** B06
- **Physical Lines:** 749
- **Coverage:** 749/749 lines (100%)

## Types & Structures (6)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `MlxEndpoint` | L16 | pub |
| struct | `MlxMemorySample` | L34 | pub |
| struct | `RuntimeState` | L80 | private |
| struct | `MlxRequestLease` | L103 | pub |
| struct | `StartupWire` | L279 | private |
| struct | `ReadyWire` | L288 | private |

## Symbols & Routines (41)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `fmt` | L23 | `fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` |
| function | `suflyor_process_footprint` | L41 | `fn suflyor_process_footprint(pid: u32, bytes: *mut u64) -> i32` |
| function | `process_footprint` | L45 | `fn process_footprint(pid: u32) -> Option<u64>` |
| function | `memory_sample` | L54 | `fn memory_sample() -> MlxMemorySample` |
| function | `last_load_ms` | L75 | `fn last_load_ms() -> Option<u64>` |
| function | `state` | L92 | `fn state() -> &'static Mutex<RuntimeState>` |
| function | `lifecycle` | L97 | `fn lifecycle() -> &'static Mutex<()>` |
| function | `drop` | L108 | `fn drop(&mut self) -> ()` |
| function | `active_endpoint_for_model` | L114 | `fn active_endpoint_for_model(model: &str) -> Option<MlxEndpoint>` |
| function | `endpoint_matching_model` | L120 | `fn endpoint_matching_model(endpoint: Option<&MlxEndpoint>, model: &str) -> Option<MlxEndpoint>` |
| function | `is_owned_endpoint` | L125 | `fn is_owned_endpoint(base_url: &str) -> bool` |
| function | `owned_endpoint_matches` | L135 | `fn owned_endpoint_matches(active: Option<&MlxEndpoint>, last_owned: Option<&str>, base_url: &str,) -> bool` |
| function | `selected_model` | L144 | `fn selected_model() -> Option<String>` |
| function | `begin_intent` | L154 | `fn begin_intent() -> u64` |
| function | `intent_is_current` | L161 | `fn intent_is_current(generation: u64) -> bool` |
| function | `start` | L165 | `fn start(model: &str) -> Result<MlxEndpoint>` |
| function | `start_locked` | L170 | `fn start_locked(model: &str) -> Result<MlxEndpoint>` |
| function | `stop` | L182 | `fn stop() -> ()` |
| function | `stop_locked` | L187 | `fn stop_locked() -> ()` |
| function | `stop_if_idle` | L212 | `fn stop_if_idle() -> bool` |
| function | `acquire_request` | L228 | `fn acquire_request(model: &str) -> Result<(MlxEndpoint, MlxRequestLease)>` |
| function | `release_request` | L246 | `fn release_request(state: &mut RuntimeState, generation: u64) -> ()` |
| function | `clear_endpoint` | L252 | `fn clear_endpoint(state: &mut RuntimeState) -> ()` |
| function | `reap_exited_child` | L259 | `fn reap_exited_child(state: &mut RuntimeState) -> ()` |
| function | `reap_exited_child` | L275 | `fn reap_exited_child(_state: &mut RuntimeState) -> ()` |
| function | `parse_ready` | L296 | `fn parse_ready(line: &str, expected_model: &str) -> Result<u16>` |
| function | `is_valid_mlx_token` | L312 | `fn is_valid_mlx_token(token: &str) -> bool` |
| function | `is_valid_mlx_code` | L321 | `fn is_valid_mlx_code(code: &str) -> bool` |
| function | `parse_mlx_stderr_line` | L329 | `fn parse_mlx_stderr_line(line: &str) -> Option<&str>` |
| function | `drain_mlx_stderr` | L365 | `fn drain_mlx_stderr(stderr: std::process::ChildStderr) -> ()` |
| function | `random_token` | L391 | `fn random_token() -> Result<String>` |
| function | `start_macos` | L398 | `fn start_macos(model: &str) -> Result<MlxEndpoint>` |
| function | `probe` | L543 | `fn probe(endpoint: &MlxEndpoint, path: &str, expected_model: Option<&str>) -> Result<bool>` |
| function | `ready_parser_is_bounded_exact_and_loopback_port_only` | L612 | `fn ready_parser_is_bounded_exact_and_loopback_port_only() -> ()` |
| function | `generation_fence_rejects_stale_intents` | L633 | `fn generation_fence_rejects_stale_intents() -> ()` |
| function | `endpoint_debug_redacts_session_token` | L645 | `fn endpoint_debug_redacts_session_token() -> ()` |
| function | `ownership_matches_only_exact_current_or_last_endpoint` | L657 | `fn ownership_matches_only_exact_current_or_last_endpoint() -> ()` |
| function | `request_release_is_generation_fenced` | L686 | `fn request_release_is_generation_fenced() -> ()` |
| function | `mlx_stderr_privacy_drain_accepts_exact_structured_shape` | L702 | `fn mlx_stderr_privacy_drain_accepts_exact_structured_shape() -> ()` |
| function | `mlx_stderr_privacy_drain_rejects_malformed_path_and_unbounded_tokens` | L711 | `fn mlx_stderr_privacy_drain_rejects_malformed_path_and_unbounded_tokens() -> ()` |
| function | `non_macos_runtime_fails_closed` | L744 | `fn non_macos_runtime_fails_closed() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L480
- Spawns asynchronous thread/task at L481
