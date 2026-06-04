//! Heuristic candidate EXTRACTION — the free, local, deterministic half of the
//! hybrid extractor. Mines a session's AI turns into memory-candidate
//! suggestions WITHOUT any model call (no cost, no egress), so it can run on
//! every finished session. The user then approves/rejects in the review UI; the
//! AI extractor (a separate, opt-in path) adds richer candidates on demand.
//!
//! Two heuristics:
//!  - SUBSTANTIVE ANSWERS → `answer` candidates. A Q&A whose answer is long
//!    enough to be worth keeping ("when asked X I answered Y"), best first,
//!    one per distinct question, capped.
//!  - REPEATED TOPICS → `weak_topic` candidates. A content word that shows up
//!    in two or more DISTINCT questions — a topic the user keeps hitting, worth
//!    prepping. Counted, stop-words removed, capped.
//!
//! Pure + fully deterministic (stable ordering, no clock, no RNG) → unit-tested.
//! Output rows are `NewMemoryCandidate`s the caller inserts via
//! [`crate::persistence::Store::insert_candidate`].

use std::collections::{HashMap, HashSet};

use crate::persistence::{AiTurn, NewMemoryCandidate};

/// Min answer length (chars) for a Q&A to become an `answer` candidate.
const MIN_ANSWER_CHARS: usize = 80;
/// Cap on `answer` candidates per extraction (best — longest answer — first).
const MAX_ANSWER_CANDIDATES: usize = 5;
/// Min length (chars) for a question token to count as a topic word.
const MIN_TOPIC_TOKEN_LEN: usize = 4;
/// A topic word must appear in at least this many DISTINCT questions.
const MIN_TOPIC_OCCURRENCES: usize = 2;
/// Cap on `weak_topic` candidates per extraction (most-repeated first).
const MAX_TOPIC_CANDIDATES: usize = 5;
/// The single default profile (multi-profile memory is an open question).
const DEFAULT_PROFILE: &str = "default";

/// Heuristically extract memory candidates from one session's AI turns. Free +
/// deterministic — safe to run on every finished session. See the module docs.
#[must_use]
pub fn extract_heuristic(session_id: &str, ai_turns: &[AiTurn]) -> Vec<NewMemoryCandidate> {
    let mut out = answer_candidates(session_id, ai_turns);
    out.extend(topic_candidates(session_id, ai_turns));
    out
}

/// Q&A turns with a substantive answer → `answer` candidates (best first, one
/// per distinct question, capped).
fn answer_candidates(session_id: &str, turns: &[AiTurn]) -> Vec<NewMemoryCandidate> {
    let mut substantive: Vec<&AiTurn> = turns
        .iter()
        .filter(|t| {
            !t.question.trim().is_empty() && t.answer.trim().chars().count() >= MIN_ANSWER_CHARS
        })
        .collect();
    // Longest answer first (the "best" Q&A); ties keep input order via a stable
    // sort, so the result is deterministic.
    substantive.sort_by(|a, b| {
        b.answer
            .trim()
            .chars()
            .count()
            .cmp(&a.answer.trim().chars().count())
    });

    let mut seen_question = HashSet::new();
    let mut out = Vec::new();
    for t in substantive {
        if !seen_question.insert(normalize(&t.question)) {
            continue; // one candidate per distinct question
        }
        let chars = t.answer.trim().chars().count();
        out.push(NewMemoryCandidate {
            profile_id: DEFAULT_PROFILE.to_string(),
            source_session_id: Some(session_id.to_string()),
            kind: "answer".to_string(),
            text: format!("Q: {}\nA: {}", t.question.trim(), t.answer.trim()),
            // Language-neutral reason: a note glyph + the answer's char count.
            reason: format!("📝 {chars}"),
        });
        if out.len() >= MAX_ANSWER_CANDIDATES {
            break;
        }
    }
    out
}

/// Content words shared across two or more DISTINCT questions → `weak_topic`
/// candidates (most-repeated first, capped).
fn topic_candidates(session_id: &str, turns: &[AiTurn]) -> Vec<NewMemoryCandidate> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    // First-seen display form (original casing) for each normalized token.
    let mut display: HashMap<String, String> = HashMap::new();

    for t in turns {
        let q = t.question.trim();
        if q.is_empty() {
            continue;
        }
        let mut counted_in_q = HashSet::new();
        for tok in tokenize(q) {
            let norm = tok.to_lowercase();
            if norm.chars().count() < MIN_TOPIC_TOKEN_LEN || is_stopword(&norm) {
                continue;
            }
            // Count a token at most ONCE per question (distinct-question freq).
            if counted_in_q.insert(norm.clone()) {
                *counts.entry(norm.clone()).or_insert(0) += 1;
                display.entry(norm).or_insert_with(|| tok.to_string());
            }
        }
    }

    let mut topics: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(_, c)| *c >= MIN_TOPIC_OCCURRENCES)
        .collect();
    // Most-repeated first; ties broken by token for stable, deterministic order.
    topics.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    topics
        .into_iter()
        .take(MAX_TOPIC_CANDIDATES)
        .map(|(norm, count)| {
            let text = display.get(&norm).cloned().unwrap_or(norm);
            NewMemoryCandidate {
                profile_id: DEFAULT_PROFILE.to_string(),
                source_session_id: Some(session_id.to_string()),
                kind: "weak_topic".to_string(),
                text,
                // Language-neutral reason: a repeat glyph + the occurrence count.
                reason: format!("🔁 ×{count}"),
            }
        })
        .collect()
}

