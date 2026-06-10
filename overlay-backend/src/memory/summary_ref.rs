//! summary_ref (v0.16.0) — keyword-gated memory REFERENCE for the meeting
//! Summary ("личная база знаний", tester request).
//!
//! The Summary deliberately does NOT get the unconditional memory fold that
//! answer paths use (`context_for_meeting`) — v0.12.0's rule: the recap is a
//! factual digest of the transcript, not an answer AS the user. What it DOES
//! need is term decoding: when the call mentions an internal project / name
//! the user has saved («Проект Альфа — внутренняя CRM…»), the secretary
//! should understand what «Альфа» is. So a fact is injected ONLY when one of
//! its name-like key terms actually occurs in this transcript, and the prompt
//! frames the block as decode-only («не добавляй факты, которых не было в
//! разговоре») — the factual-digest integrity line holds.
//!
//! ## v1 relevance heuristic (embeddings are Phase 4)
//!
//! A fact's KEY TERMS are its name-like tokens:
//!
//! - a capitalized token that is not the first word («Проект **Альфа**»,
//!   «команда: **Маша**»);
//! - the first word too when the fact is a definition («**Альфа** — наша CRM»);
//! - an ALL-CAPS token (CRM, API);
//! - a Latin-script token inside a Cyrillic fact («наша **Jira**»).
//!
//! A term matches the transcript case-insensitively; Cyrillic-friendly: terms
//! of ≥5 chars match as a prefix with the last char dropped, so «Альфа» also
//! matches «Альфе» / «Альфу». A fact with NO name-like terms (e.g. a plain
//! preference «отвечай кратко») never reaches the Summary — correct: it is
//! not terminology.

use std::collections::HashSet;

use crate::persistence::{open_default_store, MemoryItem};

/// Max matched facts injected into one Summary reference block.
const MAX_REF_ITEMS: usize = 5;
/// Max total characters of the reference block (token-budget guard).
const MAX_REF_CHARS: usize = 800;
/// Per-fact character cap so one long fact can't crowd out the rest.
const MAX_REF_ITEM_CHARS: usize = 240;

/// Extract the name-like key terms of a fact, lowercased + deduped. Pure.
#[must_use]
pub fn key_terms(text: &str) -> Vec<String> {
    let has_cyrillic = text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    // Tokenize with byte positions so the "definition first word" rule can
    // look at what follows the token in the ORIGINAL text. `sentence_start`
    // marks a token preceded by .!?/newline — its capitalization is sentence
    // case, NOT name-ness (review v0.16.0: «…CRM. Завтра встреча» must not
    // turn «Завтра» into a term that matches every transcript).
    let mut tokens: Vec<(usize, &str, bool)> = Vec::new(); // (end, tok, sentence_start)
    let mut start: Option<usize> = None;
    let mut at_sentence_start = true;
    for (i, ch) in text.char_indices() {
        if ch.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
        } else {
            if let Some(s) = start.take() {
                tokens.push((i, &text[s..i], at_sentence_start));
                at_sentence_start = false;
            }
            if matches!(ch, '.' | '!' | '?' | '\n' | ';') {
                at_sentence_start = true;
            }
        }
    }
    if let Some(s) = start {
        tokens.push((text.len(), &text[s..], at_sentence_start));
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (idx, &(end, tok, sentence_start)) in tokens.iter().enumerate() {
        if tok.chars().all(|c| c.is_numeric()) {
            continue; // bare numbers are never terminology
        }
        let first_upper = tok.chars().next().is_some_and(char::is_uppercase);
        let letters = tok.chars().filter(|c| c.is_alphabetic()).count();
        let all_caps = letters >= 2 && tok.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
        let latin_in_cyrillic = has_cyrillic
            && tok.chars().any(|c| c.is_ascii_alphabetic())
            && tok.chars().all(|c| c.is_ascii_alphanumeric());
        // First word counts only in the definition pattern «Альфа — …» /
        // "Alpha: …" (otherwise it's just sentence capitalization).
        let first_word_definition = idx == 0 && first_upper && {
            let after = text[end..].trim_start();
            after.starts_with('—')
                || after.starts_with('–')
                || after.starts_with('-')
                || after.starts_with(':')
                || after.starts_with('=')
        };
        let capitalized_mid = idx > 0 && !sentence_start && first_upper && tok.chars().count() >= 3;
        if all_caps || latin_in_cyrillic || capitalized_mid || first_word_definition {
            let lower = tok.to_lowercase();
            if seen.insert(lower.clone()) {
                out.push(lower);
            }
        }
    }
    out
}

/// Lowercased word set of a transcript. Pure.
fn transcript_tokens(transcript: &str) -> HashSet<String> {
    transcript
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Did this (lowercased) term come up in the transcript? Terms of ≥5 chars
/// match as a prefix with the final char dropped — a cheap Cyrillic-declension
/// stem («альфа» → «альф» matches «альфе»/«альфу»); shorter terms (CRM, Jira,
/// Маша) must match a token exactly so they can't fire inside longer words.
fn term_in_tokens(term: &str, tokens: &HashSet<String>) -> bool {
    let n = term.chars().count();
    if n >= 5 {
        let stem: String = term.chars().take(n - 1).collect();
        tokens.iter().any(|t| t.starts_with(&stem))
    } else {
        tokens.contains(term)
    }
}

/// The subset of `items` whose key terms occur in `transcript`, in the given
/// (newest-first) order. Pure → unit-tested.
#[must_use]
pub fn relevant_items<'a>(items: &'a [MemoryItem], transcript: &str) -> Vec<&'a MemoryItem> {
    let tokens = transcript_tokens(transcript);
    items
        .iter()
        .filter(|it| {
            key_terms(&it.text)
                .iter()
                .any(|term| term_in_tokens(term, &tokens))
        })
        .collect()
}

