---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_511e8d777368"
source_path: "overlay-backend/tests/archive_cycle.rs"
batch_id: "B09"
total_lines: 181
symbols_count: 5
review_state: validated
---

# File Map: `overlay-backend/tests/archive_cycle.rs`

- **Batch:** B09
- **Physical Lines:** 181
- **Coverage:** 181/181 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `write_journal` | L24 | `fn write_journal(dir: &Path, id: &str, lines: &[&str]) -> PathBuf` |
| function | `full_cycle_record_transcribe_journal_index_archive` | L34 | `fn full_cycle_record_transcribe_journal_index_archive() -> ()` |
| function | `session_finished_in_current_run_becomes_visible_after_stop_index` | L103 | `fn session_finished_in_current_run_becomes_visible_after_stop_index() -> ()` |
| function | `stop_index_replaces_partial_row_wholesale` | L136 | `fn stop_index_replaces_partial_row_wholesale() -> ()` |
| function | `missing_recordings_dir_is_a_graceful_error_for_re_stt` | L174 | `fn missing_recordings_dir_is_a_graceful_error_for_re_stt() -> ()` |
