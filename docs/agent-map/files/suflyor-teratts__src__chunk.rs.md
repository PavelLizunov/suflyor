---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_65fa1d51efab"
source_path: "suflyor-teratts/src/chunk.rs"
batch_id: "B12"
total_lines: 107
symbols_count: 9
review_state: validated
---

# File Map: `suflyor-teratts/src/chunk.rs`

- **Batch:** B12
- **Physical Lines:** 107
- **Coverage:** 107/107 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `sanitize` | L7 | `fn sanitize(text: &str) -> String` |
| function | `chunk_text` | L13 | `fn chunk_text(text: &str) -> Vec<String>` |
| function | `push_trimmed` | L34 | `fn push_trimmed(out: &mut Vec<String>, s: String) -> ()` |
| function | `split_sentences` | L41 | `fn split_sentences(text: &str) -> Vec<String>` |
| function | `hard_split` | L56 | `fn hard_split(s: &str, max: usize) -> Vec<String>` |
| function | `splits_on_sentence_boundaries` | L81 | `fn splits_on_sentence_boundaries() -> ()` |
| function | `hard_splits_long_sentences` | L88 | `fn hard_splits_long_sentences() -> ()` |
| function | `empty_and_blank_produce_nothing` | L98 | `fn empty_and_blank_produce_nothing() -> ()` |
| function | `sanitize_drops_control_chars_but_keeps_newlines` | L104 | `fn sanitize_drops_control_chars_but_keeps_newlines() -> ()` |
