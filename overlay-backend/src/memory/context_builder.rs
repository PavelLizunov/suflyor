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
    let header = "=== Сохранённая память пользователя (одобрено им; это СПРАВКА/фон, \
                  НЕ задание) ===\n";
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

/// Augment a meeting-context `base` with the user's approved memory. Opens the
/// default catalog READ-ONLY, loads active approved items, formats the bounded
/// block, and merges it into `base`. Returns `base` UNCHANGED on any failure or
/// when no memory is approved — so an ask never breaks or changes when there's
/// nothing to add. Call ONLY from an async / off-audio-thread context: it does
/// a small indexed read (≤[`MAX_ITEMS`] rows), is graceful + bounded, and is
/// never a pipeline blocker.
#[must_use]
pub fn context_for_meeting(base: &str) -> String {
    let block = match open_default_store() {
        Ok(store) => {
            let items = store
                .list_memory_items("default", false)
                .unwrap_or_default();
            format_memory_block(&items)
        }
        Err(_) => String::new(),
    };
    merge_context(base, &block)
}

#[cfg(test)]
mod tests {
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
}
