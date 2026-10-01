---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_62c2b62709d8"
source_path: "slint-experiment/src/bin/overlay_host/settings_memory.rs"
batch_id: "B02"
total_lines: 391
symbols_count: 10
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "251731b673fc5de138f010ab1aeba4e81c35d109a9ca393d1cba4e369ba56bb5"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_memory.rs`

- **Batch:** B02
- **Physical Lines:** 391
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `bump_refresh_gen` | L52 | `fn bump_refresh_gen() -> u64` |
| function | `is_latest_refresh` | L56 | `fn is_latest_refresh(gen: u64) -> bool` |
| function | `wire_memory` | L62 | `fn wire_memory(win: &SettingsWindow) -> ()` |
| function | `read_memory_lists` | L265 | `fn read_memory_lists() -> (Vec<MemoryCandidate>, Vec<MemoryItem>)` |
| function | `apply_memory_rows` | L281 | `fn apply_memory_rows(win: &SettingsWindow, cands: &[MemoryCandidate], items: &[MemoryItem]) -> ()` |
| function | `reload_memory` | L311 | `fn reload_memory(win: &SettingsWindow) -> ()` |
| function | `run_extract` | L329 | `fn run_extract() -> usize` |
| function | `candidate_row` | L358 | `fn candidate_row(c: &MemoryCandidate) -> MemoryRow` |
| function | `item_row` | L370 | `fn item_row(m: &MemoryItem) -> MemoryRow` |
| function | `kind_glyph` | L382 | `fn kind_glyph(kind: &str) -> &'static str` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L140
- Spawns asynchronous thread/task at L167
- Spawns asynchronous thread/task at L231
- Spawns asynchronous thread/task at L293
- Spawns asynchronous thread/task at L314
