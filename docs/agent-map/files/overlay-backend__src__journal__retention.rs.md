---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7f56970bd1cf"
source_path: "overlay-backend/src/journal/retention.rs"
batch_id: "B08"
total_lines: 55
symbols_count: 2
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "f3881102edd23e3e0c96806eab0e7696ae113a162f2cc256e38eeb4a93e5ca12"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/journal/retention.rs`

- **Batch:** B08
- **Physical Lines:** 55
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `prune_old_sessions` | L8 | `fn prune_old_sessions(dir: &Path, keep: usize) -> Result<usize>` |
| function | `prune_old_sessions_with_size_cap` | L12 | `fn prune_old_sessions_with_size_cap(dir: &Path, keep: usize, max_bytes: u64) -> Result<usize>` |
