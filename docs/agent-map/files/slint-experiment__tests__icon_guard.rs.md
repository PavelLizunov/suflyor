---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3b87e7c40cd4"
source_path: "slint-experiment/tests/icon_guard.rs"
batch_id: "B05"
total_lines: 197
symbols_count: 5
review_state: validated
---

# File Map: `slint-experiment/tests/icon_guard.rs`

- **Batch:** B05
- **Physical Lines:** 197
- **Coverage:** 197/197 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `allowed_attributes` | L62 | `fn allowed_attributes(tag: &str) -> Option<&'static [&'static str]>` |
| function | `validate_primitive` | L74 | `fn validate_primitive(name: &str, source: &str) -> Result<(), String>` |
| function | `validate_icon` | L114 | `fn validate_icon(name: &str, svg: &str) -> Vec<String>` |
| function | `complete_icon_set_uses_the_astra_contract` | L157 | `fn complete_icon_set_uses_the_astra_contract() -> ()` |
| function | `rejects_non_astra_svg_features` | L179 | `fn rejects_non_astra_svg_features() -> ()` |
