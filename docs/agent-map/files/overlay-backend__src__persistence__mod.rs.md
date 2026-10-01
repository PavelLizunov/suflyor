---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_f22b82ab5539"
source_path: "overlay-backend/src/persistence/mod.rs"
batch_id: "B08"
total_lines: 78
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "1e8063ff9682cfac09720ac4fb760ee3860ac13cb08245961746ff54975de959"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/persistence/mod.rs`

- **Batch:** B08
- **Physical Lines:** 78
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `sessions_dir` | L44 | `fn sessions_dir() -> Option<PathBuf>` |
| function | `reindex_default` | L53 | `fn reindex_default(skip_active: Option<&str>) -> Result<IndexStats>` |
| function | `open_default_store` | L75 | `fn open_default_store() -> Result<Store>` |