/// Collapse whitespace + lowercase — the key for deduping similar questions.
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Split on every non-alphanumeric char (Unicode-aware → keeps Cyrillic +
/// Latin tokens intact, drops punctuation).
fn tokenize(s: &str) -> Vec<&str> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect()
}

/// A small RU + EN stop-word set so topic mining surfaces real subject words
/// (e.g. "kubernetes", "транзакции") rather than connectives. Tokens shorter
/// than [`MIN_TOPIC_TOKEN_LEN`] are already dropped, so this only lists 4+ char
/// fillers.
fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        // English
        "what" | "that" | "this" | "with" | "from" | "your" | "have" | "about"
            | "into" | "does" | "when" | "which" | "would" | "should" | "could"
            | "their" | "there" | "where" | "will" | "they" | "then" | "than"
            | "such" | "some" | "just" | "also" | "very" | "much" | "more"
        // Russian
            | "что" | "как" | "для" | "это" | "или" | "так" | "его" | "при"
            | "если" | "чтобы" | "когда" | "почему" | "какой" | "какая"
            | "какие" | "меня" | "тебя" | "может" | "можно" | "нужно" | "есть"
            | "быть" | "была" | "были" | "было" | "этот" | "этом" | "того"
            | "чем" | "над" | "под" | "про"
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn turn(q: &str, a: &str) -> AiTurn {
        AiTurn {
            session_id: "s".into(),
            unix_ms: 0,
            purpose: "live_ask".into(),
            model: "m".into(),
            question: q.into(),
            answer: a.into(),
            latency_ms: None,
            attached_screenshot: false,
        }
    }

    fn long(prefix: &str) -> String {
        // A >= MIN_ANSWER_CHARS answer.
        format!("{prefix} {}", "x".repeat(MIN_ANSWER_CHARS))
    }

    #[test]
    fn substantive_answers_become_candidates() {
        let turns = vec![
            turn("what is a hash map", &long("a key-value structure")),
            turn("ping", "ok"), // answer too short → skipped
        ];
        let cands = extract_heuristic("sess-1", &turns);
        let answers: Vec<_> = cands.iter().filter(|c| c.kind == "answer").collect();
        assert_eq!(answers.len(), 1);
        assert!(answers[0].text.starts_with("Q: what is a hash map\nA:"));
        assert_eq!(answers[0].source_session_id.as_deref(), Some("sess-1"));
        assert_eq!(answers[0].profile_id, "default");
        assert!(answers[0].reason.starts_with("📝"));
    }

    #[test]
    fn duplicate_questions_yield_one_answer_candidate() {
        let turns = vec![
            turn("explain B-trees", &long("first")),
            turn("Explain   B-trees", &long("second, longer answer here")),
        ];
        let answers: Vec<_> = extract_heuristic("s", &turns)
            .into_iter()
            .filter(|c| c.kind == "answer")
            .collect();
        assert_eq!(answers.len(), 1); // normalized question dedup
    }

    #[test]
    fn answer_candidates_are_capped() {
        let turns: Vec<AiTurn> = (0..10)
            .map(|i| turn(&format!("question number {i}"), &long("answer")))
            .collect();
        let answers = extract_heuristic("s", &turns)
            .into_iter()
            .filter(|c| c.kind == "answer")
            .count();
        assert_eq!(answers, MAX_ANSWER_CANDIDATES);
    }

    #[test]
    fn repeated_topic_becomes_weak_topic() {
        let turns = vec![
            turn("how to debug kubernetes pods", "..."),
            turn("kubernetes ingress setup", "..."),
            turn("what is rust ownership", "..."),
        ];
        let topics: Vec<_> = extract_heuristic("s", &turns)
            .into_iter()
            .filter(|c| c.kind == "weak_topic")
            .collect();
        // "kubernetes" appears in 2 distinct questions; "rust"/"ownership" once.
        assert!(topics.iter().any(|c| c.text.to_lowercase() == "kubernetes"));
        assert!(topics.iter().all(|c| c.text.to_lowercase() != "rust"));
        let kube = topics
            .iter()
            .find(|c| c.text.to_lowercase() == "kubernetes")
            .unwrap();
        assert_eq!(kube.reason, "🔁 ×2");
    }

    #[test]
    fn stopwords_and_short_tokens_are_not_topics() {
        let turns = vec![
            turn("what is that with this", "..."),
            turn("what is that with this", "..."),
        ];
        let topics = extract_heuristic("s", &turns)
            .into_iter()
            .filter(|c| c.kind == "weak_topic")
            .count();
        assert_eq!(topics, 0); // all stop-words / < 4 chars
    }

    #[test]
    fn repeated_cyrillic_topic_is_mined() {
        let turns = vec![
            turn("расскажи про транзакции в базе", "..."),
            turn("изоляция транзакции уровни", "..."),
        ];
        let topics: Vec<_> = extract_heuristic("s", &turns)
            .into_iter()
            .filter(|c| c.kind == "weak_topic")
            .collect();
        assert!(topics.iter().any(|c| c.text.to_lowercase() == "транзакции"));
    }

    #[test]
    fn empty_session_yields_nothing() {
        assert!(extract_heuristic("s", &[]).is_empty());
    }

    #[test]
    fn extraction_is_deterministic() {
        let turns = vec![
            turn("docker compose networking", "..."),
            turn("docker volumes persistence", "..."),
            turn("kubernetes docker runtime", "..."),
        ];
        let a = extract_heuristic("s", &turns);
        let b = extract_heuristic("s", &turns);
        let texts_a: Vec<_> = a.iter().map(|c| &c.text).collect();
        let texts_b: Vec<_> = b.iter().map(|c| &c.text).collect();
        assert_eq!(texts_a, texts_b);
    }
}
