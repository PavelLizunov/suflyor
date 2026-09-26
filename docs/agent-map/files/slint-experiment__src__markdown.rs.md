---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_b757e4c951cf"
source_path: "slint-experiment/src/markdown.rs"
batch_id: "B01"
total_lines: 938
symbols_count: 30
review_state: validated
---

# File Map: `slint-experiment/src/markdown.rs`

- **Batch:** B01
- **Physical Lines:** 938
- **Coverage:** 938/938 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `of` | L6 | private |
| struct | `they` | L39 | private |
| struct | `Block` | L41 | pub |

## Symbols & Routines (30)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L49 | `fn new(kind: i32, text: String, lang: String) -> Self` |
| function | `parse` | L62 | `fn parse(source: &str) -> Vec<Block>` |
| function | `parse_streaming` | L71 | `fn parse_streaming(source: &str) -> Vec<Block>` |
| function | `stable_streaming_prefix` | L75 | `fn stable_streaming_prefix(source: &str) -> &str` |
| function | `canonicalize_tex_math_delimiters` | L104 | `fn canonicalize_tex_math_delimiters(source: &str) -> Cow<'_, str>` |
| function | `fence_marker` | L205 | `fn fence_marker(line: &str) -> Option<(u8, usize)>` |
| function | `parse_single_pass` | L218 | `fn parse_single_pass(source: &str) -> Vec<Block>` |
| function | `flush` | L489 | `fn flush(out: &mut Vec<Block>, raw: &mut String, display: &mut String, kind_slot: &mut Option<i32>, lang: &mut String,) -> ()` |
| function | `format_table` | L529 | `fn format_table(rows: &[Vec<String>]) -> String` |
| function | `truncate_cell` | L566 | `fn truncate_cell(cell: &str, max: usize) -> String` |
| function | `sample_tile_markdown` | L579 | `fn sample_tile_markdown(sequence: u32) -> String` |
| function | `main` | L594 | `fn main() -> ()` |
| function | `link_url_is_preserved_in_text` | L616 | `fn link_url_is_preserved_in_text() -> ()` |
| function | `autolink_url_not_duplicated` | L634 | `fn autolink_url_not_duplicated() -> ()` |
| function | `gfm_table_parses_to_single_aligned_table_block` | L647 | `fn gfm_table_parses_to_single_aligned_table_block() -> ()` |
| function | `table_cells_do_not_bleed_into_surrounding_paragraphs` | L671 | `fn table_cells_do_not_bleed_into_surrounding_paragraphs() -> ()` |
| function | `over_long_table_cell_is_truncated_with_ellipsis` | L693 | `fn over_long_table_cell_is_truncated_with_ellipsis() -> ()` |
| function | `math_display_handles_display_math_but_preserves_code_urls_and_currency` | L711 | `fn math_display_handles_display_math_but_preserves_code_urls_and_currency() -> ()` |
| function | `math_display_normalizes_single_letter_inline_math` | L744 | `fn math_display_normalizes_single_letter_inline_math() -> ()` |
| function | `fenced_math_is_displayed_as_math_instead_of_a_code_box` | L754 | `fn fenced_math_is_displayed_as_math_instead_of_a_code_box() -> ()` |
| function | `tex_bracket_display_is_math_before_markdown_heading_rules` | L768 | `fn tex_bracket_display_is_math_before_markdown_heading_rules() -> ()` |
| function | `bare_matrix_paragraph_does_not_expose_tex_scaffolding` | L801 | `fn bare_matrix_paragraph_does_not_expose_tex_scaffolding() -> ()` |
| function | `bare_cases_paragraph_keeps_equations_on_separate_rows` | L825 | `fn bare_cases_paragraph_keeps_equations_on_separate_rows() -> ()` |
| function | `tex_bracket_delimiters_inside_code_stay_literal` | L834 | `fn tex_bracket_delimiters_inside_code_stay_literal() -> ()` |
| function | `bare_tex_inside_inline_code_stays_literal` | L850 | `fn bare_tex_inside_inline_code_stays_literal() -> ()` |
| function | `tex_parenthesis_inline_becomes_normalized_math` | L856 | `fn tex_parenthesis_inline_becomes_normalized_math() -> ()` |
| function | `streaming_hides_only_the_unfinished_formula_tail` | L866 | `fn streaming_hides_only_the_unfinished_formula_tail() -> ()` |
| function | `streaming_plain_prose_is_not_delayed` | L881 | `fn streaming_plain_prose_is_not_delayed() -> ()` |
| function | `parse_streaming_cost` | L896 | `fn parse_streaming_cost() -> ()` |
| function | `demo` | L902 | `fn demo() -> i32` |
