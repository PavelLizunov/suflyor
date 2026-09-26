---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e27298c7e609"
source_path: "overlay-backend/src/journal/tests.rs"
batch_id: "B08"
total_lines: 1076
symbols_count: 55
review_state: validated
---

# File Map: `overlay-backend/src/journal/tests.rs`

- **Batch:** B08
- **Physical Lines:** 1076
- **Coverage:** 1076/1076 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SessionStartOwned` | L759 | private |

## Symbols & Routines (55)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `unix_to_ymdhms_known_dates` | L13 | `fn unix_to_ymdhms_known_dates() -> ()` |
| function | `format_msk_label_shifts_utc_plus_three` | L21 | `fn format_msk_label_shifts_utc_plus_three() -> ()` |
| function | `stamp_to_unix_secs_round_trips_chrono_like_stamp` | L37 | `fn stamp_to_unix_secs_round_trips_chrono_like_stamp() -> ()` |
| function | `stamp_format_is_sortable` | L51 | `fn stamp_format_is_sortable() -> ()` |
| function | `event_serializes_with_kind_tag` | L59 | `fn event_serializes_with_kind_tag() -> ()` |
| function | `default_journal_write_is_noop` | L72 | `fn default_journal_write_is_noop() -> ()` |
| function | `open_journal_at` | L84 | `fn open_journal_at(path: &Path) -> Journal` |
| function | `shutdown_drains_burst_and_terminal_stop_durably` | L104 | `fn shutdown_drains_burst_and_terminal_stop_durably() -> ()` |
| function | `shutdown_is_idempotent_and_rejects_later_writes` | L138 | `fn shutdown_is_idempotent_and_rejects_later_writes() -> ()` |
| function | `open_session_creates_writable_file` | L175 | `fn open_session_creates_writable_file() -> ()` |
| function | `make_jsonl_file` | L189 | `fn make_jsonl_file(dir: &Path, name: &str) -> PathBuf` |
| function | `prune_keeps_newest_n_files` | L196 | `fn prune_keeps_newest_n_files() -> ()` |
| function | `prune_keep_larger_than_count_is_noop` | L221 | `fn prune_keep_larger_than_count_is_noop() -> ()` |
| function | `prune_ignores_non_jsonl_files` | L233 | `fn prune_ignores_non_jsonl_files() -> ()` |
| function | `prune_keep_zero_deletes_all_jsonl` | L255 | `fn prune_keep_zero_deletes_all_jsonl() -> ()` |
| function | `prune_empty_dir_returns_zero` | L267 | `fn prune_empty_dir_returns_zero() -> ()` |
| function | `v015_unlimited_spelling_deletes_nothing` | L276 | `fn v015_unlimited_spelling_deletes_nothing() -> ()` |
| function | `make_jsonl_files` | L296 | `fn make_jsonl_files(dir: &Path, count: usize, size_bytes: usize) -> Vec<PathBuf>` |
| function | `size_cap_zero_disables_size_based_prune` | L310 | `fn size_cap_zero_disables_size_based_prune() -> ()` |
| function | `size_cap_under_budget_no_op` | L322 | `fn size_cap_under_budget_no_op() -> ()` |
| function | `size_cap_evicts_oldest_first` | L333 | `fn size_cap_evicts_oldest_first() -> ()` |
| function | `size_cap_combines_with_count_prune` | L354 | `fn size_cap_combines_with_count_prune() -> ()` |
| function | `size_cap_exactly_at_budget_no_op` | L368 | `fn size_cap_exactly_at_budget_no_op() -> ()` |
| function | `bump_transcript_lines_per_source` | L382 | `fn bump_transcript_lines_per_source() -> ()` |
| function | `bump_transcript_unknown_source_ignored` | L416 | `fn bump_transcript_unknown_source_ignored() -> ()` |
| function | `bump_detector_decision_split_by_triggered` | L432 | `fn bump_detector_decision_split_by_triggered() -> ()` |
| function | `bump_ai_response_accumulates_cost_microcents` | L466 | `fn bump_ai_response_accumulates_cost_microcents() -> ()` |
| function | `bump_ai_response_cost_saturates_no_panic` | L499 | `fn bump_ai_response_cost_saturates_no_panic() -> ()` |
| function | `bump_session_meta_events_do_not_count` | L525 | `fn bump_session_meta_events_do_not_count() -> ()` |
| function | `bump_full_event_mix_aggregates_correctly` | L564 | `fn bump_full_event_mix_aggregates_correctly() -> ()` |
| function | `snapshot_counters_returns_independent_clone` | L665 | `fn snapshot_counters_returns_independent_clone() -> ()` |
| function | `session_summary_serializes_with_kind_tag` | L692 | `fn session_summary_serializes_with_kind_tag() -> ()` |
| function | `session_start_normal_serializes_without_recovery_field` | L717 | `fn session_start_normal_serializes_without_recovery_field() -> ()` |
| function | `session_start_recovered_serializes_with_recovery_field` | L739 | `fn session_start_recovered_serializes_with_recovery_field() -> ()` |
| function | `old_session_start_line_deserializes_to_none` | L765 | `fn old_session_start_line_deserializes_to_none() -> ()` |
| function | `new_session_start_line_deserializes_id` | L773 | `fn new_session_start_line_deserializes_id() -> ()` |
| function | `start_line` | L789 | `fn start_line(age_ms: u64) -> String` |
| function | `transcript_line` | L796 | `fn transcript_line(source: &str, text: &str) -> String` |
| function | `ai_request_line` | L800 | `fn ai_request_line(user_prompt: &str) -> String` |
| function | `ai_response_line` | L806 | `fn ai_response_line(text: &str) -> String` |
| function | `write_jsonl` | L812 | `fn write_jsonl(dir: &Path, name: &str, lines: &[String]) -> PathBuf` |
| function | `fresh_dir` | L818 | `fn fresh_dir(tag: &str) -> PathBuf` |
| function | `graceful_stop_returns_none` | L825 | `fn graceful_stop_returns_none() -> ()` |
| function | `graceful_summary_returns_none` | L841 | `fn graceful_summary_returns_none() -> ()` |
| function | `crash_returns_some_with_last_lines_and_qa` | L859 | `fn crash_returns_some_with_last_lines_and_qa() -> ()` |
| function | `truncated_tail_parses_rest_no_panic` | L892 | `fn truncated_tail_parses_rest_no_panic() -> ()` |
| function | `stale_unfinished_returns_none` | L921 | `fn stale_unfinished_returns_none() -> ()` |
| function | `empty_dir_returns_none` | L937 | `fn empty_dir_returns_none() -> ()` |
| function | `missing_dir_returns_none_no_panic` | L944 | `fn missing_dir_returns_none_no_panic() -> ()` |
| function | `only_start_no_lines_returns_some_with_empty_context` | L951 | `fn only_start_no_lines_returns_some_with_empty_context() -> ()` |
| function | `no_start_at_all_returns_none` | L964 | `fn no_start_at_all_returns_none() -> ()` |
| function | `newest_file_is_chosen` | L977 | `fn newest_file_is_chosen() -> ()` |
| function | `last_qa_uses_the_most_recent_pair` | L1005 | `fn last_qa_uses_the_most_recent_pair() -> ()` |
| function | `last_lines_capped_at_recovery_limit` | L1028 | `fn last_lines_capped_at_recovery_limit() -> ()` |
| function | `append_bookmark_creates_file_with_header_then_appends_entries` | L1043 | `fn append_bookmark_creates_file_with_header_then_appends_entries() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L90
