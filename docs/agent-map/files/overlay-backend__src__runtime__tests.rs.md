---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_af12b658d7e9"
source_path: "overlay-backend/src/runtime/tests.rs"
batch_id: "B06"
total_lines: 1165
symbols_count: 48
review_state: validated
---

# File Map: `overlay-backend/src/runtime/tests.rs`

- **Batch:** B06
- **Physical Lines:** 1165
- **Coverage:** 1165/1165 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `CapturedTile` | L22 | private |

## Symbols & Routines (48)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `hermetic_empty_config` | L16 | `fn hermetic_empty_config() -> crate::config::SharedConfig` |
| function | `emit` | L25 | `fn emit(&self, _channel: &str, _payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L27 | `fn spawn_tile_full(&self, spec: TileSpec, _monitor: MonitorHint, _stealth: bool, _kind: TileKind,) -> Result<String, String>` |
| function | `run_post_meeting_debrief_with_noop_events_does_not_panic` | L44 | `fn run_post_meeting_debrief_with_noop_events_does_not_panic() -> ()` |
| function | `line` | L60 | `fn line(source: AudioSource, text: &str, ms: u64) -> TranscriptLine` |
| function | `summary_seed_preserves_full_input_for_exact_token_routing` | L69 | `fn summary_seed_preserves_full_input_for_exact_token_routing() -> ()` |
| function | `summary_gate_requires_two_lines` | L95 | `fn summary_gate_requires_two_lines() -> ()` |
| function | `summary_format_labels_channels_ru_en` | L106 | `fn summary_format_labels_channels_ru_en() -> ()` |
| function | `summary_truncate_passes_under_budget_unchanged` | L123 | `fn summary_truncate_passes_under_budget_unchanged() -> ()` |
| function | `summary_truncate_keeps_head_tail_and_marker` | L131 | `fn summary_truncate_keeps_head_tail_and_marker() -> ()` |
| function | `summary_truncate_handles_single_giant_line` | L145 | `fn summary_truncate_handles_single_giant_line() -> ()` |
| function | `summary_prompt_has_sections_and_honesty_rules_ru_en` | L158 | `fn summary_prompt_has_sections_and_honesty_rules_ru_en() -> ()` |
| function | `summary_seed_is_system_plus_user_with_transcript` | L192 | `fn summary_seed_is_system_plus_user_with_transcript() -> ()` |
| function | `summary_seed_matches_what_run_meeting_summary_would_send` | L217 | `fn summary_seed_matches_what_run_meeting_summary_would_send() -> ()` |
| function | `split_for_map_packs_lines_within_budget_and_preserves_words` | L237 | `fn split_for_map_packs_lines_within_budget_and_preserves_words() -> ()` |
| function | `split_for_map_word_wraps_one_giant_line` | L251 | `fn split_for_map_word_wraps_one_giant_line() -> ()` |
| function | `reduce_seed_preserves_every_conspectus_for_hierarchical_routing` | L267 | `fn reduce_seed_preserves_every_conspectus_for_hierarchical_routing() -> ()` |
| function | `exact_context_budget_keeps_the_required_reserve` | L287 | `fn exact_context_budget_keeps_the_required_reserve() -> ()` |
| function | `exact_budget_resplit_keeps_technical_identifiers_whole` | L293 | `fn exact_budget_resplit_keeps_technical_identifiers_whole() -> ()` |
| function | `reduce_seed_carries_rules_part_headers_and_memory_ref` | L301 | `fn reduce_seed_carries_rules_part_headers_and_memory_ref() -> ()` |
| function | `partial_prompt_is_no_fabrication_and_part_numbered` | L327 | `fn partial_prompt_is_no_fabrication_and_part_numbered() -> ()` |
| function | `summary_seed_memory_ref_is_decode_only_and_none_is_byte_identical` | L338 | `fn summary_seed_memory_ref_is_decode_only_and_none_is_byte_identical() -> ()` |
| function | `text_of` | L339 | `fn text_of(m: &ai::ChatMessage) -> String` |
| function | `run_meeting_summary_with_noop_events_does_not_panic` | L372 | `fn run_meeting_summary_with_noop_events_does_not_panic() -> ()` |
| function | `incomplete_map_keeps_every_gap_for_retry` | L389 | `fn incomplete_map_keeps_every_gap_for_retry() -> ()` |
| function | `prompt_always_contains_injection_guard` | L420 | `fn prompt_always_contains_injection_guard() -> ()` |
| function | `prompt_contains_garbage_and_offtopic_guards` | L448 | `fn prompt_contains_garbage_and_offtopic_guards() -> ()` |
| function | `prompt_contains_whisper_artifact_recovery_hints` | L472 | `fn prompt_contains_whisper_artifact_recovery_hints() -> ()` |
| function | `prompt_handles_long_transcript` | L492 | `fn prompt_handles_long_transcript() -> ()` |
| function | `prompt_handles_empty_transcript` | L513 | `fn prompt_handles_empty_transcript() -> ()` |
| function | `prompt_enforces_russian_response_when_configured` | L526 | `fn prompt_enforces_russian_response_when_configured() -> ()` |
| function | `prompt_offtopic_guard_present_with_empty_context` | L544 | `fn prompt_offtopic_guard_present_with_empty_context() -> ()` |
| function | `prompt_keyword_trigger_includes_keyword_and_line` | L558 | `fn prompt_keyword_trigger_includes_keyword_and_line() -> ()` |
| function | `prompt_live_coaching_adds_readaloud_rules` | L574 | `fn prompt_live_coaching_adds_readaloud_rules() -> ()` |
| function | `compact_prompt_is_materially_shorter_and_standard_unchanged` | L590 | `fn compact_prompt_is_materially_shorter_and_standard_unchanged() -> ()` |
| function | `compact_prompt_preserves_guards_context_language_coaching` | L617 | `fn compact_prompt_preserves_guards_context_language_coaching() -> ()` |
| function | `compact_prompt_grounds_probes_load_average_and_etcd` | L684 | `fn compact_prompt_grounds_probes_load_average_and_etcd() -> ()` |
| function | `reask_last_no_prior_qa_emits_error_and_returns_none` | L716 | `fn reask_last_no_prior_qa_emits_error_and_returns_none() -> ()` |
| function | `ask_stream_loop_processes_deltas_then_done_and_calls_cost_apply_once` | L734 | `fn ask_stream_loop_processes_deltas_then_done_and_calls_cost_apply_once() -> ()` |
| function | `ask_stream_loop_error_path_does_not_journal_partial_response` | L800 | `fn ask_stream_loop_error_path_does_not_journal_partial_response() -> ()` |
| function | `ask_stream_loop_journals_caller_supplied_purpose` | L864 | `fn ask_stream_loop_journals_caller_supplied_purpose() -> ()` |
| function | `ask_stream_loop_local_journals_zero_cost` | L926 | `fn ask_stream_loop_local_journals_zero_cost() -> ()` |
| function | `manual_spawn_tile_empty_transcript_returns_none` | L986 | `fn manual_spawn_tile_empty_transcript_returns_none() -> ()` |
| function | `manual_spawn_empty_notice_follows_ui_not_response_language` | L1004 | `fn manual_spawn_empty_notice_follows_ui_not_response_language() -> ()` |
| function | `manual_spawn_deep_lock_precedes_empty_transcript_for_managed_local_only` | L1036 | `fn manual_spawn_deep_lock_precedes_empty_transcript_for_managed_local_only() -> ()` |
| function | `deterministic_summary_and_debrief_chrome_follow_ui_language` | L1099 | `fn deterministic_summary_and_debrief_chrome_follow_ui_language() -> ()` |
| function | `manual_spawn_tile_over_budget_warns_but_proceeds` | L1121 | `fn manual_spawn_tile_over_budget_warns_but_proceeds() -> ()` |
| function | `reask_last_ai_error_returns_none_without_panic` | L1145 | `fn reask_last_ai_error_returns_none_without_panic() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L736
- Spawns asynchronous thread/task at L738
- Instantiates IPC channel at L802
- Spawns asynchronous thread/task at L803
- Instantiates IPC channel at L865
- Spawns asynchronous thread/task at L866
- Instantiates IPC channel at L937
- Spawns asynchronous thread/task at L938
