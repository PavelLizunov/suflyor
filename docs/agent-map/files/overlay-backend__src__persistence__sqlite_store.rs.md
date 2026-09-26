---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1e9d2e9d5f9e"
source_path: "overlay-backend/src/persistence/sqlite_store.rs"
batch_id: "B08"
total_lines: 1403
symbols_count: 65
review_state: validated
---

# File Map: `overlay-backend/src/persistence/sqlite_store.rs`

- **Batch:** B08
- **Physical Lines:** 1403
- **Coverage:** 1403/1403 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Store` | L20 | pub |

## Symbols & Routines (65)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `default_path` | L29 | `fn default_path() -> Option<PathBuf>` |
| function | `open` | L36 | `fn open(path: &Path) -> Result<Self>` |
| function | `open_in_memory` | L72 | `fn open_in_memory() -> Result<Self>` |
| function | `replace_session` | L84 | `fn replace_session(&mut self, session: &Session, utterances: &[Utterance], ai_turns: &[AiTurn],) -> Result<()>` |
| function | `delete_session` | L162 | `fn delete_session(&mut self, session_id: &str) -> Result<()>` |
| function | `list_sessions` | L182 | `fn list_sessions(&self) -> Result<Vec<Session>>` |
| function | `get_session` | L202 | `fn get_session(&self, id: &str) -> Result<Option<Session>>` |
| function | `count_utterances` | L222 | `fn count_utterances(&self, session_id: &str) -> Result<i64>` |
| function | `count_ai_turns` | L233 | `fn count_ai_turns(&self, session_id: &str) -> Result<i64>` |
| function | `get_diarization` | L245 | `fn get_diarization(&self, session_id: &str) -> Result<Option<Diarization>>` |
| function | `put_diarization` | L285 | `fn put_diarization(&self, d: &Diarization) -> Result<()>` |
| function | `rename_speaker` | L309 | `fn rename_speaker(&self, session_id: &str, speaker: i32, name: &str) -> Result<()>` |
| function | `backfill_session_models` | L333 | `fn backfill_session_models(&self) -> Result<usize>` |
| function | `finalized_session_ids` | L354 | `fn finalized_session_ids(&self) -> Result<std::collections::HashSet<String>>` |
| function | `session_utterances` | L370 | `fn session_utterances(&self, session_id: &str) -> Result<Vec<Utterance>>` |
| function | `session_ai_turns` | L389 | `fn session_ai_turns(&self, session_id: &str) -> Result<Vec<AiTurn>>` |
| function | `search` | L411 | `fn search(&self, query: &str, limit: i64) -> Result<Vec<SearchHit>>` |
| function | `insert_candidate` | L446 | `fn insert_candidate(&mut self, c: &NewMemoryCandidate, created_at_ms: i64) -> Result<i64>` |
| function | `list_candidates` | L471 | `fn list_candidates(&self, profile_id: &str, status: &str, limit: i64,) -> Result<Vec<MemoryCandidate>>` |
| function | `candidate_texts` | L500 | `fn candidate_texts(&self, profile_id: &str) -> Result<Vec<String>>` |
| function | `count_candidates` | L516 | `fn count_candidates(&self, profile_id: &str, status: &str) -> Result<i64>` |
| function | `set_candidate_status` | L528 | `fn set_candidate_status(&mut self, id: i64, status: &str) -> Result<()>` |
| function | `update_candidate_text` | L539 | `fn update_candidate_text(&mut self, id: i64, text: &str) -> Result<()>` |
| function | `approve_candidate` | L552 | `fn approve_candidate(&mut self, candidate_id: i64, approved_at_ms: i64) -> Result<i64>` |
| function | `insert_memory_item` | L580 | `fn insert_memory_item(&mut self, m: &NewMemoryItem, approved_at_ms: i64) -> Result<i64>` |
| function | `list_memory_items` | L607 | `fn list_memory_items(&self, profile_id: &str, include_archived: bool, limit: i64,) -> Result<Vec<MemoryItem>>` |
| function | `update_memory_item_text` | L641 | `fn update_memory_item_text(&mut self, id: i64, text: &str) -> Result<()>` |
| function | `restore_memory_item_source` | L655 | `fn restore_memory_item_source(&mut self, id: i64) -> Result<bool>` |
| function | `archive_memory_item` | L671 | `fn archive_memory_item(&mut self, id: i64, archived_at_ms: i64) -> Result<()>` |
| function | `delete_memory_item` | L682 | `fn delete_memory_item(&mut self, id: i64) -> Result<()>` |
| function | `row_to_session` | L691 | `fn row_to_session(row: &Row) -> rusqlite::Result<Session>` |
| function | `row_to_utterance` | L707 | `fn row_to_utterance(row: &Row) -> rusqlite::Result<Utterance>` |
| function | `row_to_ai_turn` | L718 | `fn row_to_ai_turn(row: &Row) -> rusqlite::Result<AiTurn>` |
| function | `row_to_candidate` | L732 | `fn row_to_candidate(row: &Row) -> rusqlite::Result<MemoryCandidate>` |
| function | `row_to_item` | L746 | `fn row_to_item(row: &Row) -> rusqlite::Result<MemoryItem>` |
| function | `sample_session` | L767 | `fn sample_session(id: &str) -> Session` |
| function | `utt` | L782 | `fn utt(id: &str, ms: i64, source: &str, text: &str) -> Utterance` |
| function | `delete_session_removes_row_children_and_search` | L793 | `fn delete_session_removes_row_children_and_search() -> ()` |
| function | `open_in_memory_migrates_to_latest` | L823 | `fn open_in_memory_migrates_to_latest() -> ()` |
| function | `replace_session_round_trips` | L833 | `fn replace_session_round_trips() -> ()` |
| function | `utterance_audio_ms_round_trips` | L859 | `fn utterance_audio_ms_round_trips() -> ()` |
| function | `backfill_sets_headline_to_turn_mode` | L879 | `fn backfill_sets_headline_to_turn_mode() -> ()` |
| function | `backfill_nulls_turnless_sessions` | L917 | `fn backfill_nulls_turnless_sessions() -> ()` |
| function | `backfill_mode_wins_and_ignores_empty` | L930 | `fn backfill_mode_wins_and_ignores_empty() -> ()` |
| function | `backfill_tie_breaks_toward_most_recent` | L957 | `fn backfill_tie_breaks_toward_most_recent() -> ()` |
| function | `reindex_is_idempotent_no_duplicates` | L980 | `fn reindex_is_idempotent_no_duplicates() -> ()` |
| function | `list_sessions_orders_newest_first` | L992 | `fn list_sessions_orders_newest_first() -> ()` |
| function | `get_missing_session_is_none` | L1010 | `fn get_missing_session_is_none() -> ()` |
| function | `fts_search_finds_question_answer_and_utterance` | L1016 | `fn fts_search_finds_question_answer_and_utterance() -> ()` |
| function | `fts_reindex_does_not_duplicate_hits` | L1050 | `fn fts_reindex_does_not_duplicate_hits() -> ()` |
| function | `fts_search_matches_russian` | L1063 | `fn fts_search_matches_russian() -> ()` |
| function | `new_cand` | L1075 | `fn new_cand(text: &str) -> NewMemoryCandidate` |
| function | `latest_migration_version_is_6` | L1086 | `fn latest_migration_version_is_6() -> ()` |
| function | `candidate_insert_then_list_pending` | L1099 | `fn candidate_insert_then_list_pending() -> ()` |
| function | `list_candidates_respects_limit_newest_first` | L1113 | `fn list_candidates_respects_limit_newest_first() -> ()` |
| function | `reject_candidate_leaves_no_item` | L1129 | `fn reject_candidate_leaves_no_item() -> ()` |
| function | `approve_candidate_mints_item_and_marks_approved` | L1142 | `fn approve_candidate_mints_item_and_marks_approved() -> ()` |
| function | `approve_candidate_twice_mints_only_one_item` | L1161 | `fn approve_candidate_twice_mints_only_one_item() -> ()` |
| function | `approve_missing_candidate_errs` | L1175 | `fn approve_missing_candidate_errs() -> ()` |
| function | `edit_candidate_then_approve_keeps_edit` | L1186 | `fn edit_candidate_then_approve_keeps_edit() -> ()` |
| function | `archive_hides_from_active_but_kept` | L1196 | `fn archive_hides_from_active_but_kept() -> ()` |
| function | `manual_item_insert_then_hard_delete` | L1211 | `fn manual_item_insert_then_hard_delete() -> ()` |
| function | `normalization_columns_round_trip_and_manual_edit` | L1239 | `fn normalization_columns_round_trip_and_manual_edit() -> ()` |
| function | `restore_memory_item_source_is_explicit_and_lossless` | L1270 | `fn restore_memory_item_source_is_explicit_and_lossless() -> ()` |
| function | `diarization_round_trips_and_rename_updates_names` | L1344 | `fn diarization_round_trips_and_rename_updates_names() -> ()` |
