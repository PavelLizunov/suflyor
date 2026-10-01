---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1bbe4d0007b8"
source_path: "suflyor-tts/src/main.rs"
batch_id: "B11"
total_lines: 404
symbols_count: 12
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "52b73be9ae04ef95b016e2548ce47931d915fe671fdeebc40b72d8b5c9c59480"
source_matches_reconciliation_baseline: true
---

# File Map: `suflyor-tts/src/main.rs`

- **Batch:** B11
- **Physical Lines:** 404
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Cmd` | L37 | private |
| enum | `Message` | L48 | private |
| struct | `PlaybackSpeed` | L55 | private |

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `default` | L58 | `fn default() -> Self` |
| function | `set` | L64 | `fn set(&mut self, percent: i32) -> f32` |
| function | `factor` | L69 | `fn factor(self) -> f32` |
| function | `parse_cmd` | L81 | `fn parse_cmd(line: &str) -> Option<Cmd>` |
| function | `main` | L126 | `fn main() -> ()` |
| function | `worker` | L155 | `fn worker(rx: mpsc::Receiver<Message>, events: mpsc::Sender<Message>) -> ()` |
| function | `emit_playback_event` | L350 | `fn emit_playback_event(out: &mut impl Write, kind: &str, id: u64) -> ()` |
| function | `emit_playback_failure` | L355 | `fn emit_playback_failure(out: &mut impl Write, id: u64, reason: &str) -> ()` |
| function | `parses_bounded_seek_and_playback_speed` | L367 | `fn parses_bounded_seek_and_playback_speed() -> ()` |
| function | `playback_speed_is_remembered_for_the_next_player` | L379 | `fn playback_speed_is_remembered_for_the_next_player() -> ()` |
| function | `playback_done_is_consumed_once_and_stale_ids_are_ignored` | L387 | `fn playback_done_is_consumed_once_and_stale_ids_are_ignored() -> ()` |
| function | `parses_voice_cmd_rejects_path_traversal` | L395 | `fn parses_voice_cmd_rejects_path_traversal() -> ()` |

## Heuristic behavior and concurrency matches

- Instantiates IPC channel at L138
- Spawns asynchronous thread/task at L140
