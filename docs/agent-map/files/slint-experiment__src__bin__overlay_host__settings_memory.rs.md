---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_62c2b62709d8"
source_path: "slint-experiment/src/bin/overlay_host/settings_memory.rs"
batch_id: "B02"
total_lines: 391
symbols_count: 10
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_memory.rs`

- **Batch:** B02
- **Physical Lines:** 391
- **Coverage:** 391/391 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

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

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L140
- Spawns asynchronous thread/task at L167
- Spawns asynchronous thread/task at L231
- Spawns asynchronous thread/task at L293
- Spawns asynchronous thread/task at L314
