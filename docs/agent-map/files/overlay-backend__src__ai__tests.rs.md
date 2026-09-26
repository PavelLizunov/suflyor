---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ab9ab44fb491"
source_path: "overlay-backend/src/ai/tests.rs"
batch_id: "B06"
total_lines: 707
symbols_count: 38
review_state: validated
---

# File Map: `overlay-backend/src/ai/tests.rs`

- **Batch:** B06
- **Physical Lines:** 707
- **Coverage:** 707/707 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `CapturedRequest` | L424 | private |

## Symbols & Routines (38)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `managed_mlx_intent_is_exact_and_does_not_capture_external_local_servers` | L9 | `fn managed_mlx_intent_is_exact_and_does_not_capture_external_local_servers() -> ()` |
| function | `managed_gemma_uses_the_handoff_sampler_without_forced_seed` | L30 | `fn managed_gemma_uses_the_handoff_sampler_without_forced_seed() -> ()` |
| function | `queued_stream_stops_when_receiver_is_dropped` | L48 | `fn queued_stream_stops_when_receiver_is_dropped() -> ()` |
| function | `force_no_think_overrides_global_toggle` | L86 | `fn force_no_think_overrides_global_toggle() -> ()` |
| function | `thinking_disabled` | L87 | `fn thinking_disabled(body: &Value) -> bool` |
| function | `drain_returns_empty_when_no_complete_frame` | L120 | `fn drain_returns_empty_when_no_complete_frame() -> ()` |
| function | `drain_splits_at_double_newline` | L128 | `fn drain_splits_at_double_newline() -> ()` |
| function | `drain_does_not_panic_when_utf8_split_across_chunks` | L140 | `fn drain_does_not_panic_when_utf8_split_across_chunks() -> ()` |
| function | `drain_handles_multiple_frames_in_one_chunk` | L167 | `fn drain_handles_multiple_frames_in_one_chunk() -> ()` |
| function | `drain_normalizes_crlf_sse_frames` | L175 | `fn drain_normalizes_crlf_sse_frames() -> ()` |
| function | `build_request_always_includes_system_prompt` | L188 | `fn build_request_always_includes_system_prompt() -> ()` |
| function | `build_request_injects_kb_reference_for_named_term` | L203 | `fn build_request_injects_kb_reference_for_named_term() -> ()` |
| function | `cost_microcents_haiku_known_value` | L236 | `fn cost_microcents_haiku_known_value() -> ()` |
| function | `cost_microcents_sonnet_pricing` | L246 | `fn cost_microcents_sonnet_pricing() -> ()` |
| function | `gpt_5_2_pricing_and_endpoint_debug_are_safe` | L255 | `fn gpt_5_2_pricing_and_endpoint_debug_are_safe() -> ()` |
| function | `cost_unknown_model_defaults_to_sonnet` | L284 | `fn cost_unknown_model_defaults_to_sonnet() -> ()` |
| function | `cost_zero_tokens_is_zero` | L295 | `fn cost_zero_tokens_is_zero() -> ()` |
| function | `microcents_to_usd_boundaries` | L304 | `fn microcents_to_usd_boundaries() -> ()` |
| function | `cost_saturating_no_overflow` | L313 | `fn cost_saturating_no_overflow() -> ()` |
| function | `permanent_error_400_no_retry` | L322 | `fn permanent_error_400_no_retry() -> ()` |
| function | `permanent_error_auth_no_retry` | L329 | `fn permanent_error_auth_no_retry() -> ()` |
| function | `permanent_error_404_no_retry` | L337 | `fn permanent_error_404_no_retry() -> ()` |
| function | `permanent_error_413_no_retry` | L344 | `fn permanent_error_413_no_retry() -> ()` |
| function | `transient_error_5xx_retries` | L350 | `fn transient_error_5xx_retries() -> ()` |
| function | `transient_error_429_retries` | L360 | `fn transient_error_429_retries() -> ()` |
| function | `transient_network_errors_retry` | L367 | `fn transient_network_errors_retry() -> ()` |
| function | `empty_error_does_not_match_permanent` | L376 | `fn empty_error_does_not_match_permanent() -> ()` |
| function | `build_request_attaches_screenshot_as_image_part` | L384 | `fn build_request_attaches_screenshot_as_image_part() -> ()` |
| function | `serve_one_completion` | L406 | `fn serve_one_completion(body: &'static str) -> String` |
| function | `serve_one_capture` | L430 | `fn serve_one_capture(response_body: &'static str, content_type: &'static str,) -> (String, std::sync::mpsc::Receiver<CapturedRequest>)` |
| function | `header` | L464 | `fn header(captured: &'a CapturedRequest, name: &str) -> Option<&'a str>` |
| function | `direct_openai_uses_responses_contract` | L473 | `fn direct_openai_uses_responses_contract() -> ()` |
| function | `direct_anthropic_uses_messages_contract_and_headers` | L510 | `fn direct_anthropic_uses_messages_contract_and_headers() -> ()` |
| function | `openai_finish_reason_is_terminal_without_optional_metrics` | L545 | `fn openai_finish_reason_is_terminal_without_optional_metrics() -> ()` |
| function | `native_streams_emit_delta_and_terminal_event` | L580 | `fn native_streams_emit_delta_and_terminal_event() -> ()` |
| function | `complete_with_usage_surfaces_provider_length_finish_reason` | L619 | `fn complete_with_usage_surfaces_provider_length_finish_reason() -> ()` |
| function | `complete_with_usage_defaults_finish_reason_to_stop_when_absent` | L638 | `fn complete_with_usage_defaults_finish_reason_to_stop_when_absent() -> ()` |
| function | `deep_lock_guard_refuses_every_managed_sender` | L654 | `fn deep_lock_guard_refuses_every_managed_sender() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L409
- Spawns asynchronous thread/task at L437
- Spawns asynchronous thread/task at L549
