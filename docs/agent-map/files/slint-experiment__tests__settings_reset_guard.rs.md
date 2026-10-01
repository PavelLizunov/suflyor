---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_6efd8bec0c71"
source_path: "slint-experiment/tests/settings_reset_guard.rs"
batch_id: "B05"
total_lines: 158
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "82b018dbf62f0f3c5ecc6e33aa1591211278472f09662ef38184ef305d7cae3c"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/settings_reset_guard.rs`

- **Batch:** B05
- **Physical Lines:** 158
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `transient_string_props` | L33 | `fn transient_string_props(slint_src: &str) -> Vec<String>` |
| function | `populate_body` | L55 | `fn populate_body(rs_src: &str) -> &str` |
| function | `every_transient_status_prop_is_reset_on_reopen` | L72 | `fn every_transient_status_prop_is_reset_on_reopen() -> ()` |
| function | `parser_finds_the_known_transient_props` | L107 | `fn parser_finds_the_known_transient_props() -> ()` |
| function | `updates_tab_check_result_is_reset_on_reopen` | L136 | `fn updates_tab_check_result_is_reset_on_reopen() -> ()` |
