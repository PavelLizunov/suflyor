---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_01b67db06b65"
source_path: "slint-experiment/src/bin/overlay_host_windows.rs"
batch_id: "B01"
total_lines: 4962
symbols_count: 8
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host_windows.rs`

- **Batch:** B01
- **Physical Lines:** 4962
- **Coverage:** 4962/4962 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `MicGuard` | L411 | pub(crate) |
| struct | `PttRec` | L2829 | private |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `to_md_blocks` | L366 | `fn to_md_blocks(md: &str) -> Vec<MarkdownBlock>` |
| function | `to_md_blocks_streaming` | L372 | `fn to_md_blocks_streaming(md: &str) -> Vec<MarkdownBlock>` |
| function | `map_md_blocks` | L376 | `fn map_md_blocks(blocks: Vec<markdown::Block>) -> Vec<MarkdownBlock>` |
| function | `drop` | L414 | `fn drop(&mut self) -> ()` |
| function | `try_acquire_mic` | L424 | `fn try_acquire_mic() -> Option<MicGuard>` |
| function | `main` | L474 | `fn main() -> Result<(), slint::PlatformError>` |
| function | `stop_session_and_maybe_debrief` | L4895 | `fn stop_session_and_maybe_debrief(runtime: SharedSlintRuntime, events: Arc<dyn RuntimeEvents>, cfg: config::SharedConfig, session_id: String, session_secs: u64, runtime_handle: &tokio::runtime::Handle,) -> ()` |
| function | `mic_guard_is_a_single_latch` | L4934 | `fn mic_guard_is_a_single_latch() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L539
- Instantiates IPC channel at L745
- Spawns asynchronous thread/task at L957
- Spawns asynchronous thread/task at L2178
- Instantiates IPC channel at L2835
- Instantiates IPC channel at L2838
- Spawns asynchronous thread/task at L2890
- Spawns asynchronous thread/task at L2966
- Spawns asynchronous thread/task at L4151
- Spawns asynchronous thread/task at L4225

## Configuration Access

- Configuration read at L610: `ai::set_prompt_cache(cfg.read().ai_prompt_cache);`
- Configuration read at L4229: `config.ai_provider == "mlx"`
- Configuration read at L4240: `config.suppress_tiles = target_listening;`

## Hotkey Mappings

- Hotkey binding/dispatch at L75: `// glob), and the extracted `register_hotkeys` / `RegisteredHotkeys` so the inli`
- Hotkey binding/dispatch at L78: `// ids `register_hotkeys` hands back.`
- Hotkey binding/dispatch at L2493: `// `hotkeys::register_hotkeys` (Phase 3, docs/overlay-host-modularization-plan`
- Hotkey binding/dispatch at L2495: `// dropping the `GlobalHotKeyManager` unregisters every hotkey. The returned`
- Hotkey binding/dispatch at L2516: `} = register_hotkeys();`
- Hotkey binding/dispatch at L2536: `while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {`
