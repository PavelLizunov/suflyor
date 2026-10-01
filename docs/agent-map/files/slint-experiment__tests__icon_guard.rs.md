---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3b87e7c40cd4"
source_path: "slint-experiment/tests/icon_guard.rs"
batch_id: "B05"
total_lines: 197
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "2b4654671e113b7d2d33ced222287bc611d2825c059fe397537146e1ca69e492"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/icon_guard.rs`

- **Batch:** B05
- **Physical Lines:** 197
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `allowed_attributes` | L62 | `fn allowed_attributes(tag: &str) -> Option<&'static [&'static str]>` |
| function | `validate_primitive` | L74 | `fn validate_primitive(name: &str, source: &str) -> Result<(), String>` |
| function | `validate_icon` | L114 | `fn validate_icon(name: &str, svg: &str) -> Vec<String>` |
| function | `complete_icon_set_uses_the_astra_contract` | L157 | `fn complete_icon_set_uses_the_astra_contract() -> ()` |
| function | `rejects_non_astra_svg_features` | L179 | `fn rejects_non_astra_svg_features() -> ()` |
