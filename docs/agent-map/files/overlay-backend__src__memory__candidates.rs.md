---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_0b479d12453a"
source_path: "overlay-backend/src/memory/candidates.rs"
batch_id: "B08"
total_lines: 301
symbols_count: 16
review_state: validated
---

# File Map: `overlay-backend/src/memory/candidates.rs`

- **Batch:** B08
- **Physical Lines:** 301
- **Coverage:** 301/301 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `extract_heuristic` | L39 | `fn extract_heuristic(session_id: &str, ai_turns: &[AiTurn]) -> Vec<NewMemoryCandidate>` |
| function | `answer_candidates` | L47 | `fn answer_candidates(session_id: &str, turns: &[AiTurn]) -> Vec<NewMemoryCandidate>` |
| function | `topic_candidates` | L88 | `fn topic_candidates(session_id: &str, turns: &[AiTurn]) -> Vec<NewMemoryCandidate>` |
| function | `normalize` | L137 | `fn normalize(s: &str) -> String` |
| function | `tokenize` | L146 | `fn tokenize(s: &str) -> Vec<&str>` |
| function | `is_stopword` | L156 | `fn is_stopword(token: &str) -> bool` |
| function | `turn` | L178 | `fn turn(q: &str, a: &str) -> AiTurn` |
| function | `long` | L191 | `fn long(prefix: &str) -> String` |
| function | `substantive_answers_become_candidates` | L197 | `fn substantive_answers_become_candidates() -> ()` |
| function | `duplicate_questions_yield_one_answer_candidate` | L212 | `fn duplicate_questions_yield_one_answer_candidate() -> ()` |
| function | `answer_candidates_are_capped` | L225 | `fn answer_candidates_are_capped() -> ()` |
| function | `repeated_topic_becomes_weak_topic` | L237 | `fn repeated_topic_becomes_weak_topic() -> ()` |
| function | `stopwords_and_short_tokens_are_not_topics` | L258 | `fn stopwords_and_short_tokens_are_not_topics() -> ()` |
| function | `repeated_cyrillic_topic_is_mined` | L271 | `fn repeated_cyrillic_topic_is_mined() -> ()` |
| function | `empty_session_yields_nothing` | L284 | `fn empty_session_yields_nothing() -> ()` |
| function | `extraction_is_deterministic` | L289 | `fn extraction_is_deterministic() -> ()` |
