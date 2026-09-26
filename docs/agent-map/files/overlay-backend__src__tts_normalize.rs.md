---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_5a34cf79858c"
source_path: "overlay-backend/src/tts_normalize.rs"
batch_id: "B10"
total_lines: 395
symbols_count: 27
review_state: validated
---

# File Map: `overlay-backend/src/tts_normalize.rs`

- **Batch:** B10
- **Physical Lines:** 395
- **Coverage:** 395/395 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (27)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `normalize_for_speech` | L13 | `fn normalize_for_speech(text: &str) -> String` |
| function | `parse_at` | L29 | `fn parse_at(text: &str, pos: usize) -> Option<(usize, String)>` |
| function | `parse_version` | L115 | `fn parse_version(rest: &str) -> Option<(usize, String)>` |
| function | `parse_pair` | L143 | `fn parse_pair(after: &str, separator: char) -> Option<(usize, &str)>` |
| function | `token_ends` | L153 | `fn token_ends(text: &str, end: usize) -> bool` |
| function | `signed_integer_words` | L160 | `fn signed_integer_words(digits: &str, negative: bool) -> Option<String>` |
| function | `digits_or_integer_words` | L170 | `fn digits_or_integer_words(digits: &str) -> Option<String>` |
| function | `digits_words` | L178 | `fn digits_words(digits: &str) -> Option<String>` |
| function | `integer_words` | L186 | `fn integer_words(value: u64) -> Option<String>` |
| function | `group_words` | L208 | `fn group_words(value: u16, feminine: bool) -> Vec<&'static str>` |
| function | `plural` | L282 | `fn plural(value: u64, forms: [&'static str; 3]) -> &'static str` |
| function | `integers` | L302 | `fn integers() -> ()` |
| function | `negative_integer` | L310 | `fn negative_integer() -> ()` |
| function | `thousands_use_feminine_forms` | L318 | `fn thousands_use_feminine_forms() -> ()` |
| function | `large_integer` | L323 | `fn large_integer() -> ()` |
| function | `value_above_trillions_is_unchanged` | L331 | `fn value_above_trillions_is_unchanged() -> ()` |
| function | `percents_with_and_without_space` | L336 | `fn percents_with_and_without_space() -> ()` |
| function | `clock_time` | L344 | `fn clock_time() -> ()` |
| function | `invalid_clock_is_left_as_punctuation` | L349 | `fn invalid_clock_is_left_as_punctuation() -> ()` |
| function | `hyphen_range` | L354 | `fn hyphen_range() -> ()` |
| function | `unicode_dash_ranges` | L359 | `fn unicode_dash_ranges() -> ()` |
| function | `decimal_dot_and_comma` | L364 | `fn decimal_dot_and_comma() -> ()` |
| function | `version` | L369 | `fn version() -> ()` |
| function | `version_preserves_leading_zeroes` | L374 | `fn version_preserves_leading_zeroes() -> ()` |
| function | `no_numbers_is_identity` | L379 | `fn no_numbers_is_identity() -> ()` |
| function | `mixed_russian_english_sentence` | L384 | `fn mixed_russian_english_sentence() -> ()` |
| function | `digits_inside_identifiers_are_untouched` | L392 | `fn digits_inside_identifiers_are_untouched() -> ()` |
