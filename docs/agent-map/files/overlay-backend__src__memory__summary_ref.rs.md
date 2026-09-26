---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2c192ecdadfd"
source_path: "overlay-backend/src/memory/summary_ref.rs"
batch_id: "B08"
total_lines: 284
symbols_count: 13
review_state: validated
---

# File Map: `overlay-backend/src/memory/summary_ref.rs`

- **Batch:** B08
- **Physical Lines:** 284
- **Coverage:** 284/284 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (13)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `key_terms` | L43 | `fn key_terms(text: &str) -> Vec<String>` |
| function | `transcript_tokens` | L106 | `fn transcript_tokens(transcript: &str) -> HashSet<String>` |
| function | `term_in_tokens` | L118 | `fn term_in_tokens(term: &str, tokens: &HashSet<String>) -> bool` |
| function | `relevant_items` | L131 | `fn relevant_items(items: &'a [MemoryItem], transcript: &str) -> Vec<&'a MemoryItem>` |
| function | `format_summary_reference` | L147 | `fn format_summary_reference(matched: &[&MemoryItem]) -> String` |
| function | `summary_reference_for_transcript` | L172 | `fn summary_reference_for_transcript(transcript: &str) -> Option<String>` |
| function | `item` | L197 | `fn item(text: &str) -> MemoryItem` |
| function | `key_terms_finds_names_caps_and_latin` | L214 | `fn key_terms_finds_names_caps_and_latin() -> ()` |
| function | `key_terms_skips_capitalized_sentence_starts_inside_multi_sentence_facts` | L229 | `fn key_terms_skips_capitalized_sentence_starts_inside_multi_sentence_facts() -> ()` |
| function | `key_terms_first_word_definition_and_latin_in_cyrillic` | L240 | `fn key_terms_first_word_definition_and_latin_in_cyrillic() -> ()` |
| function | `relevance_matches_declined_form_and_skips_unmentioned` | L252 | `fn relevance_matches_declined_form_and_skips_unmentioned() -> ()` |
| function | `short_terms_match_exactly_not_inside_longer_words` | L265 | `fn short_terms_match_exactly_not_inside_longer_words() -> ()` |
| function | `reference_block_is_bounded_and_empty_when_nothing` | L274 | `fn reference_block_is_bounded_and_empty_when_nothing() -> ()` |
