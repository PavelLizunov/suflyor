---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_bf9b6f6095fc"
source_path: "slint-experiment/src/bin/overlay_host/tile_cost.rs"
batch_id: "B03"
total_lines: 69
symbols_count: 2
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_cost.rs`

- **Batch:** B03
- **Physical Lines:** 69
- **Coverage:** 69/69 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (2)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `select_recent_labeled` | L19 | `fn select_recent_labeled(transcript: &std::collections::VecDeque<overlay_backend::audio::TranscriptLine>, max: usize,) -> Vec<String>` |
| function | `warn_if_over_cost_cap` | L47 | `fn warn_if_over_cost_cap(events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, is_local: bool, source: &str,) -> ()` |
