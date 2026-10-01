---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7b60268bd40c"
source_path: "slint-experiment/src/bin/overlay_host/settings_mlx.rs"
batch_id: "B02"
total_lines: 334
symbols_count: 13
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "1b4807016c748c583b846f75790e151e7ad21d49a3904c80d25e9d2701ebb051"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_mlx.rs`

- **Batch:** B02
- **Physical Lines:** 334
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Role` | L10 | private |

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `model` | L16 | `fn model(self) -> &'static str` |
| function | `total_size` | L25 | `fn total_size(self) -> u64` |
| function | `update_state` | L35 | `fn update_state(window: &SettingsWindow, role: Role, installed: bool, failed: bool) -> ()` |
| function | `update_active_state` | L51 | `fn update_active_state(window: &SettingsWindow) -> ()` |
| function | `set_busy` | L59 | `fn set_busy(window: &SettingsWindow, role: Role, busy: bool) -> ()` |
| function | `format_mebibytes` | L66 | `fn format_mebibytes(bytes: u64) -> String` |
| function | `set_progress` | L70 | `fn set_progress(window: &SettingsWindow, role: Role, done: u64, total: u64) -> ()` |
| function | `install` | L87 | `fn install(role: Role, weak: slint::Weak<SettingsWindow>, cancel: Arc<AtomicBool>) -> ()` |
| function | `enable` | L122 | `fn enable(role: Role, weak: slint::Weak<SettingsWindow>, cfg: overlay_backend::config::SharedConfig,) -> ()` |
| function | `refresh` | L190 | `fn refresh(role: Role, weak: slint::Weak<SettingsWindow>) -> ()` |
| function | `wire` | L205 | `fn wire(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |
| function | `populate` | L280 | `fn populate(win: &SettingsWindow) -> ()` |
| function | `progress_bytes_match_the_windows_megabyte_display` | L329 | `fn progress_bytes_match_the_windows_megabyte_display() -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L97
- Spawns asynchronous thread/task at L131
- Spawns asynchronous thread/task at L191

## Heuristic configuration access matches

- Configuration read at L137: `config.ai_provider.clone(),`
- Configuration read at L138: `config.ai_mlx_model.clone(),`
- Configuration read at L144: `config.ai_mlx_model = role.model().into();`
- Configuration read at L145: `config.ai_provider = "mlx".into();`
- Configuration read at L158: `config.ai_provider = previous.0;`
- Configuration read at L159: `config.ai_mlx_model = previous.1;`
