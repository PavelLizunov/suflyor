---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7f56970bd1cf"
source_path: "overlay-backend/src/journal/retention.rs"
batch_id: "B08"
total_lines: 55
symbols_count: 2
review_state: validated
---

# File Map: `overlay-backend/src/journal/retention.rs`

- **Batch:** B08
- **Physical Lines:** 55
- **Coverage:** 55/55 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `prune_old_sessions` | L8 | `fn prune_old_sessions(dir: &Path, keep: usize) -> Result<usize>` |
| function | `prune_old_sessions_with_size_cap` | L12 | `fn prune_old_sessions_with_size_cap(dir: &Path, keep: usize, max_bytes: u64) -> Result<usize>` |
