---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4b36527b1237"
source_path: "slint-experiment/src/session_namer.rs"
batch_id: "B01"
total_lines: 358
symbols_count: 16
review_state: validated
---

# File Map: `slint-experiment/src/session_namer.rs`

- **Batch:** B01
- **Physical Lines:** 358
- **Coverage:** 358/358 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `NamerAction` | L47 | private |
| struct | `Gate` | L58 | private |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `decide` | L70 | `fn decide(g: &Gate) -> NamerAction` |
| function | `maybe_spawn_namer` | L99 | `fn maybe_spawn_namer(rt: &SharedSlintRuntime, cfg: &SharedConfig, now_ms: u128) -> ()` |
| function | `generate_name` | L187 | `fn generate_name(ep: &AiEndpoint, lines: &[String]) -> Option<String>` |
| function | `clean_name` | L216 | `fn clean_name(raw: &str) -> String` |
| function | `bar_label` | L228 | `fn bar_label(name: &str) -> String` |
| function | `clean_strips_wrapping_quotes_and_trailing_period` | L249 | `fn clean_strips_wrapping_quotes_and_trailing_period() -> ()` |
| function | `clean_takes_first_line_only` | L259 | `fn clean_takes_first_line_only() -> ()` |
| function | `clean_caps_length` | L264 | `fn clean_caps_length() -> ()` |
| function | `clean_blank_is_empty` | L270 | `fn clean_blank_is_empty() -> ()` |
| function | `clean_keeps_interior_dots` | L276 | `fn clean_keeps_interior_dots() -> ()` |
| function | `bar_label_passes_short_names_unchanged` | L282 | `fn bar_label_passes_short_names_unchanged() -> ()` |
| function | `bar_label_caps_long_names_with_ellipsis` | L291 | `fn bar_label_caps_long_names_with_ellipsis() -> ()` |
| function | `gate` | L298 | `fn gate(requested: bool, has_name: bool, len: usize) -> Gate` |
| function | `decide_first_only_after_trigger_lines` | L312 | `fn decide_first_only_after_trigger_lines() -> ()` |
| function | `decide_inflight_blocks_everything` | L324 | `fn decide_inflight_blocks_everything() -> ()` |
| function | `decide_regen_needs_name_interval_growth_and_lull` | L331 | `fn decide_regen_needs_name_interval_growth_and_lull() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L142

## Configuration Access

- Configuration read at L132: `let ep = cfg.read().ai_endpoint(true);`
