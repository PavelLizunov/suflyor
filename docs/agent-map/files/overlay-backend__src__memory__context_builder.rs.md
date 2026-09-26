---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_51c8660d6495"
source_path: "overlay-backend/src/memory/context_builder.rs"
batch_id: "B08"
total_lines: 344
symbols_count: 23
review_state: validated
---

# File Map: `overlay-backend/src/memory/context_builder.rs`

- **Batch:** B08
- **Physical Lines:** 344
- **Coverage:** 344/344 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (23)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `looks_like_memory_instruction` | L26 | `fn looks_like_memory_instruction(text: &str) -> bool` |
| function | `format_memory_block` | L51 | `fn format_memory_block(items: &[MemoryItem]) -> String` |
| function | `merge_context` | L93 | `fn merge_context(base: &str, block: &str) -> String` |
| function | `query_terms` | L106 | `fn query_terms(query: &str) -> Vec<String>` |
| function | `score_item` | L122 | `fn score_item(terms: &[String], it: &MemoryItem) -> usize` |
| function | `rank_by_relevance` | L140 | `fn rank_by_relevance(query: &str, items: &[MemoryItem]) -> Option<Vec<MemoryItem>>` |
| function | `context_for_meeting` | L176 | `fn context_for_meeting(base: &str, query: Option<&str>) -> String` |
| function | `item` | L197 | `fn item(text: &str) -> MemoryItem` |
| function | `empty_items_yield_empty_block` | L214 | `fn empty_items_yield_empty_block() -> ()` |
| function | `block_has_header_lines_footer` | L220 | `fn block_has_header_lines_footer() -> ()` |
| function | `item_text_is_whitespace_collapsed` | L230 | `fn item_text_is_whitespace_collapsed() -> ()` |
| function | `instruction_like_memory_is_not_injected` | L237 | `fn instruction_like_memory_is_not_injected() -> ()` |
| function | `caps_at_max_items` | L249 | `fn caps_at_max_items() -> ()` |
| function | `respects_char_budget` | L257 | `fn respects_char_budget() -> ()` |
| function | `merge_context_branches` | L267 | `fn merge_context_branches() -> ()` |
| function | `people_fact` | L277 | `fn people_fact() -> MemoryItem` |
| function | `diminutive_finds_full_name` | L285 | `fn diminutive_finds_full_name() -> ()` |
| function | `declension_finds_full_name` | L294 | `fn declension_finds_full_name() -> ()` |
| function | `typo_surname_finds_fact` | L301 | `fn typo_surname_finds_fact() -> ()` |
| function | `relevant_old_fact_beats_newer_noise` | L310 | `fn relevant_old_fact_beats_newer_noise() -> ()` |
| function | `no_match_or_no_terms_falls_back` | L322 | `fn no_match_or_no_terms_falls_back() -> ()` |
| function | `short_tokens_do_not_match` | L332 | `fn short_tokens_do_not_match() -> ()` |
| function | `entity_column_is_searched_too` | L339 | `fn entity_column_is_searched_too() -> ()` |
