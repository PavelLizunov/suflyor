---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e3e78904112f"
source_path: "overlay-backend/src/session_admin.rs"
batch_id: "B08"
total_lines: 146
symbols_count: 6
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "b106022b166e16de97775fa74720b5392737fac6b093ed290bb84cec63865d0b"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/session_admin.rs`

- **Batch:** B08
- **Physical Lines:** 146
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `is_safe_id` | L19 | `fn is_safe_id(session_id: &str) -> bool` |
| function | `delete_session_files_in` | L39 | `fn delete_session_files_in(sessions_dir: &Path, recordings_dir: &Path, session_id: &str,) -> Result<()>` |
| function | `delete_session_everywhere` | L86 | `fn delete_session_everywhere(store: &mut Store, session_id: &str) -> Result<()>` |
| function | `rejects_unsafe_ids` | L108 | `fn rejects_unsafe_ids() -> ()` |
| function | `deletes_jsonl_and_recordings_then_idempotent` | L119 | `fn deletes_jsonl_and_recordings_then_idempotent() -> ()` |
| function | `missing_artifacts_are_ok` | L137 | `fn missing_artifacts_are_ok() -> ()` |
