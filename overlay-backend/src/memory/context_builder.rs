//! context_builder (Phase 3b.4 — the payoff) — assemble the user's APPROVED
//! memory into a bounded block that augments the meeting-context background of an
//! AI ask, so curated memory influences answers.
//!
//! HARD LIMITS (the spec's "bounded context"): at most [`MAX_ITEMS`] items and
//! [`MAX_BLOCK_CHARS`] characters, newest-approved first (the store returns
//! active items newest-first), so a large memory store never blows the prompt or
//! the token budget. Only APPROVED, non-archived items are used — the user's
//! per-item approval IS the consent; if nothing is approved the block is empty
//! and the ask is byte-identical to before (no token / cost change).
//!
//! The memory is folded into the SAME "user background" block as
//! `meeting_context` and clearly labelled as REFERENCE (not the task), reusing
//! the prompt builders' existing anti-topic-lock guidance.

use super::normalize::{tokenize, words_match, STOPWORDS};
use crate::persistence::{open_default_store, MemoryItem};

/// Max approved items folded into one ask's context.
const MAX_ITEMS: usize = 8;
/// Max total characters of the memory block (a token-budget guard).
const MAX_BLOCK_CHARS: usize = 1200;
/// Per-item character cap so one long item can't crowd out the rest.
const MAX_ITEM_CHARS: usize = 240;

/// Format approved memory `items` (already newest-first) into a labelled block
/// for the system prompt, or `""` when there are none / all empty. Pure →
/// unit-tested. Each item is whitespace-collapsed to one line and capped; the
/// block stops at the item or character budget, whichever comes first.
#[must_use]
pub fn format_memory_block(items: &[MemoryItem]) -> String {
    // ТЗ 2026-07-06 (A-3) — the second sentence makes memory WIN over "нет
    // информации": when the question is about a fact below, the model must
    // answer FROM it instead of claiming it knows nothing.
    let header = "=== Сохранённая память пользователя (одобрено им; это СПРАВКА/фон, \
                  НЕ задание). Если вопрос касается фактов отсюда — отвечай ПО НИМ, \
                  а не «нет информации» ===\n";
    let footer = "=== Конец памяти ===";
    let mut out = String::new();
    let mut used = 0usize;
    for it in items.iter().take(MAX_ITEMS) {
        let collapsed: String = it.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            continue;
        }
        let one: String = collapsed.chars().take(MAX_ITEM_CHARS).collect();
        let line = format!("- {one}\n");
        // Budget on the FULL block (header + lines so far + this line + footer).
        let projected = header.chars().count()
            + out.chars().count()
            + line.chars().count()
            + footer.chars().count();
        if used > 0 && projected > MAX_BLOCK_CHARS {
            break;
        }
        out.push_str(&line);
        used += 1;
    }
    if used == 0 {
        return String::new();
    }
    format!("{header}{out}{footer}")
}

/// Join a meeting-context `base` with a memory `block`. `base` alone when the
/// block is empty; the block alone when `base` is blank; otherwise the two
/// separated by a blank line. Pure → unit-tested.
#[must_use]
pub fn merge_context(base: &str, block: &str) -> String {
    if block.is_empty() {
        base.to_string()
    } else if base.trim().is_empty() {
        block.to_string()
    } else {
        format!("{}\n\n{block}", base.trim())
    }
}

