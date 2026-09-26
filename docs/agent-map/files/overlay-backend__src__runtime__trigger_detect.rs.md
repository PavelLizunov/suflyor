---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_0a6c79b8f115"
source_path: "overlay-backend/src/runtime/trigger_detect.rs"
batch_id: "B06"
total_lines: 657
symbols_count: 16
review_state: validated
---

# File Map: `overlay-backend/src/runtime/trigger_detect.rs`

- **Batch:** B06
- **Physical Lines:** 657
- **Coverage:** 657/657 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Trigger` | L10 | pub |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `build_auto_tile_prompts` | L25 | `fn build_auto_tile_prompts(trigger: &Trigger, recent_transcript: &[String], meeting_context: &str, response_language: &str, live_coaching: bool, mlx_compact: bool,) -> (String, String)` |
| function | `looks_like_real_speech` | L219 | `fn looks_like_real_speech(text: &str) -> bool` |
| function | `strip_filler_prefix` | L249 | `fn strip_filler_prefix(lower: &str) -> String` |
| function | `split_clauses` | L384 | `fn split_clauses(text: &str) -> Vec<&str>` |
| function | `is_question_clause` | L424 | `fn is_question_clause(c_trimmed: &str) -> bool` |
| function | `extract_question_candidate` | L462 | `fn extract_question_candidate(text: &str) -> Option<String>` |
| function | `detect_trigger` | L492 | `fn detect_trigger(text: &str, keyword_list: &str) -> Option<Trigger>` |
| function | `test_extract_question_statement_plus_punctuated_question` | L545 | `fn test_extract_question_statement_plus_punctuated_question() -> ()` |
| function | `test_extract_question_statement_plus_unpunctuated_interrogative_suffix` | L554 | `fn test_extract_question_statement_plus_unpunctuated_interrogative_suffix() -> ()` |
| function | `test_extract_question_trailing_statement_after_question` | L563 | `fn test_extract_question_trailing_statement_after_question() -> ()` |
| function | `test_extract_question_ordinary_statement_none` | L572 | `fn test_extract_question_ordinary_statement_none() -> ()` |
| function | `test_extract_question_one_word_repeated_noise_none` | L578 | `fn test_extract_question_one_word_repeated_noise_none() -> ()` |
| function | `test_detect_trigger_returns_extracted_suffix` | L585 | `fn test_detect_trigger_returns_extracted_suffix() -> ()` |
| function | `test_extract_question_interrogative_prefixes_without_question_mark` | L598 | `fn test_extract_question_interrogative_prefixes_without_question_mark() -> ()` |
| function | `test_extract_question_short_questions_with_question_mark` | L622 | `fn test_extract_question_short_questions_with_question_mark() -> ()` |
| function | `test_statements_and_answers_are_not_detected_as_questions` | L638 | `fn test_statements_and_answers_are_not_detected_as_questions() -> ()` |
