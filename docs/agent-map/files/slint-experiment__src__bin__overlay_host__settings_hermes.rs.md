---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_b753c5fe9248"
source_path: "slint-experiment/src/bin/overlay_host/settings_hermes.rs"
batch_id: "B02"
total_lines: 448
symbols_count: 6
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_hermes.rs`

- **Batch:** B02
- **Physical Lines:** 448
- **Coverage:** 448/448 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `current_bridge_status` | L29 | `fn current_bridge_status(host: &str, port: u16) -> String` |
| function | `apply_bridge_state` | L46 | `fn apply_bridge_state(cfg: &overlay_backend::config::SharedConfig) -> String` |
| function | `wire_hermes_settings` | L73 | `fn wire_hermes_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `run_prepare_profile` | L368 | `fn run_prepare_profile(url: &str, key: &str, seed: &str, cfg: &overlay_backend::config::SharedConfig,) -> String` |
| function | `profile_name_from_seed` | L420 | `fn profile_name_from_seed(seed: &str) -> String` |
| function | `profile_name_derivation` | L435 | `fn profile_name_derivation() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L188
- Spawns asynchronous thread/task at L251
- Spawns asynchronous thread/task at L297
- Spawns asynchronous thread/task at L347
