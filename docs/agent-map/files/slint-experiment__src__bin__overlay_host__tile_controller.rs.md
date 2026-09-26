---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8642212f8284"
source_path: "slint-experiment/src/bin/overlay_host/tile_controller.rs"
batch_id: "B03"
total_lines: 1013
symbols_count: 20
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_controller.rs`

- **Batch:** B03
- **Physical Lines:** 1013
- **Coverage:** 1013/1013 lines (100%)

## Types & Structures (8)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `GenGatedEvents` | L156 | private |
| struct | `PttStreamSink` | L202 | pub(crate) |
| struct | `PttSinkState` | L211 | private |
| struct | `OverlayBarBridge` | L378 | pub(crate) |
| struct | `StreamingTile` | L430 | pub(crate) |
| struct | `ConvoState` | L454 | pub(crate) |
| struct | `a` | L461 | private |
| struct | `SpawnTileRequest` | L462 | pub(crate) |

## Symbols & Routines (20)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `conversations_evict_keys` | L79 | `fn conversations_evict_keys(keys: &[i32], max: usize) -> Vec<i32>` |
| function | `cost_cap_notice_spec` | L90 | `fn cost_cap_notice_spec(payload: &serde_json::Value) -> Option<(TileSpec, TileKind)>` |
| function | `install_streaming_tile` | L119 | `fn install_streaming_tile(bridge: &Arc<OverlayBarBridge>, new_tile: StreamingTile,) -> u64` |
| function | `emit` | L163 | `fn emit(&self, channel: &str, payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L169 | `fn spawn_tile_full(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `gated_events` | L182 | `fn gated_events(bridge: &Arc<OverlayBarBridge>, inner: Arc<dyn RuntimeEvents>, generation: u64,) -> Arc<dyn RuntimeEvents>` |
| function | `new` | L218 | `fn new(bridge: Arc<OverlayBarBridge>, inner: Arc<dyn RuntimeEvents>, tile: slint::Weak<TileWindow>, convo_id: i32, prefix: String, request_messages: Vec<ai::ChatMessage>,) -> Self` |
| function | `emit` | L246 | `fn emit(&self, channel: &str, payload: serde_json::Value) -> ()` |
| function | `spawn_tile_full` | L359 | `fn spawn_tile_full(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `store_conversation` | L483 | `fn store_conversation(&self, convo_id: i32, state: ConvoState) -> ()` |
| function | `drop_conversation` | L501 | `fn drop_conversation(&self, convo_id: i32) -> ()` |
| function | `handle_cost_cap_hit` | L513 | `fn handle_cost_cap_hit(&self, payload: serde_json::Value) -> ()` |
| function | `handle_ai_event` | L537 | `fn handle_ai_event(&self, payload: serde_json::Value) -> ()` |
| function | `inc_ai_in_flight` | L718 | `fn inc_ai_in_flight(&self) -> ()` |
| function | `dec_ai_in_flight` | L735 | `fn dec_ai_in_flight(&self) -> ()` |
| function | `reset_ai_in_flight` | L760 | `fn reset_ai_in_flight(&self) -> ()` |
| function | `forward_event` | L773 | `fn forward_event(&self, channel: String, payload: serde_json::Value) -> ()` |
| function | `schedule_spawn_tile` | L963 | `fn schedule_spawn_tile(&self, spec: TileSpec, monitor: MonitorHint, stealth: bool, kind: TileKind,) -> Result<String, String>` |
| function | `cap_hit_maps_to_error_notice_tile_with_reason_as_answer` | L992 | `fn cap_hit_maps_to_error_notice_tile_with_reason_as_answer() -> ()` |
| function | `cap_hit_without_usable_reason_spawns_nothing` | L1008 | `fn cap_hit_without_usable_reason_spawns_nothing() -> ()` |
