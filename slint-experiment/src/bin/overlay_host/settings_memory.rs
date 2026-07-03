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
use overlay_backend::persistence::{
    open_default_store, MemoryCandidate, MemoryItem, NewMemoryItem,
};

use super::{ComponentHandle, MemoryRow, ModelRc, SettingsWindow, SharedString, VecModel};

/// The single memory profile (multi-profile is an open question — see 3b.1).
const PROFILE: &str = "default";
/// How many of the most-recent sessions the Extract action mines.
const EXTRACT_RECENT_SESSIONS: usize = 12;
/// Cap on rows loaded into the review tab: the newest N candidates / items.
/// A DISPLAY bound only — the DB keeps everything and the AI context reads the full
/// set. The rows render in UN-VIRTUALIZED lists inside a ScrollView, so the tab's
/// content height ≈ 2·N·(row height). The Slint software renderer holds coordinates
/// in i16 (max 32767px); once content exceeds that, a rounded-rect row's wrapped
/// coordinate panics the SW renderer (`draw_rounded_rectangle_line` → `Shifted::new`)
/// and corrupts skia (Баг5). With each row clamped to ≤120px in settings_panel.slint,
/// N=100 keeps content well under the i16 limit (see the guard test below).
const MEMORY_TAB_CAP: i64 = 100;

// Баг5 guard (compile-time): keep the un-virtualized lists under the SW renderer's
// i16 (32767px) coordinate limit. 2 lists · cap · per-row px (120px clamp in
// settings_panel.slint + headroom) + a generous card allowance must stay under.
// Raising MEMORY_TAB_CAP without lowering the per-row clamp fails the build.
const _: () = assert!(MEMORY_TAB_CAP * 2 * 140 + 800 < 32_767);

/// Guards re-entering the (worker-thread) extractor while a run is in flight, so
/// a double-click can't launch two scans (P1-3).
static EXTRACT_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Wire the 💭 Memory tab: load the candidate + item lists and bind the
/// approve / reject / delete / extract callbacks.
pub(crate) fn wire_memory(win: &SettingsWindow) {
    reload_memory(win);

    {
        let weak = win.as_weak();
        win.on_memory_approve(move |id| {
            if let Some(w) = weak.upgrade() {
                if let Ok(mut store) = open_default_store() {
                    if let Err(e) = store.approve_candidate(i64::from(id), now_ms()) {
                        eprintln!("[overlay-host] memory approve failed: {e:#}");
                    }
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
                    if let Err(e) = store.set_candidate_status(i64::from(id), "rejected") {
                        eprintln!("[overlay-host] memory reject failed: {e:#}");
                    }
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
                    if let Err(e) = store.delete_memory_item(i64::from(id)) {
                        eprintln!("[overlay-host] memory delete failed: {e:#}");
                    }
                }
                reload_memory(&w);
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_memory_extract(move || {
            let Some(w) = weak.upgrade() else { return };
            // P1-3: the extractor scans candidate texts + reads AI turns for the
            // recent sessions + inserts — far too heavy for the Slint event loop
            // (froze the window on a large DB). Run it on a worker thread; the
            // status text is the busy indicator and EXTRACT_RUNNING guards a
            // double-fire. The Slint models are only ever touched back on the
            // event loop via invoke_from_event_loop.
            if EXTRACT_RUNNING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            w.set_memory_status(SharedString::from("⏳ извлекаю…"));
            let done = w.as_weak();
            std::thread::spawn(move || {
                let inserted = run_extract();
                let _ = slint::invoke_from_event_loop(move || {
                    EXTRACT_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
                    if let Some(w) = done.upgrade() {
                        w.set_memory_status(SharedString::from(format!("➕ {inserted}")));
                        reload_memory(&w);
                    }
                });
            });
        });
    }
    // v0.16.0 — manual "add your own fact" (personal knowledge base). Inserts
    // straight into the APPROVED items as kind `note` (typing the fact IS the
    // consent the approve step otherwise provides); empty input is a no-op.
    {
        let weak = win.as_weak();
        win.on_memory_add_fact(move |text| {
            let Some(w) = weak.upgrade() else { return };
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }
            let outcome = match open_default_store() {
                Ok(mut store) => store
                    .insert_memory_item(
                        &NewMemoryItem {
                            profile_id: PROFILE.into(),
                            kind: "note".into(),
                            text: trimmed.to_string(),
                            source_session_id: None,
                            source_text: None,
                            entity: None,
                            norm_status: "none".into(),
                        },
                        now_ms(),
                    )
                    .map(|_| ())
                    .map_err(|e| format!("{e:#}")),
                Err(e) => Err(format!("{e:#}")),
            };
            match outcome {
                Ok(()) => {
                    // Clear the input ONLY on a confirmed write — otherwise a DB
                    // failure silently discarded the user's typed fact (audit Q5).
                    w.set_memory_add_text(SharedString::default());
                    w.set_memory_status(SharedString::from("➕ факт добавлен"));
                    reload_memory(&w);
                }
                Err(e) => {
                    eprintln!("[overlay-host] memory add-fact failed: {e}");
                    w.set_memory_status(SharedString::from(
                        "[err] не удалось сохранить факт — попробуйте ещё раз",
                    ));
                }
            }
        });
    }
    // A1 (ТЗ 2026-07-02) — save an inline edit of an approved fact. Empty/whitespace
    // text = "no change" (keep the original, just leave edit mode); a non-empty edit
    // goes through the existing update_memory_item_text, then reload + exit edit mode.
    {
        let weak = win.as_weak();
        win.on_memory_edit_save(move |id, text| {
            let Some(w) = weak.upgrade() else { return };
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                match open_default_store() {
                    Ok(mut store) => {
                        if let Err(e) = store.update_memory_item_text(i64::from(id), trimmed) {
                            eprintln!("[overlay-host] memory edit failed: {e:#}");
                        }
                    }
                    Err(e) => eprintln!("[overlay-host] memory edit: store open failed: {e:#}"),
                }
            }
            w.set_memory_editing_id(-1);
            reload_memory(&w);
        });
    }
}

/// Re-open the catalog, load pending candidates + active items, and push both
/// into the tab's models. Best-effort: a catalog-open failure leaves the lists
/// empty rather than crashing.
pub(crate) fn reload_memory(win: &SettingsWindow) {
    let (cands, items) = match open_default_store() {
        Ok(store) => (
            store
                .list_candidates(PROFILE, "pending", MEMORY_TAB_CAP)
                .unwrap_or_default(),
            store
                .list_memory_items(PROFILE, false, MEMORY_TAB_CAP)
                .unwrap_or_default(),
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
    // Existing candidate texts across ALL statuses → never re-suggest one. One
    // text-only query instead of three unbounded full-row scans (P1-3).
    let mut seen: HashSet<String> = store
        .candidate_texts(PROFILE)
        .unwrap_or_default()
        .into_iter()
        .collect();
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
