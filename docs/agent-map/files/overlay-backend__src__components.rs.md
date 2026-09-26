---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2a90759f91ff"
source_path: "overlay-backend/src/components.rs"
batch_id: "B09"
total_lines: 177
symbols_count: 6
review_state: validated
---

# File Map: `overlay-backend/src/components.rs`

- **Batch:** B09
- **Physical Lines:** 177
- **Coverage:** 177/177 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `ComponentKind` | L19 | pub |
| struct | `ComponentStatus` | L34 | pub |

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `status` | L47 | `fn status(cfg: &Config) -> Vec<ComponentStatus>` |
| function | `engine_detail` | L121 | `fn engine_detail(build: Option<u32>) -> String` |
| function | `local_model_detail` | L126 | `fn local_model_detail(fallback: Option<&str>, quality: bool) -> String` |
| function | `engine_detail_formats_build_or_empty` | L139 | `fn engine_detail_formats_build_or_empty() -> ()` |
| function | `local_model_detail_covers_tiers` | L145 | `fn local_model_detail_covers_tiers() -> ()` |
| function | `status_returns_every_component_once` | L157 | `fn status_returns_every_component_once() -> ()` |
