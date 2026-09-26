---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2f48fcaef3d7"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows/archive.rs"
batch_id: "B03"
total_lines: 1455
symbols_count: 26
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows/archive.rs`

- **Batch:** B03
- **Physical Lines:** 1455
- **Coverage:** 1455/1455 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (26)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `open_archive` | L32 | `fn open_archive(archive_ref: &Rc<RefCell<Option<ArchiveWindow>>>, transcript_slot: &Rc<RefCell<Option<TranscriptWindow>>>, tiles_ref: &TileWindows, state: &slint_replay::app_state::SharedState, weak_overlay: &slint::Weak<OverlayBarWindow>, cfg: &overlay_backend::config::SharedConfig, events: &Arc<dyn RuntimeEvents>, rt_handle: &tokio::runtime::Handle, slint_rt: &SharedSlintRuntime,) -> ()` |
| function | `spawn_content_tile` | L803 | `fn spawn_content_tile(title: &str, source_label: &str, body_md: &str, tiles: &TileWindows, state: &slint_replay::app_state::SharedState, weak_overlay: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `archive_row_at` | L883 | `fn archive_row_at(model: &slint::ModelRc<ArchiveRow>, idx: i32) -> Option<ArchiveRow>` |
| function | `fts_query` | L897 | `fn fts_query(raw: &str) -> String` |
| function | `pretty_session_label` | L909 | `fn pretty_session_label(id: &str) -> String` |
| function | `archive_time_label` | L925 | `fn archive_time_label(started_at_ms: Option<i64>, id: &str) -> String` |
| function | `status_label` | L940 | `fn status_label(status: &str, ru: bool) -> &'static str` |
| function | `recording_ids_snapshot` | L966 | `fn recording_ids_snapshot() -> std::collections::HashSet<String>` |
| function | `session_title` | L981 | `fn session_title(started_at_ms: Option<i64>, id: &str) -> String` |
| function | `session_to_row` | L991 | `fn session_to_row(s: &Session, recordings: &std::collections::HashSet<String>, conspects: &std::collections::HashSet<String>, debriefs: &std::collections::HashSet<String>, ru: bool,) -> ArchiveRow` |
| function | `hit_to_row` | L1055 | `fn hit_to_row(h: &SearchHit, recordings: &std::collections::HashSet<String>, conspects: &std::collections::HashSet<String>, debriefs: &std::collections::HashSet<String>, ru: bool,) -> ArchiveRow` |
| function | `build_session_markdown` | L1119 | `fn build_session_markdown(session: Option<&Session>, utterances: &[Utterance], ai_turns: &[AiTurn],) -> String` |
| function | `pretty_label_parses_stem` | L1190 | `fn pretty_label_parses_stem() -> ()` |
| function | `pretty_label_falls_back_on_odd_id` | L1198 | `fn pretty_label_falls_back_on_odd_id() -> ()` |
| function | `archive_time_label_prefers_started_at_then_id_then_raw` | L1204 | `fn archive_time_label_prefers_started_at_then_id_then_raw() -> ()` |
| function | `fts_query_prefixes_tokens_and_drops_punctuation` | L1225 | `fn fts_query_prefixes_tokens_and_drops_punctuation() -> ()` |
| function | `sample_session` | L1233 | `fn sample_session() -> Session` |
| function | `no_recordings` | L1249 | `fn no_recordings() -> std::collections::HashSet<String>` |
| function | `session_row_uses_localized_human_counts` | L1254 | `fn session_row_uses_localized_human_counts() -> ()` |
| function | `session_row_never_shows_raw_metadata_or_state` | L1290 | `fn session_row_never_shows_raw_metadata_or_state() -> ()` |
| function | `session_row_flags_abnormal_status_with_a_localized_word` | L1307 | `fn session_row_flags_abnormal_status_with_a_localized_word() -> ()` |
| function | `session_row_shows_cost_when_nonzero` | L1341 | `fn session_row_shows_cost_when_nonzero() -> ()` |
| function | `hit_row_tags_kind_and_caps_snippet` | L1355 | `fn hit_row_tags_kind_and_caps_snippet() -> ()` |
| function | `session_markdown_has_transcript_and_qa` | L1399 | `fn session_markdown_has_transcript_and_qa() -> ()` |
| function | `session_markdown_transcript_has_timecodes_and_ru_labels` | L1424 | `fn session_markdown_transcript_has_timecodes_and_ru_labels() -> ()` |
| function | `session_markdown_empty_shows_not_saved_notice` | L1451 | `fn session_markdown_empty_shows_not_saved_notice() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L115
