---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_5dcacc9b3263"
source_path: "suflyor-teratts/src/textnorm.rs"
batch_id: "B12"
total_lines: 480
symbols_count: 23
review_state: validated
---

# File Map: `suflyor-teratts/src/textnorm.rs`

- **Batch:** B12
- **Physical Lines:** 480
- **Coverage:** 480/480 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `ModelText` | L17 | pub |
| struct | `TagToken` | L137 | private |

## Symbols & Routines (23)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `ensure_language_tags` | L24 | `fn ensure_language_tags(text: &str, lang: &str) -> String` |
| function | `prepare` | L33 | `fn prepare(raw_text: &str, indexer: &UnicodeIndexer) -> Result<ModelText>` |
| function | `add_punctuation_spaces` | L55 | `fn add_punctuation_spaces(text: &str) -> String` |
| function | `add_number_word_spaces` | L82 | `fn add_number_word_spaces(text: &str) -> String` |
| function | `is_word_letter` | L100 | `fn is_word_letter(c: char) -> bool` |
| function | `skip_unsupported` | L114 | `fn skip_unsupported(text: &str, indexer: &UnicodeIndexer, preserve_digits: bool) -> String` |
| function | `find_tag_tokens` | L142 | `fn find_tag_tokens(text: &str) -> Vec<TagToken>` |
| function | `validate_language_tags` | L173 | `fn validate_language_tags(text: &str) -> Result<()>` |
| function | `is_wordish` | L236 | `fn is_wordish(c: char) -> bool` |
| function | `expand_tagged_numbers` | L242 | `fn expand_tagged_numbers(text: &str) -> String` |
| function | `language_spans` | L278 | `fn language_spans(text: &str) -> Vec<([char` |
| function | `tag_token_positions` | L296 | `fn tag_token_positions(chars: &[char]) -> Vec<(bool, [char` |
| function | `match_number` | L325 | `fn match_number(chars: &[char], i: usize) -> Option<(String, usize)>` |
| function | `lookahead_ok` | L326 | `fn lookahead_ok(chars: &[char], pos: usize) -> bool` |
| function | `test_indexer` | L370 | `fn test_indexer() -> UnicodeIndexer` |
| function | `wraps_untagged_text_and_keeps_tagged` | L396 | `fn wraps_untagged_text_and_keeps_tagged() -> ()` |
| function | `punctuation_spacing_keeps_decimals_and_tags` | L402 | `fn punctuation_spacing_keeps_decimals_and_tags() -> ()` |
| function | `number_word_spacing` | L410 | `fn number_word_spacing() -> ()` |
| function | `language_tag_validation_rules` | L418 | `fn language_tag_validation_rules() -> ()` |
| function | `expands_tagged_numbers_per_language` | L431 | `fn expands_tagged_numbers_per_language() -> ()` |
| function | `full_pipeline_produces_nfkd_and_duration_text` | L457 | `fn full_pipeline_produces_nfkd_and_duration_text() -> ()` |
| function | `manual_stress_markers_survive_pipeline` | L467 | `fn manual_stress_markers_survive_pipeline() -> ()` |
| function | `unsupported_characters_are_dropped` | L475 | `fn unsupported_characters_are_dropped() -> ()` |
