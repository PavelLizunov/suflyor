---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_0a9689ed453b"
source_path: "slint-experiment/src/math_display.rs"
batch_id: "B01"
total_lines: 652
symbols_count: 27
review_state: validated
---

# File Map: `slint-experiment/src/math_display.rs`

- **Batch:** B01
- **Physical Lines:** 652
- **Coverage:** 652/652 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (27)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `normalize_math_display` | L8 | `fn normalize_math_display(input: &str) -> String` |
| function | `normalize_math_fragment` | L57 | `fn normalize_math_fragment(input: &str) -> String` |
| function | `restore_case_rows` | L69 | `fn restore_case_rows(input: &str) -> String` |
| function | `normalize_pmatrices` | L134 | `fn normalize_pmatrices(input: &str) -> Option<String>` |
| function | `has_math_delimiter` | L183 | `fn has_math_delimiter(input: &str) -> bool` |
| function | `looks_like_bare_math` | L187 | `fn looks_like_bare_math(text: &str) -> bool` |
| function | `looks_like_delimited_math` | L196 | `fn looks_like_delimited_math(text: &str) -> bool` |
| function | `starts_url` | L205 | `fn starts_url(input: &str, index: usize) -> bool` |
| function | `url_end` | L209 | `fn url_end(input: &str, index: usize) -> usize` |
| function | `delimiter_at` | L216 | `fn delimiter_at(input: &str, index: usize) -> Option<(&'static str, &'static str)>` |
| function | `looks_like_math` | L231 | `fn looks_like_math(text: &str) -> bool` |
| function | `well_formed_fragment` | L239 | `fn well_formed_fragment(text: &str) -> bool` |
| function | `looks_like_numeric_amounts` | L278 | `fn looks_like_numeric_amounts(text: &str) -> bool` |
| function | `push_normalized` | L291 | `fn push_normalized(out: &mut String, text: &str) -> ()` |
| function | `tex_argument_at` | L391 | `fn tex_argument_at(text: &str, start: usize) -> Option<(&str, usize)>` |
| function | `braced_group_at` | L405 | `fn braced_group_at(text: &str, start: usize) -> Option<(&str, usize)>` |
| function | `script_at` | L427 | `fn script_at(text: &str, start: usize, subscript: bool) -> Option<(String, usize)>` |
| function | `script_char` | L446 | `fn script_char(ch: char, subscript: bool) -> Option<char>` |
| function | `named_symbol` | L525 | `fn named_symbol(command: &str) -> Option<&'static str>` |
| function | `normalizes_reference_formula_with_or_without_delimiters` | L563 | `fn normalizes_reference_formula_with_or_without_delimiters() -> ()` |
| function | `malformed_unknown_and_currency_like_text_stays_literal` | L571 | `fn malformed_unknown_and_currency_like_text_stays_literal() -> ()` |
| function | `urls_are_verbatim_and_normalization_is_idempotent` | L590 | `fn urls_are_verbatim_and_normalization_is_idempotent() -> ()` |
| function | `strips_delimiters_from_single_letter_variables` | L604 | `fn strips_delimiters_from_single_letter_variables() -> ()` |
| function | `renders_pmatrix_rows_without_latex_scaffolding` | L612 | `fn renders_pmatrix_rows_without_latex_scaffolding() -> ()` |
| function | `renders_every_pmatrix_in_one_display_fragment` | L618 | `fn renders_every_pmatrix_in_one_display_fragment() -> ()` |
| function | `renders_common_algebra_without_raw_tex_commands` | L633 | `fn renders_common_algebra_without_raw_tex_commands() -> ()` |
| function | `renders_cases_as_readable_lines` | L644 | `fn renders_cases_as_readable_lines() -> ()` |
