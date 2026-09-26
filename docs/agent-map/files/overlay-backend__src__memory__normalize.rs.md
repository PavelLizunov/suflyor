---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_46fc1aad3d86"
source_path: "overlay-backend/src/memory/normalize.rs"
batch_id: "B08"
total_lines: 956
symbols_count: 38
review_state: validated
---

# File Map: `overlay-backend/src/memory/normalize.rs`

- **Batch:** B08
- **Physical Lines:** 956
- **Coverage:** 956/956 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `NormalizedFact` | L517 | pub |
| struct | `ParsedFact` | L637 | private |
| struct | `FactsDto` | L649 | private |
| struct | `FactDto` | L654 | private |

## Symbols & Routines (38)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `word_core` | L34 | `fn word_core(tok: &str) -> String` |
| function | `is_cyr` | L42 | `fn is_cyr(c: char) -> bool` |
| function | `cyr_upper_confusable` | L48 | `fn cyr_upper_confusable(c: char) -> Option<char>` |
| function | `split_fused_token` | L74 | `fn split_fused_token(tok: &str) -> Vec<String>` |
| function | `heuristic_clean` | L137 | `fn heuristic_clean(text: &str) -> String` |
| function | `tokenize` | L158 | `fn tokenize(s: &str) -> Vec<(String, usize, usize)>` |
| function | `is_boundary_at` | L177 | `fn is_boundary_at(chars: &[char], i: usize) -> bool` |
| function | `crosses_clause_boundary` | L187 | `fn crosses_clause_boundary(s: &str) -> bool` |
| function | `segment_clauses` | L201 | `fn segment_clauses(s: &str) -> Vec<String>` |
| function | `push_trimmed` | L247 | `fn push_trimmed(s: &str, out: &mut Vec<String>) -> ()` |
| function | `heuristic_condense` | L259 | `fn heuristic_condense(cleaned: &str) -> String` |
| function | `locate_span` | L289 | `fn locate_span(source: &str, quote: &str) -> Option<String>` |
| function | `common_prefix_len` | L390 | `fn common_prefix_len(a: &str, b: &str) -> usize` |
| function | `words_match` | L399 | `fn words_match(a: &str, b: &str) -> bool` |
| function | `grounded_in_order` | L410 | `fn grounded_in_order(fact_words: &[String], src_words: &[String]) -> bool` |
| function | `content_words` | L423 | `fn content_words(s: &str) -> Vec<String>` |
| function | `digit_tokens` | L439 | `fn digit_tokens(s: &str) -> Vec<String>` |
| function | `push_digit_token` | L455 | `fn push_digit_token(cur: &str, out: &mut Vec<String>) -> ()` |
| function | `negation_words` | L463 | `fn negation_words(s: &str) -> Vec<String>` |
| function | `is_ordered_subsequence` | L474 | `fn is_ordered_subsequence(needles: &[String], haystack: &[String]) -> bool` |
| function | `validate_rewrite` | L493 | `fn validate_rewrite(span: &str, fact: &str) -> bool` |
| function | `normalize_fact` | L557 | `fn normalize_fact(raw: &str, base_url: &str, bearer: &str, model: &str,) -> anyhow::Result<Option<NormalizedFact>>` |
| function | `parse_facts` | L647 | `fn parse_facts(resp: &str) -> Vec<ParsedFact>` |
| function | `rewrite_prompt_treats_memory_source_as_untrusted_data` | L701 | `fn rewrite_prompt_treats_memory_source_as_untrusted_data() -> ()` |
| function | `heuristic_collapses_ws_and_dedups_stutter_words` | L708 | `fn heuristic_collapses_ws_and_dedups_stutter_words() -> ()` |
| function | `split_fused_token_de_garbles_mixed_script_only` | L724 | `fn split_fused_token_de_garbles_mixed_script_only() -> ()` |
| function | `locate_span_returns_verbatim_source_slice` | L755 | `fn locate_span_returns_verbatim_source_slice() -> ()` |
| function | `locate_span_rejects_truncated_identifier_and_fabrication` | L768 | `fn locate_span_rejects_truncated_identifier_and_fabrication() -> ()` |
| function | `locate_span_rejects_recombination` | L776 | `fn locate_span_rejects_recombination() -> ()` |
| function | `locate_span_rejects_boundary_and_overlong` | L790 | `fn locate_span_rejects_boundary_and_overlong() -> ()` |
| function | `parse_facts_handles_clause_index_and_multi` | L806 | `fn parse_facts_handles_clause_index_and_multi() -> ()` |
| function | `parse_facts_rejects_empty_and_garbage` | L826 | `fn parse_facts_rejects_empty_and_garbage() -> ()` |
| function | `segment_clauses_splits_drops_filler_and_repacks` | L834 | `fn segment_clauses_splits_drops_filler_and_repacks() -> ()` |
| function | `owner_garbled_line_cleans_end_to_end` | L861 | `fn owner_garbled_line_cleans_end_to_end() -> ()` |
| function | `heuristic_condense_picks_best_clauses_not_whole_line` | L884 | `fn heuristic_condense_picks_best_clauses_not_whole_line() -> ()` |
| function | `validate_rewrite_accepts_faithful_clean_rewrite` | L901 | `fn validate_rewrite_accepts_faithful_clean_rewrite() -> ()` |
| function | `validate_rewrite_rejects_fabrication_number_and_negation_change` | L916 | `fn validate_rewrite_rejects_fabrication_number_and_negation_change() -> ()` |
| function | `validate_rewrite_rejects_reorder_and_within_clause_recombination` | L938 | `fn validate_rewrite_rejects_reorder_and_within_clause_recombination() -> ()` |
