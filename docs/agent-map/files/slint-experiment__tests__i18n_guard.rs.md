---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4c9c11ffac09"
source_path: "slint-experiment/tests/i18n_guard.rs"
batch_id: "B05"
total_lines: 396
symbols_count: 9
review_state: validated
---

# File Map: `slint-experiment/tests/i18n_guard.rs`

- **Batch:** B05
- **Physical Lines:** 396
- **Coverage:** 396/396 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `S` | L307 | private |

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `tr_msgids` | L21 | `fn tr_msgids(src: &str) -> Vec<String>` |
| function | `po_msgids` | L63 | `fn po_msgids(src: &str) -> HashSet<String>` |
| function | `text_expressions` | L81 | `fn text_expressions(src: &str) -> Vec<(usize, &str)>` |
| function | `visible_literal_text` | L197 | `fn visible_literal_text(literal: &str) -> String` |
| function | `needs_translation` | L221 | `fn needs_translation(literal: &str) -> bool` |
| function | `bare_text_literals` | L234 | `fn bare_text_literals(expression: &str) -> Vec<String>` |
| function | `bare_literal_scanner_distinguishes_display_text_from_tokens` | L287 | `fn bare_literal_scanner_distinguishes_display_text_from_tokens() -> ()` |
| function | `every_display_literal_is_translated_or_technical` | L320 | `fn every_display_literal_is_translated_or_technical() -> ()` |
| function | `every_tr_string_has_a_russian_translation` | L352 | `fn every_tr_string_has_a_russian_translation() -> ()` |
