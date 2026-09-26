---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_20dbd1d5e44a"
source_path: "overlay-backend/src/runtime.rs"
batch_id: "B06"
total_lines: 1908
symbols_count: 21
review_state: validated
---

# File Map: `overlay-backend/src/runtime.rs`

- **Batch:** B06
- **Physical Lines:** 1908
- **Coverage:** 1908/1908 lines (100%)

## Types & Structures (7)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `a` | L5 | private |
| struct | `ManagedPrepSession` | L431 | private |
| struct | `ReaskInputs` | L1150 | pub |
| struct | `ReaskOutcome` | L1175 | pub |
| struct | `ManualSpawnInputs` | L1440 | pub |
| struct | `ManualSpawnOutcome` | L1464 | pub |
| trait | `method` | L72 | private |

## Symbols & Routines (21)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `run_post_meeting_debrief` | L75 | `fn run_post_meeting_debrief(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, transcript: Vec<TranscriptLine>, session_id: String,) -> ()` |
| function | `spawn_debrief_notice` | L199 | `fn spawn_debrief_notice(events: &dyn RuntimeEvents, cfg: &SharedConfig, body: String) -> ()` |
| function | `run_meeting_summary` | L236 | `fn run_meeting_summary(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, transcript: Vec<TranscriptLine>, session_id: String, force: bool,) -> ()` |
| function | `retry_meeting_summary` | L388 | `fn retry_meeting_summary(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, session_id: String,) -> ()` |
| function | `start` | L443 | `fn start(cfg: &SharedConfig) -> anyhow::Result<Option<Self>>` |
| function | `restore` | L517 | `fn restore(mut self) -> ()` |
| function | `drop` | L552 | `fn drop(&mut self) -> ()` |
| function | `summary_complete` | L574 | `fn summary_complete(protocol: ai::AiProtocol, base_url: &str, bearer: &str, model: &str, messages: Vec<ai::ChatMessage>, max_tokens: u32, exclusive: bool,) -> anyhow::Result<String>` |
| function | `split_map_source` | L602 | `fn split_map_source(source: &str) -> Option<(String, String)>` |
| function | `ensure_map_parts_fit` | L620 | `fn ensure_map_parts_fit(cs: &mut Conspect, base_url: &str, bearer: &str, model: &str, context_tokens: Option<u32>,) -> bool` |
| function | `pack_reduce_batches` | L679 | `fn pack_reduce_batches(partials: Vec<String>, is_ru: bool, is_local: bool, base_url: &str, bearer: &str, model: &str, context_tokens: Option<u32>,) -> anyhow::Result<Vec<Vec<String>>>` |
| function | `reduce_summary_full` | L734 | `fn reduce_summary_full(mut partials: Vec<String>, is_ru: bool, is_local: bool, memory_ref: Option<&str>, protocol: ai::AiProtocol, base_url: &str, bearer: &str, model: &str, context_tokens: Option<u32>, exclusive: bool,) -> anyhow::Result<String>` |
| function | `finish_summary_from_conspect` | L813 | `fn finish_summary_from_conspect(events: &Arc<dyn RuntimeEvents>, cfg: &SharedConfig, cs: Conspect, tile_title: String, monitor_hint: MonitorHint, stealth: bool, ui_is_ru: bool,) -> ()` |
| function | `finish_summary_from_conspect_inner` | L856 | `fn finish_summary_from_conspect_inner(events: &Arc<dyn RuntimeEvents>, cfg: &SharedConfig, mut cs: Conspect, tile_title: String, monitor_hint: MonitorHint, stealth: bool, ui_is_ru: bool, exclusive: bool,) -> ()` |
| function | `summary_tile_title` | L1059 | `fn summary_tile_title(ui_is_ru: bool) -> String` |
| function | `monitor_hint_from` | L1068 | `fn monitor_hint_from(preferred_monitor: Option<&str>) -> MonitorHint` |
| function | `spawn_summary_tile` | L1078 | `fn spawn_summary_tile(events: &Arc<dyn RuntimeEvents>, tile_title: String, answer: String, monitor_hint: MonitorHint, stealth: bool, session_id: String,) -> ()` |
| function | `spawn_summary_error_tile` | L1107 | `fn spawn_summary_error_tile(events: &Arc<dyn RuntimeEvents>, tile_title: String, monitor_hint: MonitorHint, stealth: bool, ui_is_ru: bool, session_id: Option<String>,) -> ()` |
| function | `reask_last` | L1200 | `fn reask_last(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, inputs: ReaskInputs,) -> Option<ReaskOutcome>` |
| function | `manual_spawn_tile` | L1483 | `fn manual_spawn_tile(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, inputs: ManualSpawnInputs,) -> Option<ManualSpawnOutcome>` |
| function | `ask_stream_loop` | L1810 | `fn ask_stream_loop(events: Arc<dyn RuntimeEvents>, mut ai_rx: tokio::sync::mpsc::Receiver<ai::AiEvent>, model: String, purpose: &'static str, is_local: bool, sys_full: String, usr_full: String, journal: Option<Journal>, health: Arc<HealthSignals>, t0: std::time::Instant, cost_apply: CostApplyFn,) -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L1759
- Spawns asynchronous thread/task at L1773
- Spawns asynchronous thread/task at L1777
