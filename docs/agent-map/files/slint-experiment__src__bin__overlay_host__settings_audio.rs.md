---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_263a3f9d3007"
source_path: "slint-experiment/src/bin/overlay_host/settings_audio.rs"
batch_id: "B02"
total_lines: 494
symbols_count: 16
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_audio.rs`

- **Batch:** B02
- **Physical Lines:** 494
- **Coverage:** 494/494 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `DeviceChoices` | L8 | private |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L16 | `fn new(devices: Vec<String>, saved: Option<&str>, default_label: &str) -> Self` |
| function | `selection` | L46 | `fn selection(names: &[String], index: i32, missing: bool) -> Option<Option<String>>` |
| function | `saved_index` | L58 | `fn saved_index(names: &[String], saved: Option<&str>) -> i32` |
| function | `persist_selection` | L71 | `fn persist_selection(cfg: &config::SharedConfig, system: bool, selected: Option<String>, persist: impl FnOnce(&config::Config) -> anyhow::Result<()>, ) -> anyhow::Result<()>` |
| function | `model` | L91 | `fn model(names: Vec<String>) -> ModelRc<SharedString>` |
| function | `show_devices` | L100 | `fn show_devices(win: &SettingsWindow, cfg: &config::SharedConfig, inputs: Vec<String>, outputs: Vec<String>,) -> ()` |
| function | `refresh` | L130 | `fn refresh(win: &SettingsWindow, cfg: &config::SharedConfig) -> ()` |
| function | `choose` | L158 | `fn choose(win: &SettingsWindow, cfg: &config::SharedConfig, system: bool, index: i32, persist: impl FnOnce(&config::Config) -> anyhow::Result<()>, )` |
| function | `wire` | L223 | `fn wire(win: &SettingsWindow, cfg: &config::SharedConfig) -> ()` |
| function | `default_is_not_the_first_endpoint_or_a_translated_name` | L254 | `fn default_is_not_the_first_endpoint_or_a_translated_name() -> ()` |
| function | `missing_saved_device_is_preserved_but_not_selectable` | L279 | `fn missing_saved_device_is_preserved_but_not_selectable() -> ()` |
| function | `empty_list_can_clear_an_obsolete_binding_without_saving_a_placeholder` | L296 | `fn empty_list_can_clear_an_obsolete_binding_without_saving_a_placeholder() -> ()` |
| function | `capture_mixes_remain_selectable_and_duplicates_do_not_shift_selection` | L308 | `fn capture_mixes_remain_selectable_and_duplicates_do_not_shift_selection() -> ()` |
| function | `window_state_tracks_selection_refresh_and_save_failure` | L322 | `fn window_state_tracks_selection_refresh_and_save_failure() -> ()` |
| function | `live_audio_fixture` | L402 | `fn live_audio_fixture() -> ()` |
| function | `save_failure_keeps_config_and_unrelated_fields` | L469 | `fn save_failure_keeps_config_and_unrelated_fields() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L140

## Configuration Access

- Configuration read at L326: `cfg.write().system_audio_device = Some("Old headset".into());`
- Configuration read at L337: `cfg.read().system_audio_device.as_deref(),`
- Configuration read at L351: `cfg.read().system_audio_device.as_deref(),`
- Configuration read at L356: `assert!(cfg.read().system_audio_device.is_none());`
- Configuration read at L360: `cfg.write().system_audio_device = Some("A50 Stream Out".into());`
- Configuration read at L375: `cfg.read().system_audio_device.as_deref(),`
- Configuration read at L392: `cfg.read().system_audio_device.as_deref(),`
- Configuration read at L418: `cfg.write().system_audio_device = Some("Disconnected headphones".into());`