/// Format matched facts into the bounded reference block body (plain `- fact`
/// lines — the prompt framing lives in `summary_system_prompt`'s caller).
/// Empty string when nothing fits. Pure → unit-tested.
#[must_use]
pub fn format_summary_reference(matched: &[&MemoryItem]) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for it in matched.iter().take(MAX_REF_ITEMS) {
        let collapsed: String = it.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            continue;
        }
        let one: String = collapsed.chars().take(MAX_REF_ITEM_CHARS).collect();
        let line = format!("- {one}\n");
        if used > 0 && out.chars().count() + line.chars().count() > MAX_REF_CHARS {
            break;
        }
        out.push_str(&line);
        used += 1;
    }
    out.trim_end().to_string()
}

/// Load approved memory and return the reference block for THIS transcript,
/// or `None` when no fact's terms came up (the common case — the Summary
/// request is then byte-identical to a no-memory build). Opens the catalog
/// read-only; graceful: any store failure → `None`, never an error. Small
/// indexed read — fine on a user-initiated path.
#[must_use]
pub fn summary_reference_for_transcript(transcript: &str) -> Option<String> {
    let items = open_default_store()
        .ok()?
        .list_memory_items("default", false)
        .unwrap_or_default();
    if items.is_empty() {
        return None;
    }
    let matched = relevant_items(&items, transcript);
    if matched.is_empty() {
        return None;
    }
    let block = format_summary_reference(&matched);
    if block.is_empty() {
        None
    } else {
        Some(block)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn item(text: &str) -> MemoryItem {
        MemoryItem {
            id: 0,
            profile_id: "default".into(),
            kind: "note".into(),
            text: text.into(),
            source_session_id: None,
            approved_at_ms: 0,
            archived_at_ms: None,
            embedding_status: "none".into(),
        }
    }

    #[test]
    fn key_terms_finds_names_caps_and_latin() {
        let terms = key_terms("Проект Альфа — внутренняя CRM, команда: Маша, Петя");
        assert!(terms.contains(&"альфа".to_string()), "{terms:?}");
        assert!(terms.contains(&"crm".to_string()));
        assert!(terms.contains(&"маша".to_string()));
        assert!(terms.contains(&"петя".to_string()));
        // «Проект» is the sentence-initial word of a NON-definition opener
        // («Проект Альфа» — no dash right after) → not a term.
        assert!(!terms.contains(&"проект".to_string()));
        // Lowercase common words are never terms.
        assert!(!terms.contains(&"внутренняя".to_string()));
        assert!(!terms.contains(&"команда".to_string()));
    }

    #[test]
    fn key_terms_skips_capitalized_sentence_starts_inside_multi_sentence_facts() {
        // «Завтра» opens the SECOND sentence — sentence case, not a name; it
        // must NOT become a term (else the fact matches every transcript that
        // says "tomorrow"). A real name mid-sentence («у Маши») still counts.
        let terms = key_terms("Проект Альфа — наша CRM. Завтра встреча у Маши");
        assert!(!terms.contains(&"завтра".to_string()), "{terms:?}");
        assert!(terms.contains(&"альфа".to_string()));
        assert!(terms.contains(&"маши".to_string()));
    }

    #[test]
    fn key_terms_first_word_definition_and_latin_in_cyrillic() {
        let terms = key_terms("Альфа — наша CRM в Jira");
        assert!(
            terms.contains(&"альфа".to_string()),
            "definition first word"
        );
        assert!(terms.contains(&"jira".to_string()), "latin inside cyrillic");
        // A plain lowercase preference has no terms at all.
        assert!(key_terms("отвечай кратко и по делу").is_empty());
    }

    #[test]
    fn relevance_matches_declined_form_and_skips_unmentioned() {
        let items = vec![
            item("Проект Альфа — внутренняя CRM, команда: Маша, Петя"),
            item("Проект Гамма — биллинг на Go"),
        ];
        // Transcript mentions «Альфе» (declined!) but never «Гамма».
        let transcript = "Вы: обсудим задачи по Альфе на этой неделе\nСобеседник: давайте";
        let matched = relevant_items(&items, transcript);
        assert_eq!(matched.len(), 1);
        assert!(matched[0].text.contains("Альфа"));
    }

    #[test]
    fn short_terms_match_exactly_not_inside_longer_words() {
        let items = vec![item("Маша — наш тимлид")];
        // «машина» contains «маша»? It does NOT start with «маша» and the
        // 4-char term requires an EXACT token → no match.
        assert!(relevant_items(&items, "поговорим про машину и шины").is_empty());
        assert_eq!(relevant_items(&items, "Маша придёт позже").len(), 1);
    }

    #[test]
    fn reference_block_is_bounded_and_empty_when_nothing() {
        assert_eq!(format_summary_reference(&[]), "");
        let many: Vec<MemoryItem> = (0..10)
            .map(|i| item(&format!("Факт{i} — описание номер {i}")))
            .collect();
        let refs: Vec<&MemoryItem> = many.iter().collect();
        let block = format_summary_reference(&refs);
        assert_eq!(block.matches("- ").count(), MAX_REF_ITEMS);
        assert!(block.chars().count() <= MAX_REF_CHARS + MAX_REF_ITEM_CHARS);
    }
}
