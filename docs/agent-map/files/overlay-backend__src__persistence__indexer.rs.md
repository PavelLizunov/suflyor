---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2bdc0820cbe1"
source_path: "overlay-backend/src/persistence/indexer.rs"
batch_id: "B08"
total_lines: 449
symbols_count: 13
review_state: validated
---

# File Map: `overlay-backend/src/persistence/indexer.rs`

- **Batch:** B08
- **Physical Lines:** 449
- **Coverage:** 449/449 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `IndexStats` | L18 | pub |

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `index_journal_file` | L32 | `fn index_journal_file(store: &mut Store, path: &Path) -> Result<Option<Session>>` |
| function | `index_all` | L177 | `fn index_all(store: &mut Store, sessions_dir: &Path, skip_active: Option<&str>,) -> Result<IndexStats>` |
| function | `num` | L216 | `fn num(v: &Value, key: &str) -> Option<i64>` |
| function | `text` | L221 | `fn text(v: &Value, key: &str) -> String` |
| function | `flag` | L228 | `fn flag(v: &Value, key: &str) -> bool` |
| function | `write_jsonl` | L238 | `fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> std::path::PathBuf` |
| function | `indexes_a_completed_session_pairing_qa` | L248 | `fn indexes_a_completed_session_pairing_qa() -> ()` |
| function | `missing_session_stop_is_crashed` | L280 | `fn missing_session_stop_is_crashed() -> ()` |
| function | `corrupt_line_is_skipped_not_fatal` | L296 | `fn corrupt_line_is_skipped_not_fatal() -> ()` |
| function | `index_all_skips_already_indexed_and_active` | L314 | `fn index_all_skips_already_indexed_and_active() -> ()` |
| function | `index_all_on_missing_dir_is_empty_not_error` | L347 | `fn index_all_on_missing_dir_is_empty_not_error() -> ()` |
| function | `index_all_heals_a_crashed_row_once_a_stop_appears` | L357 | `fn index_all_heals_a_crashed_row_once_a_stop_appears() -> ()` |
| function | `index_all_skips_empty_journal_then_indexes_later_content` | L405 | `fn index_all_skips_empty_journal_then_indexes_later_content() -> ()` |
