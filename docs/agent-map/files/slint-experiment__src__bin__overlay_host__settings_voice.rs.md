---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c1d0dad06c53"
source_path: "slint-experiment/src/bin/overlay_host/settings_voice.rs"
batch_id: "B02"
total_lines: 475
symbols_count: 12
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_voice.rs`

- **Batch:** B02
- **Physical Lines:** 475
- **Coverage:** 475/475 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `tera_engine_label` | L20 | `fn tera_engine_label(ru: bool) -> &'static str` |
| function | `tera_voice_label` | L30 | `fn tera_voice_label(id: &str) -> String` |
| function | `tts_unavailable_status` | L36 | `fn tts_unavailable_status(ru: bool) -> &'static str` |
| function | `tts_test_status` | L44 | `fn tts_test_status(accepted: bool, ru: bool) -> &'static str` |
| function | `tera_status_line` | L54 | `fn tera_status_line(state: overlay_backend::teratts_install::TeraInstalled, ru: bool,) -> String` |
| function | `tts_rate_for_preset` | L80 | `fn tts_rate_for_preset(idx: i32) -> i32` |
| function | `preset_for_tts_rate` | L93 | `fn preset_for_tts_rate(rate: i32) -> i32` |
| function | `wire_voice_settings` | L101 | `fn wire_voice_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `preset_rate_round_trips` | L417 | `fn preset_rate_round_trips() -> ()` |
| function | `stray_index_and_rate_default_to_normal` | L425 | `fn stray_index_and_rate_default_to_normal() -> ()` |
| function | `tera_labels_are_localized_and_ascii_safe` | L437 | `fn tera_labels_are_localized_and_ascii_safe() -> ()` |
| function | `unavailable_status_is_localized_and_screen_share_safe` | L461 | `fn unavailable_status_is_localized_and_screen_share_safe() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L222
- Spawns asynchronous thread/task at L334
