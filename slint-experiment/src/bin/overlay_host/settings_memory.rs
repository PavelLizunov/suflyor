//! 💭 Memory tab (Phase 3b.3) — the curated-personal-memory REVIEW UI, wired
//! into the Settings window. Lists the pending `memory_candidates` (Approve /
//! Reject) + the APPROVED `memory_items` (Delete) over the SQLite memory tables
//! (3b.1), and runs the heuristic extractor (3b.2a) over recent sessions on the
//! "Extract" button. Everything goes through `persistence::Store` (the SQL
//! surface); the AI "deep extract" button lands with 3b.2b.
//!
//! Opened from Settings, so this is OFF the hot path — each action opens a fresh
//! `Store`, mutates, and reloads the two lists. SECURITY: renders only the
//! user's own candidate/item text (their session Q&A / topics); no secret
//! reaches the tab.

use std::collections::HashSet;

use overlay_backend::memory::extract_heuristic;
use overlay_backend::persistence::{open_default_store, MemoryCandidate, MemoryItem};

use super::{ComponentHandle, MemoryRow, ModelRc, SettingsWindow, SharedString, VecModel};

/// The single memory profile (multi-profile is an open question — see 3b.1).
const PROFILE: &str = "default";
/// How many of the most-recent sessions the Extract action mines.
const EXTRACT_RECENT_SESSIONS: usize = 12;

/// Wire the 💭 Memory tab: load the candidate + item lists and bind the
/// approve / reject / delete / extract callbacks.
pub(crate) fn wire_memory(win: &SettingsWindow) {
    reload_memory(win);

    {
        let weak = win.as_weak();
        win.on_memory_approve(move |id| {
            if let Some(w) = weak.upgrade() {
                if let Ok(mut store) = open_default_store() {
                    let _ = store.approve_candidate(i64::from(id), now_ms());
                }
                reload_memory(&w);
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_memory_reject(move |id| {
            if let Some(w) = weak.upgrade() {
                if let Ok(mut store) = open_default_store() {
                    let _ = store.set_candidate_status(i64::from(id), "rejected");
                }
                reload_memory(&w);
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_memory_delete_item(move |id| {
            if let Some(w) = weak.upgrade() {
                if let Ok(mut store) = open_default_store() {
                    let _ = store.delete_memory_item(i64::from(id));
                }
                reload_memory(&w);
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_memory_extract(move || {
            if let Some(w) = weak.upgrade() {
                let inserted = run_extract();
                w.set_memory_status(SharedString::from(format!("➕ {inserted}")));
                reload_memory(&w);
            }
        });
    }
}

/// Re-open the catalog, load pending candidates + active items, and push both
/// into the tab's models. Best-effort: a catalog-open failure leaves the lists
/// empty rather than crashing.
fn reload_memory(win: &SettingsWindow) {
    let (cands, items) = match open_default_store() {
        Ok(store) => (
            store
                .list_candidates(PROFILE, "pending")
                .unwrap_or_default(),
            store.list_memory_items(PROFILE, false).unwrap_or_default(),
        ),
        Err(_) => (Vec::new(), Vec::new()),
    };
    let c_rows: Vec<MemoryRow> = cands.iter().map(candidate_row).collect();
    let i_rows: Vec<MemoryRow> = items.iter().map(item_row).collect();
    win.set_memory_candidates(ModelRc::new(VecModel::from(c_rows)));
    win.set_memory_items(ModelRc::new(VecModel::from(i_rows)));
}

/// Run the heuristic extractor over the most-recent sessions, inserting any
/// candidate whose text isn't already present (in ANY status — so a rejected or
/// approved one never re-appears). Returns how many NEW candidates were added.
fn run_extract() -> usize {
    let Ok(mut store) = open_default_store() else {
        return 0;
    };
    // Existing candidate texts across ALL statuses → never re-suggest one.
    let mut seen: HashSet<String> = HashSet::new();
    for status in ["pending", "approved", "rejected"] {
        if let Ok(cs) = store.list_candidates(PROFILE, status) {
            for c in cs {
                seen.insert(c.text);
            }
        }
    }
    let sessions = store.list_sessions().unwrap_or_default();
    let mut inserted = 0usize;
    for s in sessions.into_iter().take(EXTRACT_RECENT_SESSIONS) {
        let turns = store.session_ai_turns(&s.id).unwrap_or_default();
        for cand in extract_heuristic(&s.id, &turns) {
            // Skip a text we've already suggested; otherwise insert + count.
            if seen.insert(cand.text.clone()) && store.insert_candidate(&cand, now_ms()).is_ok() {
                inserted += 1;
            }
        }
    }
    inserted
}

/// Candidate → row: kind glyph + the heuristic reason + the candidate text.
/// The DB id is i64; an out-of-range value down-casts to `-1` (a guaranteed
/// no-op on the next callback) rather than silently wrapping onto a real row.
fn candidate_row(c: &MemoryCandidate) -> MemoryRow {
    MemoryRow {
        id: i32::try_from(c.id).unwrap_or(-1),
        kind: SharedString::from(kind_glyph(&c.kind)),
        text: SharedString::from(c.text.clone()),
        reason: SharedString::from(c.reason.clone()),
    }
}

/// Approved item → row: kind glyph + the item text (no reason).
fn item_row(m: &MemoryItem) -> MemoryRow {
    MemoryRow {
        id: i32::try_from(m.id).unwrap_or(-1),
        kind: SharedString::from(kind_glyph(&m.kind)),
        text: SharedString::from(m.text.clone()),
        reason: SharedString::default(),
    }
}

/// A small glyph per memory kind (language-neutral, like the candidate reasons).
fn kind_glyph(kind: &str) -> &'static str {
    match kind {
        "answer" => "📝",
        "weak_topic" => "🔁",
        "preference" => "⚙",
        "experience" => "⭐",
        "note" => "📌",
        _ => "•",
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
