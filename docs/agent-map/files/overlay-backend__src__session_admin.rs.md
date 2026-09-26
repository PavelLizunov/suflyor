---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e3e78904112f"
source_path: "overlay-backend/src/session_admin.rs"
batch_id: "B08"
total_lines: 146
symbols_count: 6
review_state: validated
---

# File Map: `overlay-backend/src/session_admin.rs`

- **Batch:** B08
- **Physical Lines:** 146
- **Coverage:** 146/146 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `is_safe_id` | L19 | `fn is_safe_id(session_id: &str) -> bool` |
| function | `delete_session_files_in` | L39 | `fn delete_session_files_in(sessions_dir: &Path, recordings_dir: &Path, session_id: &str,) -> Result<()>` |
| function | `delete_session_everywhere` | L86 | `fn delete_session_everywhere(store: &mut Store, session_id: &str) -> Result<()>` |
| function | `rejects_unsafe_ids` | L108 | `fn rejects_unsafe_ids() -> ()` |
| function | `deletes_jsonl_and_recordings_then_idempotent` | L119 | `fn deletes_jsonl_and_recordings_then_idempotent() -> ()` |
| function | `missing_artifacts_are_ok` | L137 | `fn missing_artifacts_are_ok() -> ()` |
