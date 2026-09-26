---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1e6d97067940"
source_path: "slint-experiment/src/slint_events.rs"
batch_id: "B01"
total_lines: 254
symbols_count: 12
review_state: validated
---

# File Map: `slint-experiment/src/slint_events.rs`

- **Batch:** B01
- **Physical Lines:** 254
- **Coverage:** 254/254 lines (100%)

## Types & Structures (8)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `that` | L37 | private |
| struct | `SlintEvents` | L75 | pub |
| struct | `from` | L80 | private |
| struct | `RecordingBridge` | L128 | private |
| trait | `methods` | L9 | private |
| trait | `SlintUiBridge` | L46 | pub |
| trait | `impl` | L247 | private |
| trait | `shape` | L248 | private |

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `forward_event` | L50 | `fn forward_event(&self, channel: String, payload: serde_json::Value) -> ()` |
| function | `schedule_spawn_tile` | L63 | `fn schedule_spawn_tile(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `new` | L92 | `fn new(bridge: Arc<dyn SlintUiBridge>) -> Self` |
| function | `emit` | L98 | `fn emit(&self, channel: &str, payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L102 | `fn spawn_tile_full(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `forward_event` | L135 | `fn forward_event(&self, channel: String, payload: serde_json::Value) -> ()` |
| function | `schedule_spawn_tile` | L142 | `fn schedule_spawn_tile(&self, spec: TileSpec, _monitor: MonitorHint, _stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `lock_evs` | L161 | `fn lock_evs(bridge: &RecordingBridge,) -> std::sync::MutexGuard<'_, Vec<(String, serde_json::Value)>>` |
| function | `lock_spawns` | L170 | `fn lock_spawns(bridge: &RecordingBridge,) -> std::sync::MutexGuard<'_, Vec<(TileSpec, TileKind)>>` |
| function | `emit_forwards_channel_and_payload_to_bridge` | L180 | `fn emit_forwards_channel_and_payload_to_bridge() -> ()` |
| function | `spawn_tile_full_returns_label_and_records_spec` | L200 | `fn spawn_tile_full_returns_label_and_records_spec() -> ()` |
| function | `slint_events_is_a_runtime_events_trait_object` | L246 | `fn slint_events_is_a_runtime_events_trait_object() -> ()` |
