---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_65a6d1836dd3"
source_path: "overlay-backend/src/journal/recovery.rs"
batch_id: "B08"
total_lines: 176
symbols_count: 5
review_state: validated
---

# File Map: `overlay-backend/src/journal/recovery.rs`

- **Batch:** B08
- **Physical Lines:** 176
- **Coverage:** 176/176 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `UnfinishedSession` | L11 | pub |

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `find_unfinished_session_in_default_dir` | L21 | `fn find_unfinished_session_in_default_dir() -> Option<UnfinishedSession>` |
| function | `find_unfinished_session` | L27 | `fn find_unfinished_session(journal_dir: &Path) -> Option<UnfinishedSession>` |
| function | `newest_jsonl` | L32 | `fn newest_jsonl(dir: &Path) -> Option<PathBuf>` |
| function | `parse_unfinished` | L50 | `fn parse_unfinished(path: &Path) -> Option<UnfinishedSession>` |
| function | `json_u64` | L165 | `fn json_u64(v: &serde_json::Value) -> Option<u64>` |
