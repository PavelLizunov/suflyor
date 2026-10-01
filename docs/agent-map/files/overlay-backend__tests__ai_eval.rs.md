---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_195d940b954a"
source_path: "overlay-backend/tests/ai_eval.rs"
batch_id: "B09"
total_lines: 114
symbols_count: 7
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "cfdb3a71d9d9833777d328e3f8a3ce51bb72a5fbc34af0f8d0038a030d1378fe"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/tests/ai_eval.rs`

- **Batch:** B09
- **Physical Lines:** 114
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (7)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `summary_has_all_headings` | L26 | `fn summary_has_all_headings(summary: &str, headings: &[&str]) -> bool` |
| function | `looks_like_untranslated_echo` | L34 | `fn looks_like_untranslated_echo(output: &str) -> bool` |
| function | `name_is_clean` | L47 | `fn name_is_clean(name: &str) -> bool` |
| function | `summary_heading_checker_catches_a_dropped_section` | L59 | `fn summary_heading_checker_catches_a_dropped_section() -> ()` |
| function | `translate_echo_detector` | L73 | `fn translate_echo_detector() -> ()` |
| function | `auto_name_contract` | L89 | `fn auto_name_contract() -> ()` |
| function | `live_local_ai_invariants` | L103 | `fn live_local_ai_invariants() -> ()` |
