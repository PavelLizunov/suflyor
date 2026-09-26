---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c25bc77cb21b"
source_path: "suflyor-teratts/src/num2words.rs"
batch_id: "B12"
total_lines: 424
symbols_count: 16
review_state: validated
---

# File Map: `suflyor-teratts/src/num2words.rs`

- **Batch:** B12
- **Physical Lines:** 424
- **Coverage:** 424/424 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `RuScale` | L214 | private |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `num2words` | L10 | `fn num2words(literal: &str, lang: &str) -> Option<String>` |
| function | `spell_below_1000_en` | L77 | `fn spell_below_1000_en(n: u64) -> String` |
| function | `spell_below_100_en_words` | L97 | `fn spell_below_100_en_words(n: u64) -> String` |
| function | `spell_int_en` | L111 | `fn spell_int_en(n: u64) -> String` |
| function | `spell_decimal_en` | L135 | `fn spell_decimal_en(int_value: u64, frac: Option<&str>) -> Option<String>` |
| function | `plural_index` | L238 | `fn plural_index(n: u64) -> usize` |
| function | `spell_below_1000_ru` | L250 | `fn spell_below_1000_ru(n: u64, feminine: bool) -> String` |
| function | `spell_int_ru` | L272 | `fn spell_int_ru(n: u64) -> String` |
| function | `frac_denominator_ru` | L296 | `fn frac_denominator_ru(digits: usize, one: bool) -> Option<&'static str>` |
| function | `spell_decimal_ru` | L311 | `fn spell_decimal_ru(int_value: u64, frac: Option<&str>) -> Option<String>` |
| function | `whole_form_ru` | L350 | `fn whole_form_ru(n: u64) -> &'static str` |
| function | `english_integers` | L365 | `fn english_integers() -> ()` |
| function | `english_decimals_and_negatives` | L377 | `fn english_decimals_and_negatives() -> ()` |
| function | `russian_integers` | L385 | `fn russian_integers() -> ()` |
| function | `russian_decimals` | L401 | `fn russian_decimals() -> ()` |
| function | `rejects_garbage` | L416 | `fn rejects_garbage() -> ()` |
