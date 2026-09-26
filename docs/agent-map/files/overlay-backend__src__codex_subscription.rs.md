---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_d8be9762539b"
source_path: "overlay-backend/src/codex_subscription.rs"
batch_id: "B06"
total_lines: 2065
symbols_count: 82
review_state: validated
---

# File Map: `overlay-backend/src/codex_subscription.rs`

- **Batch:** B06
- **Physical Lines:** 2065
- **Coverage:** 2065/2065 lines (100%)

## Types & Structures (11)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `AccountState` | L59 | pub |
| enum | `LoginEvent` | L68 | pub |
| enum | `TurnEvent` | L100 | pub |
| enum | `TurnFailure` | L107 | pub |
| enum | `TurnNotification` | L119 | private |
| enum | `RpcFailure` | L132 | private |
| enum | `LoginProgress` | L1302 | private |
| struct | `LoginAttempt` | L56 | pub |
| struct | `CodexModel` | L81 | pub |
| struct | `ProviderSnapshot` | L91 | pub |
| struct | `AppServer` | L145 | private |

## Symbols & Routines (82)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `drop` | L152 | `fn drop(&mut self) -> ()` |
| function | `start` | L159 | `fn start() -> Result<Self, RpcFailure>` |
| function | `initialize` | L199 | `fn initialize(&mut self) -> Result<(), RpcFailure>` |
| function | `initialize_experimental` | L203 | `fn initialize_experimental(&mut self) -> Result<(), RpcFailure>` |
| function | `initialize_with_experimental` | L207 | `fn initialize_with_experimental(&mut self, experimental: bool) -> Result<(), RpcFailure>` |
| function | `write` | L213 | `fn write(&mut self, value: Value) -> Result<(), RpcFailure>` |
| function | `request` | L219 | `fn request(&mut self, id: u64, method: &str, params: Value) -> Result<Value, RpcFailure>` |
| function | `request_secure` | L224 | `fn request_secure(&mut self, id: u64, method: &str, params: Value,) -> Result<Value, RpcFailure>` |
| function | `wait_for_response` | L243 | `fn wait_for_response(&self, id: u64, timeout: Duration) -> Result<Value, RpcFailure>` |
| function | `initialize_params` | L258 | `fn initialize_params(experimental: bool) -> Value` |
| function | `provider_snapshot` | L280 | `fn provider_snapshot() -> ProviderSnapshot` |
| function | `provider_snapshot_inner` | L296 | `fn provider_snapshot_inner() -> Result<ProviderSnapshot, RpcFailure>` |
| function | `list_models_paginated` | L320 | `fn list_models_paginated(server: &mut AppServer, first_id: u64,) -> Result<Vec<CodexModel>, RpcFailure>` |
| function | `append_model_page` | L340 | `fn append_model_page(models: &mut Vec<CodexModel>, page: &Value,) -> Result<Option<String>, RpcFailure>` |
| function | `parse_model` | L372 | `fn parse_model(value: &Value) -> Result<CodexModel, RpcFailure>` |
| function | `string_array` | L402 | `fn string_array(value: Option<&Value>, max_items: usize, max_len: usize,) -> Result<Vec<String>, RpcFailure>` |
| function | `reasoning_effort_array` | L424 | `fn reasoning_effort_array(value: Option<&Value>) -> Result<Vec<String>, RpcFailure>` |
| function | `parse_rate_limits` | L442 | `fn parse_rate_limits(value: &Value) -> Option<String>` |
| function | `safe_model_id` | L453 | `fn safe_model_id(value: &str) -> Option<String>` |
| function | `safe_short_label` | L462 | `fn safe_short_label(value: &str) -> Option<String>` |
| function | `safe_display_label` | L466 | `fn safe_display_label(value: &str) -> Option<String>` |
| function | `account_state` | L476 | `fn account_state() -> AccountState` |
| function | `account_state_inner` | L484 | `fn account_state_inner() -> Result<AccountState, RpcFailure>` |
| function | `begin_device_login` | L494 | `fn begin_device_login() -> LoginAttempt` |
| function | `cancel_pending_login` | L499 | `fn cancel_pending_login() -> ()` |
| function | `ensure_current_login` | L587 | `fn ensure_current_login(generation: u64) -> Result<(), RpcFailure>` |
| function | `disconnect` | L596 | `fn disconnect() -> AccountState` |
| function | `disconnect_inner` | L605 | `fn disconnect_inner() -> Result<(), RpcFailure>` |
| function | `run_turn` | L614 | `fn run_turn(model: &str, reasoning_effort: Option<&str>, messages: &[ChatMessage], mut notify: impl FnMut(TurnEvent) -> bool, ) -> Result<TokenUsage, TurnFailure>` |
| function | `map_turn_failure` | L623 | `fn map_turn_failure(failure: RpcFailure) -> TurnFailure` |
| function | `run_turn_inner` | L636 | `fn run_turn_inner(model: &str, reasoning_effort: Option<&str>, messages: &[ChatMessage], notify: &mut impl FnMut(TurnEvent) -> bool, ) -> Result<TokenUsage, RpcFailure>` |
| function | `turn_stage_failure` | L708 | `fn turn_stage_failure(stage: &'static str, failure: RpcFailure) -> RpcFailure` |
| function | `receive_turn` | L713 | `fn receive_turn(server: &mut AppServer, thread_id: &str, model: &str, reasoning_effort: Option<&str>, workspace: &Path, turn_id: &mut Option<String>, notify: &mut impl FnMut(TurnEvent) -> bool, ) -> Result<TokenUsage, RpcFailure>` |
| function | `emit_turn_event` | L814 | `fn emit_turn_event(notify: &mut impl FnMut(TurnEvent) -> bool, event: TurnEvent, ) -> Result<(), RpcFailure>` |
| function | `parse_turn_notification` | L821 | `fn parse_turn_notification(message: &Value, thread_id: &str, turn_id: &str,) -> Result<TurnNotification, RpcFailure>` |
| function | `validate_safe_items` | L940 | `fn validate_safe_items(items: Option<&Value>) -> Result<(), RpcFailure>` |
| function | `secure_turn_input` | L959 | `fn secure_turn_input(messages: &[ChatMessage]) -> Result<Vec<Value>, RpcFailure>` |
| function | `safe_image_data_url` | L1008 | `fn safe_image_data_url(value: &str) -> bool` |
| function | `secure_thread_params` | L1021 | `fn secure_thread_params(model: &str, workspace: &str) -> Value` |
| function | `secure_turn_params` | L1043 | `fn secure_turn_params(thread_id: &str, model: &str, reasoning_effort: Option<&str>, workspace: &str, input: Vec<Value>,) -> Value` |
| function | `validate_thread_settings_update` | L1066 | `fn validate_thread_settings_update(message: &Value, thread_id: &str, model: &str, reasoning_effort: Option<&str>, workspace: &Path,) -> Result<(), RpcFailure>` |
| function | `safe_reasoning_effort` | L1121 | `fn safe_reasoning_effort(value: &str) -> Result<Option<String>, RpcFailure>` |
| function | `require_secure_profile` | L1131 | `fn require_secure_profile(server: &mut AppServer, workspace: &str) -> Result<(), RpcFailure>` |
| function | `validate_thread_contract` | L1150 | `fn validate_thread_contract(result: &Value, model: &str, workspace: &Path,) -> Result<String, RpcFailure>` |
| function | `require_contract` | L1225 | `fn require_contract(valid: bool, field: &'static str) -> Result<(), RpcFailure>` |
| function | `paths_match` | L1234 | `fn paths_match(actual: &Path, expected: &Path) -> bool` |
| function | `matching_turn_params` | L1241 | `fn matching_turn_params(message: &'a Value, thread_id: &str, turn_id: Option<&str>,) -> Result<&'a Value, RpcFailure>` |
| function | `matching_completed_turn` | L1260 | `fn matching_completed_turn(message: &'a Value, thread_id: &str, turn_id: &str,) -> Result<&'a Value, RpcFailure>` |
| function | `interrupt_turn` | L1276 | `fn interrupt_turn(server: &mut AppServer, thread_id: &str, turn_id: Option<&str>) -> ()` |
| function | `nonnegative_u64` | L1286 | `fn nonnegative_u64(value: Option<&Value>) -> u64` |
| function | `ensure_workspace_empty` | L1293 | `fn ensure_workspace_empty(workspace: &Path) -> Result<(), RpcFailure>` |
| function | `parse_login_notification` | L1308 | `fn parse_login_notification(value: &Value, login_id: &str) -> Result<LoginProgress, RpcFailure>` |
| function | `parse_account_state` | L1325 | `fn parse_account_state(result: &Value) -> Result<AccountState, RpcFailure>` |
| function | `safe_plan_label` | L1346 | `fn safe_plan_label(value: &str) -> Option<String>` |
| function | `recv_until` | L1355 | `fn recv_until(receiver: &Receiver<Result<Value, RpcFailure>>, deadline: Instant,) -> Result<Value, RpcFailure>` |
| function | `allowed_signin_url` | L1370 | `fn allowed_signin_url(value: &str) -> bool` |
| function | `valid_user_code` | L1378 | `fn valid_user_code(value: &str) -> bool` |
| function | `isolated_paths` | L1386 | `fn isolated_paths() -> Result<(PathBuf, PathBuf), RpcFailure>` |
| function | `spawn_app_server` | L1394 | `fn spawn_app_server(executable: &Path, codex_home: &Path, workspace: &Path,) -> std::io::Result<Child>` |
| function | `classify_app_server_stderr` | L1434 | `fn classify_app_server_stderr(line: &str) -> Option<&'static str>` |
| function | `rpc_request` | L1449 | `fn rpc_request(id: u64, method: &str, params: Value) -> Value` |
| function | `codex_executable_candidates` | L1453 | `fn codex_executable_candidates() -> Vec<PathBuf>` |
| function | `find_existing_executable` | L1476 | `fn find_existing_executable(paths: impl IntoIterator<Item = &'a Path>) -> Option<PathBuf>` |
| function | `parses_non_secret_account_states` | L1491 | `fn parses_non_secret_account_states() -> ()` |
| function | `login_state_machine_ignores_other_ids_and_never_surfaces_error_text` | L1507 | `fn login_state_machine_ignores_other_ids_and_never_surfaces_error_text() -> ()` |
| function | `malformed_and_reauth_responses_fail_closed` | L1529 | `fn malformed_and_reauth_responses_fail_closed() -> ()` |
| function | `timeout_is_deterministic` | L1546 | `fn timeout_is_deterministic() -> ()` |
| function | `receiver_drop_on_terminal_done_is_still_a_cancellation` | L1555 | `fn receiver_drop_on_terminal_done_is_still_a_cancellation() -> ()` |
| function | `executable_detection_reports_absent_and_present` | L1564 | `fn executable_detection_reports_absent_and_present() -> ()` |
| function | `app_server_stderr_is_classified_without_forwarding_raw_details` | L1577 | `fn app_server_stderr_is_classified_without_forwarding_raw_details() -> ()` |
| function | `only_official_https_login_hosts_are_accepted` | L1590 | `fn only_official_https_login_hosts_are_accepted() -> ()` |
| function | `user_codes_are_bounded_ascii` | L1600 | `fn user_codes_are_bounded_ascii() -> ()` |
| function | `stable_json_rpc_requests_match_official_wire_shape` | L1608 | `fn stable_json_rpc_requests_match_official_wire_shape() -> ()` |
| function | `experimental_security_contract_is_explicit_and_model_pinned_twice` | L1627 | `fn experimental_security_contract_is_explicit_and_model_pinned_twice() -> ()` |
| function | `model_catalog_preserves_account_metadata_and_rejects_unsafe_ids` | L1691 | `fn model_catalog_preserves_account_metadata_and_rejects_unsafe_ids() -> ()` |
| function | `paginated_catalog_cursor_and_duplicate_exact_model_fail_closed` | L1730 | `fn paginated_catalog_cursor_and_duplicate_exact_model_fail_closed() -> ()` |
| function | `thread_contract_requires_exact_model_profile_workspace_and_no_network` | L1772 | `fn thread_contract_requires_exact_model_profile_workspace_and_no_network() -> ()` |
| function | `streaming_ids_are_exact_and_tool_items_are_denied` | L1833 | `fn streaming_ids_are_exact_and_tool_items_are_denied() -> ()` |
| function | `image_input_accepts_only_bounded_inline_user_images` | L1968 | `fn image_input_accepts_only_bounded_inline_user_images() -> ()` |
| function | `settings_update_for_explicit_effort_must_preserve_safe_contract` | L2000 | `fn settings_update_for_explicit_effort_must_preserve_safe_contract() -> ()` |
| function | `nonempty_workspace_fails_closed` | L2043 | `fn nonempty_workspace_fails_closed() -> ()` |
| function | `rate_limit_and_failures_never_echo_server_secrets` | L2054 | `fn rate_limit_and_failures_never_echo_server_secrets() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L171
- Spawns asynchronous thread/task at L182
