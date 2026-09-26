---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c7cb4493cd3a"
source_path: "overlay-backend/src/events.rs"
batch_id: "B06"
total_lines: 384
symbols_count: 15
review_state: validated
---

# File Map: `overlay-backend/src/events.rs`

- **Batch:** B06
- **Physical Lines:** 384
- **Coverage:** 384/384 lines (100%)

## Types & Structures (9)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `TileKind` | L88 | pub |
| enum | `MonitorHint` | L160 | pub |
| struct | `TileSpec` | L57 | pub |
| struct | `Noop` | L182 | pub |
| struct | `RecordingSink` | L344 | private |
| trait | `RuntimeEvents` | L24 | pub |
| trait | `method` | L72 | private |
| trait | `method` | L86 | private |
| trait | `object` | L341 | private |

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `emit` | L29 | `fn emit(&self, channel: &str, payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L46 | `fn spawn_tile_full(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `as_journal_tag` | L138 | `fn as_journal_tag(&self) -> &'static str` |
| function | `emit` | L185 | `fn emit(&self, channel: &str, _payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L188 | `fn spawn_tile_full(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `noop` | L212 | `fn noop() -> Arc<dyn RuntimeEvents>` |
| function | `noop_emit_does_not_panic_on_arbitrary_channels` | L222 | `fn noop_emit_does_not_panic_on_arbitrary_channels() -> ()` |
| function | `noop_spawn_tile_full_returns_stable_id_per_kind_and_question_len` | L230 | `fn noop_spawn_tile_full_returns_stable_id_per_kind_and_question_len() -> ()` |
| function | `tile_kind_journal_tags_are_unique` | L253 | `fn tile_kind_journal_tags_are_unique() -> ()` |
| function | `noop_spawn_tile_full_encodes_kind_in_id` | L279 | `fn noop_spawn_tile_full_encodes_kind_in_id() -> ()` |
| function | `monitor_hint_named_carries_string_through_clone_and_debug` | L301 | `fn monitor_hint_named_carries_string_through_clone_and_debug() -> ()` |
| function | `noop_spawn_tile_full_does_not_panic_on_named_hint` | L319 | `fn noop_spawn_tile_full_does_not_panic_on_named_hint() -> ()` |
| function | `trait_object_forwards_spawn_tile_full_to_impl` | L340 | `fn trait_object_forwards_spawn_tile_full_to_impl() -> ()` |
| function | `emit` | L348 | `fn emit(&self, _channel: &str, _payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L349 | `fn spawn_tile_full(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
