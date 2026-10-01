---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3cfcce409a86"
source_path: "slint-experiment/src/bin/overlay_host/tile_ask.rs"
batch_id: "B03"
total_lines: 688
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "823064c0975cd110ac67f74aeedf3ff483977c2027ce6167cf1f67016c810753"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_ask.rs`

- **Batch:** B03
- **Physical Lines:** 688
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `fire_f3_reask` | L62 | `fn fire_f3_reask(events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `fire_f6_manual_spawn` | L133 | `fn fire_f6_manual_spawn(events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `missing_cloud_auth_copy` | L191 | `fn missing_cloud_auth_copy(is_ru: bool) -> (&'static str, &'static str)` |
| function | `cloud_auth_notice_has_both_ui_languages` | L679 | `fn cloud_auth_notice_has_both_ui_languages() -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L540
- Spawns asynchronous thread/task at L638