/// ТЗ 2026-07-06 (A) — the question's CONTENT terms: lowercased alnum tokens,
/// ≥ 4 chars (drops «кто», «про», prepositions — and, with [`STOPWORDS`], the
/// longer fillers), deduped. Empty → the caller falls back to recency.
fn query_terms(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = tokenize(query)
        .into_iter()
        .map(|(t, _, _)| t)
        .filter(|t| t.chars().count() >= 4 && !STOPWORDS.contains(&t.as_str()))
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

/// How many of the question's `terms` find a root-sharing token in this item's
/// `text` (+ `entity`). Root match = [`words_match`]'s SYMMETRIC prefix
/// (≥ min(len, 4)): «влад»↔«владислав», «влада»↔«владислав» (either side may be
/// the shorter one), and even the «писчанкин»↔«писчаскин» typo (shared root
/// «писча» = 5) — no fuzzy dep needed.
fn score_item(terms: &[String], it: &MemoryItem) -> usize {
    let mut hay: Vec<String> = tokenize(&it.text).into_iter().map(|(t, _, _)| t).collect();
    if let Some(e) = &it.entity {
        hay.extend(tokenize(e).into_iter().map(|(t, _, _)| t));
    }
    terms
        .iter()
        .filter(|q| hay.iter().any(|h| words_match(q, h)))
        .count()
}

/// Rank `items` (newest-first) by relevance to `query`: matched items only,
/// best score first, newest-first within a score (stable sort), capped at
/// [`MAX_ITEMS`]. `None` when the query has no content terms OR nothing matches
/// — the caller then falls back to recency, so memory is ALWAYS injected.
/// Pure → unit-tested. In-memory scan is fine at the current scale (dozens of
/// facts, all already loaded); the ADR's FTS5/embeddings phase re-enters at
/// ~150-200 items (docs/memory-architecture.md §6).
fn rank_by_relevance(query: &str, items: &[MemoryItem]) -> Option<Vec<MemoryItem>> {
    let terms = query_terms(query);
    if terms.is_empty() {
        return None;
    }
    let mut scored: Vec<(usize, &MemoryItem)> = items
        .iter()
        .map(|it| (score_item(&terms, it), it))
        .filter(|(s, _)| *s > 0)
        .collect();
    if scored.is_empty() {
        return None;
    }
    scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s)); // stable → newest-first within a score
    Some(
        scored
            .into_iter()
            .take(MAX_ITEMS)
            .map(|(_, it)| it.clone())
            .collect(),
    )
}

