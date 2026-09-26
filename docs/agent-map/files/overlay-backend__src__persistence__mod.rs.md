---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_f22b82ab5539"
source_path: "overlay-backend/src/persistence/mod.rs"
batch_id: "B08"
total_lines: 78
symbols_count: 3
review_state: validated
---

# File Map: `overlay-backend/src/persistence/mod.rs`

- **Batch:** B08
- **Physical Lines:** 78
- **Coverage:** 78/78 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `sessions_dir` | L44 | `fn sessions_dir() -> Option<PathBuf>` |
| function | `reindex_default` | L53 | `fn reindex_default(skip_active: Option<&str>) -> Result<IndexStats>` |
| function | `open_default_store` | L75 | `fn open_default_store() -> Result<Store>` |
