---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_b753c5fe9248"
source_path: "slint-experiment/src/bin/overlay_host/settings_hermes.rs"
batch_id: "B02"
total_lines: 448
symbols_count: 6
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "b501469e0c6f2abe993826029f49f0d27cc42db86b2be87397b787a197d58f4b"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_hermes.rs`

- **Batch:** B02
- **Physical Lines:** 448
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `current_bridge_status` | L29 | `fn current_bridge_status(host: &str, port: u16) -> String` |
| function | `apply_bridge_state` | L46 | `fn apply_bridge_state(cfg: &overlay_backend::config::SharedConfig) -> String` |
| function | `wire_hermes_settings` | L73 | `fn wire_hermes_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `run_prepare_profile` | L368 | `fn run_prepare_profile(url: &str, key: &str, seed: &str, cfg: &overlay_backend::config::SharedConfig,) -> String` |
| function | `profile_name_from_seed` | L420 | `fn profile_name_from_seed(seed: &str) -> String` |
| function | `profile_name_derivation` | L435 | `fn profile_name_derivation() -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L188
- Spawns asynchronous thread/task at L251
- Spawns asynchronous thread/task at L297
- Spawns asynchronous thread/task at L347
