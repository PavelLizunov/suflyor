---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c874e8c421ec"
source_path: "overlay-backend/src/summary_source.rs"
batch_id: "B08"
total_lines: 171
symbols_count: 7
review_state: validated
---

# File Map: `overlay-backend/src/summary_source.rs`

- **Batch:** B08
- **Physical Lines:** 171
- **Coverage:** 171/171 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `the` | L52 | private |

## Symbols & Routines (7)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `is_plain_id` | L19 | `fn is_plain_id(session_id: &str) -> bool` |
| function | `from_catalog` | L29 | `fn from_catalog(store: &Store, session_id: &str) -> Option<Vec<TranscriptLine>>` |
| function | `from_jsonl_prompts` | L56 | `fn from_jsonl_prompts(session_id: &str) -> Option<Vec<TranscriptLine>>` |
| function | `from_jsonl_prompts_in` | L63 | `fn from_jsonl_prompts_in(dir: &Path, session_id: &str) -> Option<Vec<TranscriptLine>>` |
| function | `sess` | L103 | `fn sess(id: &str) -> Session` |
| function | `from_catalog_maps_utterances_else_none` | L119 | `fn from_catalog_maps_utterances_else_none() -> ()` |
| function | `from_jsonl_dedups_overlapping_windows_and_guards` | L148 | `fn from_jsonl_dedups_overlapping_windows_and_guards() -> ()` |
