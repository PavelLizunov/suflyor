---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_0ea5ac552d43"
source_path: "overlay-backend/src/journal/time.rs"
batch_id: "B08"
total_lines: 92
symbols_count: 6
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "afe2235c748030c708c23b47623264a84aa133c360fcc4b5adba3b6332e9dd33"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/journal/time.rs`

- **Batch:** B08
- **Physical Lines:** 92
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `now_unix_ms` | L3 | `fn now_unix_ms() -> u128` |
| function | `chrono_like_stamp` | L11 | `fn chrono_like_stamp() -> String` |
| function | `format_msk_label` | L26 | `fn format_msk_label(unix_ms: i64) -> String` |
| function | `stamp_to_unix_secs` | L34 | `fn stamp_to_unix_secs(id: &str) -> Option<u64>` |
| function | `days_from_civil` | L63 | `fn days_from_civil(y: i64, m: u64, d: u64) -> i64` |
| function | `unix_to_ymdhms` | L74 | `fn unix_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32)` |