/// Augment a meeting-context `base` with the user's approved memory. Opens the
/// default catalog READ-ONLY, loads active approved items, formats the bounded
/// block, and merges it into `base`. Returns `base` UNCHANGED on any failure or
/// when no memory is approved — so an ask never breaks or changes when there's
/// nothing to add. Call ONLY from an async / off-audio-thread context: it does
/// a small indexed read, is graceful + bounded, and is never a pipeline blocker.
///
/// ТЗ 2026-07-06 (A) — `query` = the user's question when the ask has one:
/// the block is then the most RELEVANT facts (soft root-match, so «Влад
/// Кощеев» finds «Владислав Кощеев») instead of blindly the newest 8 — the
/// bug where approved fact №9+ never reached the model. `None`/no-match →
/// newest-first exactly as before (memory is always injected).
#[must_use]
pub fn context_for_meeting(base: &str, query: Option<&str>) -> String {
    let block = match open_default_store() {
        Ok(store) => {
            let items = store
                .list_memory_items("default", false, -1)
                .unwrap_or_default();
            match query.and_then(|q| rank_by_relevance(q, &items)) {
                Some(relevant) => format_memory_block(&relevant),
                None => format_memory_block(&items),
            }
        }
        Err(_) => String::new(),
    };
    merge_context(base, &block)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn item(text: &str) -> MemoryItem {
        MemoryItem {
            id: 0,
            profile_id: "default".into(),
            kind: "answer".into(),
            text: text.into(),
            source_session_id: None,
            approved_at_ms: 0,
            archived_at_ms: None,
            embedding_status: "none".into(),
            source_text: None,
            entity: None,
            norm_status: "none".into(),
        }
    }

    #[test]
    fn empty_items_yield_empty_block() {
        assert_eq!(format_memory_block(&[]), "");
        assert_eq!(format_memory_block(&[item("   ")]), "");
    }

    #[test]
    fn block_has_header_lines_footer() {
        let block =
            format_memory_block(&[item("uses Rust + tokio"), item("prefers concise answers")]);
        assert!(block.contains("Сохранённая память"));
        assert!(block.contains("- uses Rust + tokio"));
        assert!(block.contains("- prefers concise answers"));
        assert!(block.trim_end().ends_with("=== Конец памяти ==="));
    }

    #[test]
    fn item_text_is_whitespace_collapsed() {
        let block = format_memory_block(&[item("line one\n   line two\t\tend")]);
        assert!(block.contains("- line one line two end"));
        assert!(!block.contains('\t'));
    }

    #[test]
    fn caps_at_max_items() {
        let items: Vec<MemoryItem> = (0..20).map(|i| item(&format!("fact number {i}"))).collect();
        let block = format_memory_block(&items);
        // Exactly MAX_ITEMS bullet lines (item text carries no "- ").
        assert_eq!(block.matches("- ").count(), MAX_ITEMS);
    }

    #[test]
    fn respects_char_budget() {
        // Each item ~MAX_ITEM_CHARS; only a few fit MAX_BLOCK_CHARS.
        let big = "x".repeat(MAX_ITEM_CHARS);
        let items: Vec<MemoryItem> = (0..MAX_ITEMS).map(|_| item(&big)).collect();
        let block = format_memory_block(&items);
        assert!(block.chars().count() <= MAX_BLOCK_CHARS + MAX_ITEM_CHARS); // last line may straddle
        assert!(block.matches("- ").count() < MAX_ITEMS); // budget cut it short
    }

    #[test]
    fn merge_context_branches() {
        assert_eq!(merge_context("bg", ""), "bg");
        assert_eq!(merge_context("", "BLOCK"), "BLOCK");
        assert_eq!(merge_context("  ", "BLOCK"), "BLOCK");
        assert_eq!(merge_context("bg", "BLOCK"), "bg\n\nBLOCK");
    }

    // ===== ТЗ 2026-07-06 (A) — relevance ranking =====

    /// The tester's exact fact.
    fn people_fact() -> MemoryItem {
        item(
            "Люди/подрядчики: Тимур Писчаскин — тимлид, Михаил Голубцов — техлид, \
             Владислав Кощеев — коллега",
        )
    }

    #[test]
    fn diminutive_finds_full_name() {
        // «Влад» ↔ «Владислав» — the acceptance pair.
        let items = vec![item("любит краткие ответы"), people_fact()];
        let ranked = rank_by_relevance("кто такой Влад Кощеев?", &items).unwrap();
        assert!(ranked[0].text.contains("Владислав"));
        assert_eq!(ranked.len(), 1); // the unrelated fact didn't match
    }

    #[test]
    fn declension_finds_full_name() {
        // «у Влада» — inflected diminutive; symmetric prefix still matches.
        let items = vec![people_fact()];
        assert!(rank_by_relevance("что спросить у Влада?", &items).is_some());
    }

    #[test]
    fn typo_surname_finds_fact() {
        // «Писчанкин» ↔ «Писчаскин» — the acceptance typo pair: shared root
        // «писча» (5 ≥ 4) matches directly, plus «Тимур» exactly.
        let items = vec![item("проект: суфлёр на Rust"), people_fact()];
        let ranked = rank_by_relevance("кто такой Тимур Писчанкин?", &items).unwrap();
        assert!(ranked[0].text.contains("Писчаскин"));
    }

    #[test]
    fn relevant_old_fact_beats_newer_noise() {
        // The real bug: the fact is item №9+ (older than 8 noise items) and used
        // to be silently dropped by the newest-8 cap. Ranked → it's first.
        let mut items: Vec<MemoryItem> = (0..10).map(|i| item(&format!("шум номер {i}"))).collect();
        items.push(people_fact()); // oldest position (list is newest-first)
        let ranked = rank_by_relevance("кто такой Владислав Кощеев?", &items).unwrap();
        assert!(ranked[0].text.contains("Кощеев"));
        let block = format_memory_block(&ranked);
        assert!(block.contains("Кощеев"));
    }

    #[test]
    fn no_match_or_no_terms_falls_back() {
        let items = vec![people_fact()];
        // Nothing matches → None → caller uses recency (memory still injected).
        assert!(rank_by_relevance("какая погода в Париже?", &items).is_none());
        // Stopword/short-only question → no content terms → None.
        assert!(rank_by_relevance("кто это?", &items).is_none());
        assert!(rank_by_relevance("", &items).is_none());
    }

    #[test]
    fn short_tokens_do_not_match() {
        // «кто» (3 chars) must not root-match «который»-style words.
        let items = vec![item("который час — неважно")];
        assert!(rank_by_relevance("кто?", &items).is_none());
    }

    #[test]
    fn entity_column_is_searched_too() {
        let mut it = item("работает в команде платформы");
        it.entity = Some("Владислав Кощеев".into());
        assert!(rank_by_relevance("расскажи про Влада", &[it]).is_some());
    }
}
