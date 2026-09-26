---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3cfcce409a86"
source_path: "slint-experiment/src/bin/overlay_host/tile_ask.rs"
batch_id: "B03"
total_lines: 688
symbols_count: 4
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_ask.rs`

- **Batch:** B03
- **Physical Lines:** 688
- **Coverage:** 688/688 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `fire_f3_reask` | L62 | `fn fire_f3_reask(events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `fire_f6_manual_spawn` | L133 | `fn fire_f6_manual_spawn(events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `missing_cloud_auth_copy` | L191 | `fn missing_cloud_auth_copy(is_ru: bool) -> (&'static str, &'static str)` |
| function | `cloud_auth_notice_has_both_ui_languages` | L679 | `fn cloud_auth_notice_has_both_ui_languages() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L540
- Spawns asynchronous thread/task at L638
