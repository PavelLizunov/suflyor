//! 📋-copy / conversation-format LEAF helpers carved out of
//! `tile_controller.rs` (a later wave of the `overlay_host.rs` modularization —
//! see `docs/overlay-host-modularization-plan.md` §5.10 and
//! `docs/overlay-host-current-review.md` §"tile_controller.rs стал новым
//! мини-монолитом").
//!
//! This module owns the pure text-derivation behind the per-tile 📋 copy
//! button plus the follow-up directive plumbing, with their unit tests:
//!
//! - `message_text` — plain text of one chat message (text body, or the text
//!   Part(s) of a vision turn, NEVER the base64 image);
//! - `FOLLOWUP_DIRECTIVE` + `strip_followup_directives` — the marker prepended
//!   to a follow-up's user message (so a weak local model treats it as a DIRECT
//!   question, not transcript noise) and the helper that strips stale copies off
//!   prior turns;
//! - `user_question_for_copy` — peel the `build_request` wrapper off a user turn
//!   so the 📋 copy shows the real question, never the raw Mic/System dump;
//! - `convo_copy_text` / `format_convo_copy` — adaptive copy text (single answer
//!   vs whole labelled 🧑/🤖 thread); `convo_copy_text` reads the
//!   `OverlayBarBridge` conversation map (that bridge stays in
//!   `tile_controller.rs`, reached here through the crate-root glob);
//! - `wire_copy` — wire the 📋 button to write the answer to the Windows
//!   clipboard + flash ✅ (copy is purely local — no network egress, safe under
//!   screen-share / stealth);
//! - the `#[cfg(test)] mod copy_tests` exercising all of the above.
//!
//! SECURITY (unchanged by this move): copy never reaches the network, and the
//! transcript-stripping in `user_question_for_copy` keeps the raw Mic/System
//! lines out of the clipboard.
//!
//! NOTE (§7): this mechanical move imports the parent crate-root via
//! `use super::*;` (it reaches `ai::*`, `OverlayBarBridge`, the Slint
//! `TileWindow`, the clipboard helper, and the `vision` prompts through it).
//! That is intentional for the extraction; the imports get narrowed in a later
//! pass.
use super::{
    ai, vision, Arc, ComponentHandle, Duration, MarkdownBlock, OverlayBarBridge, SharedString,
    TileWindow, Timer, VecModel,
};
use slint::Model;
use std::sync::atomic::{AtomicBool, Ordering};
// `conversations_evict_keys` lives in `tile_controller.rs`; only this module's
// eviction unit test (`copy_tests`) exercises it, so import it TEST-ONLY — a
// plain module-level import would be unused in the normal build (clippy -D).
#[cfg(test)]
use super::conversations_evict_keys;

/// Plain text of one chat message — the `Text` body, or for a vision turn the
/// concatenated text Part(s) only (NEVER the base64 image).
pub(crate) fn message_text(content: &ai::MessageContent) -> String {
    match content {
        ai::MessageContent::Text(t) => t.clone(),
        ai::MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ai::ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Build the clipboard text for the transcript "Copy all / Copy selected" (ТЗ1,
/// decision #7). One reply per line as `Спикер: текст`; with `with_timecodes`,
/// prefixed `[mm:ss] ` (session-relative, derived from `session_start_ms` — only
/// when it is `> 0`). When `selected` is `Some`, only those row indices are
/// included, in chronological (vector) order. Labels match the on-screen
/// transcript (decision #1: Система / Микрофон) and `build_session_markdown`;
/// internal whitespace is collapsed so one utterance = one line. Pure → tested.
///
/// Wired by the ТЗ1 transcript window's "Copy all" button
/// (`aux_windows::wire_transcript_copy`); the per-line "Copy selected" path will
/// pass a populated `selected` set in a later sub-increment.
pub(crate) fn format_transcript_for_copy(
    utts: &[overlay_backend::persistence::Utterance],
    session_start_ms: Option<i64>,
    selected: Option<&std::collections::HashSet<usize>>,
    with_timecodes: bool,
) -> String {
    let mut out = String::new();
    for (i, u) in utts.iter().enumerate() {
        if selected.is_some_and(|sel| !sel.contains(&i)) {
            continue;
        }
        let label = if u.source == "mic" {
            "Микрофон"
        } else {
            "Система"
        };
        let text = u.text.split_whitespace().collect::<Vec<_>>().join(" ");
        // F1: timecode = the line's START (previous line's timestamp; first = origin),
        // matching the on-screen transcript + the player seek.
        let prefix = if with_timecodes {
            overlay_backend::session_audio::line_start_offset_ms(utts, i, session_start_ms)
                .map(|off| format!("[{}] ", super::aux_windows::fmt_offset(off)))
                .unwrap_or_default()
        } else {
            String::new()
        };
        out.push_str(&format!("{prefix}{label}: {text}\n"));
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Strip the `build_request` wrapper from a user turn for the 📋 copy, leaving
/// the actual question. The F9/auto ask bundles the live transcript as AI
/// context ("Транскрипт последних реплик…\n\nПомоги ответить: <q>"), so the real
/// question is the bit after "Помоги ответить:" — without that we'd copy the
/// raw Mic/System transcript lines into the chat copy. A transcript-only F9 ask
/// (no explicit question) → empty, so the noisy transcript is dropped; a typed
/// follow-up is already clean and passes through unchanged.
/// V0.8.3 — prepended to a follow-up's user message sent to the model. The
/// conversation's system prompt frames the assistant as "answer the last
/// question FROM THE TRANSCRIPT", so a bare follow-up was treated as transcript
/// noise and the model re-answered the original (user saw Sonnet reply "Два" to
/// "what is arc raider"). This marker makes the follow-up an explicit DIRECT
/// question. The UI + 📋 copy still show the clean question (it's stripped in
/// user_question_for_copy); the journal logs the raw question.
pub(crate) const FOLLOWUP_DIRECTIVE: &str =
    "Это прямой вопрос пользователя к тебе (НЕ из транскрипта, НЕ предыдущий вопрос). \
     Ответь именно на него: ";

pub(crate) fn user_question_for_copy(raw: &str) -> String {
    let raw = raw.strip_prefix(FOLLOWUP_DIRECTIVE).unwrap_or(raw);
    const MARK: &str = "Помоги ответить:";
    if let Some(i) = raw.rfind(MARK) {
        return raw[i + MARK.len()..].trim().to_string();
    }
    if raw.trim_start().starts_with("Транскрипт последних реплик") {
        return String::new();
    }
    // A vision tile's first user turn is the canned screenshot prompt, not text
    // the user typed — drop it so a multi-turn vision copy doesn't render
    // "🧑 Что на этом скриншоте?…" as if the user had asked it.
    if raw.trim() == vision::DEFAULT_VISION_PROMPT
        || raw.trim().starts_with(vision::TRANSLATE_VISION_PROMPT)
    {
        return String::new();
    }
    raw.trim().to_string()
}

/// Remove the [`FOLLOWUP_DIRECTIVE`] wrapper from the given user turns. Used when
/// building a follow-up / regenerate request so that only the CURRENT question
/// carries the directive. The wrapper is stored verbatim in `conversations`
/// (`handle_ai_event` Done folds `request_messages`), so without this a 3-turn
/// thread would send the model TWO "this is THE direct question" instructions on
/// two different historical turns — and a weak local model then anchors on the
/// wrong one. Non-user turns are left untouched.
pub(crate) fn strip_followup_directives(messages: &mut [ai::ChatMessage]) {
    for m in messages.iter_mut() {
        if m.role != "user" {
            continue;
        }
        let cleaned = match &m.content {
            ai::MessageContent::Text(t) => t.strip_prefix(FOLLOWUP_DIRECTIVE).map(str::to_string),
            _ => None,
        };
        if let Some(c) = cleaned {
            m.content = ai::MessageContent::Text(c);
        }
    }
}

/// V0.8.3 — text for the 📋 copy button. Adaptive so it fits both uses:
///
/// - a single Q→A tile → just the answer (clean paste — the "screenshot →
///   answer → paste it" case);
/// - a multi-turn dialog (a branch) → the WHOLE thread, every question +
///   answer, labelled 🧑 / 🤖 — so a conversation isn't truncated to its last
///   reply (user: "копируется только последнее сообщение, а не весь чат").
///
/// System prompts are skipped; vision turns contribute their text only. Empty
/// if the tile has no (or an unknown / not-yet-seeded) conversation.
pub(crate) fn convo_copy_text(bridge: &OverlayBarBridge, convo_id: i32) -> String {
    let convos = bridge
        .conversations
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    match convos.get(&convo_id) {
        Some(c) => format_convo_copy(&c.messages, &c.rendered),
        None => String::new(),
    }
}

/// Pure formatter behind [`convo_copy_text`] — split out (no bridge / no lock)
/// so the adaptive single-vs-thread logic and the user-turn cleaning are
/// unit-testable. `rendered` is the mid-stream fallback (used when there is no
/// recorded assistant turn yet, or when every turn cleans to empty).
pub(crate) fn format_convo_copy(messages: &[ai::ChatMessage], rendered: &str) -> String {
    let turns: Vec<(&str, String)> = messages
        .iter()
        .filter(|m| m.role != "system")
        .filter_map(|m| {
            let t = message_text(&m.content).trim().to_string();
            (!t.is_empty()).then_some((m.role.as_str(), t))
        })
        .collect();
    if turns.is_empty() {
        return rendered.to_string();
    }
    let assistant_turns = turns.iter().filter(|(r, _)| *r == "assistant").count();
    if assistant_turns <= 1 {
        // Single answer: copy just it (or the rendered body if, mid-stream, no
        // assistant turn is recorded yet).
        return turns
            .iter()
            .rev()
            .find(|(r, _)| *r == "assistant")
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| rendered.to_string());
    }
    let mut out = String::new();
    for (role, text) in &turns {
        // User turns carry the build_request wrapper (transcript + "Помоги
        // ответить:") — copy only the real question, never the Mic/System dump.
        let display = if *role == "assistant" {
            (*text).clone()
        } else {
            user_question_for_copy(text)
        };
        if display.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(if *role == "assistant" {
            "Assistant: "
        } else {
            "You: "
        });
        out.push_str(display.trim());
    }
    if out.is_empty() {
        return rendered.to_string();
    }
    out
}

/// V0.8.3 — wire a tile's copy button: write the answer text to the Windows
/// clipboard and flash feedback for ~1.5 s. Called for every
/// conversational tile (those with a `convo_id`). Copy is purely local — no
/// network egress — so it stays safe under screen-share / stealth.
pub(crate) fn wire_copy(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) {
    tile.set_can_copy(true);
    let weak = tile.as_weak();
    let bridge_c = bridge.clone();
    tile.on_copy_clicked(move || {
        let text = convo_copy_text(&bridge_c, convo_id);
        if text.is_empty() {
            return;
        }
        match clipboard_win::set_clipboard_string(&text) {
            Ok(()) => {
                let Some(t) = weak.upgrade() else {
                    return;
                };
                t.set_copied(true);
                let w = t.as_weak();
                Timer::single_shot(Duration::from_millis(1500), move || {
                    if let Some(t) = w.upgrade() {
                        t.set_copied(false);
                    }
                });
            }
            Err(e) => eprintln!("[overlay-host] clipboard copy failed: {e}"),
        }
    });
}

/// C (ТЗ 2026-07-02) — wire the per-code-block 📋 copy button. STATELESS: the
/// `.slint` hands us the block's index + its CLEAN code (`block.text` is the
/// fence-stripped body — no backticks), we write only that to the clipboard and
/// flash a check on THAT block (`copied-block-index`) for ~1.5 s. Wired from
/// `wire_tile_drag`, the one hook every tile-creation path calls, so it reaches
/// F9 / PTT / vision / auto / content tiles alike. Local copy — no egress, safe
/// under screen-share / stealth (same contract as `wire_copy`).
pub(crate) fn wire_code_copy(tile: &TileWindow) {
    let weak = tile.as_weak();
    tile.on_copy_block_clicked(move |idx, code| {
        if code.is_empty() {
            return;
        }
        match clipboard_win::set_clipboard_string(code.as_str()) {
            Ok(()) => {
                let Some(t) = weak.upgrade() else {
                    return;
                };
                t.set_copied_block_index(idx);
                let w = t.as_weak();
                Timer::single_shot(Duration::from_millis(1500), move || {
                    if let Some(t) = w.upgrade() {
                        // Clear only if THIS block is still the flashed one — a
                        // newer copy of another block moved the marker; don't
                        // clobber its check early.
                        if t.get_copied_block_index() == idx {
                            t.set_copied_block_index(-1);
                        }
                    }
                });
            }
            Err(e) => eprintln!("[overlay-host] code-block copy failed: {e}"),
        }
    });
}

/// A2 (ТЗ 2026-07-02) — persist `text` as an APPROVED memory note (kind "note",
/// profile "default"): the shared write behind BOTH the per-block «⭐ В память» on
/// tiles and the per-line one on the transcript. The user typing / confirming the
/// fact IS the approval consent (same rule as the Settings "add fact"). Empty /
/// whitespace is a no-op. Local SQLite — no egress; errors are logged, not fatal.
///
/// Feature A (condense — M1-b-2): the text — a tile answer OR a raw STT span — is stored
/// INSTANTLY as its heuristic-clean (so the ⭐ feels instant), with the raw kept in `source_text`;
/// then a background thread asks the AI for 1–3 VERBATIM quotes and — only quotes that `locate_span`
/// finds as a contiguous source fragment (P4 quote-span) — swaps in the source slices (`norm_status`
/// → `llm`), else keeps the heuristic text (`heuristic`). Every ⭐/selection save comes here; typed «свой факт» (Settings)
/// does NOT (it's stored verbatim via its own path).
pub(crate) fn insert_approved_note(text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    // Store the heuristic-clean immediately (raw kept as provenance), then condense in the
    // background so the ⭐ never waits on an AI round-trip.
    let raw = trimmed.to_string();
    let cleaned = overlay_backend::memory::heuristic_clean(&raw);
    let cleaned = if cleaned.trim().is_empty() {
        raw.clone()
    } else {
        cleaned
    };
    let Some(id) = store_note(&cleaned, Some(&raw), "pending") else {
        return; // insert failed (already logged)
    };
    let ep = overlay_backend::config::load().ai_endpoint(true);
    if ep.base_url.trim().is_empty() {
        finalize_normalized(id, &raw, &cleaned, None, "heuristic"); // no AI configured → terminal
        return;
    }
    // ponytail: one OS thread per save. Saves are user-initiated + infrequent and ai::complete's
    // AI_SEMAPHORE caps concurrent AI calls, so an unbounded spawn is fine; add a bounded pool
    // only if rapid-fire saving is ever shown to pile up threads.
    std::thread::spawn(move || {
        // P3: a transport Err LEAVES the row 'pending' (not 'heuristic') so `sweep_pending` retries
        // it once the AI is back — the whole offline-reliability fix. Ok(_) is terminal (set inside).
        if let Err(e) = condense_one(id, &raw, &cleaned, &ep) {
            eprintln!("[overlay-host] normalize deferred, left 'pending' (AI offline?): {e:#}");
        }
    });
}

/// Insert one approved note; returns its id (None on failure, logged). Stamps `now` here.
fn store_note(text: &str, source_text: Option<&str>, norm_status: &str) -> Option<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    match overlay_backend::persistence::open_default_store() {
        Ok(mut store) => {
            let item = overlay_backend::persistence::NewMemoryItem {
                profile_id: "default".into(),
                kind: "note".into(),
                text: text.to_string(),
                source_session_id: None,
                source_text: source_text.map(str::to_string),
                entity: None,
                norm_status: norm_status.into(),
            };
            match store.insert_memory_item(&item, now) {
                Ok(id) => Some(id),
                Err(e) => {
                    eprintln!("[overlay-host] add-to-memory failed: {e:#}");
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("[overlay-host] add-to-memory: store open failed: {e:#}");
            None
        }
    }
}

/// Record the normalization outcome on item `id` (text + entity + status). `source_text` is the
/// raw span we normalized — the rowid-reuse guard (see `update_memory_item_normalized`).
/// Best-effort.
fn finalize_normalized(id: i64, source_text: &str, text: &str, entity: Option<&str>, status: &str) {
    match overlay_backend::persistence::open_default_store() {
        Ok(mut store) => {
            if let Err(e) =
                store.update_memory_item_normalized(id, source_text, text, entity, status)
            {
                eprintln!("[overlay-host] normalize update failed: {e:#}");
            }
        }
        Err(e) => eprintln!("[overlay-host] normalize: store open failed: {e:#}"),
    }
}

/// P3 — normalize ONE row SYNCHRONOUSLY (blocks the calling thread on a current-thread runtime; the
/// caller owns the thread). `Ok(())` = a TERMINAL outcome was recorded (`llm` if a fact located, else
/// `heuristic`); `Err` = the AI CALL failed (offline/timeout, or runtime build failed) → the row is
/// LEFT `'pending'` for a later retry. The single normalize+finalize path shared by the per-save
/// worker and [`sweep_pending`]. `cleaned` is the heuristic fallback; `ep` the resolved endpoint
/// (local OR cloud — provider-agnostic, same egress class as answers on a cloud provider).
fn condense_one(
    id: i64,
    raw: &str,
    cleaned: &str,
    ep: &overlay_backend::config::AiEndpoint,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // The inner `?` propagates a transport Err (→ stays 'pending'); Ok(None) is terminal 'heuristic'.
    match rt.block_on(overlay_backend::memory::normalize_fact(
        raw,
        &ep.base_url,
        &ep.bearer,
        &ep.model,
    ))? {
        Some(fact) => finalize_normalized(id, raw, &fact.text, fact.entity.as_deref(), "llm"),
        None => finalize_normalized(id, raw, cleaned, None, "heuristic"),
    }
    Ok(())
}

/// Re-entrancy guard for [`sweep_pending`]: boot AND every Настройки→Память open trigger it, so this
/// keeps two sweeps from double-condensing the same rows at once.
static SWEEP_RUNNING: AtomicBool = AtomicBool::new(false);

/// Clears [`SWEEP_RUNNING`] on EVERY exit of the sweep thread (early return, break, or panic).
struct SweepGuard;
impl Drop for SweepGuard {
    fn drop(&mut self) {
        SWEEP_RUNNING.store(false, Ordering::Release);
    }
}

/// P3 offline-reliability (D1 + D4): retry every memory row still `'pending'` — a save whose AI
/// condense never completed because the AI was offline at save time. ONE background thread, SEQUENTIAL,
/// and ABORTS on the first transport `Err` (AI still down → don't hammer it; the next trigger resumes).
/// Re-entrancy-guarded. Fires on boot (after a short delay so network/AI can settle) and on each
/// Настройки→Память open. No-op when no AI is configured (rows stay honestly `'pending'`).
pub(crate) fn sweep_pending() {
    if SWEEP_RUNNING.swap(true, Ordering::AcqRel) {
        return; // a sweep is already in flight
    }
    std::thread::spawn(move || {
        let _guard = SweepGuard; // release SWEEP_RUNNING however this thread exits
        let ep = overlay_backend::config::load().ai_endpoint(true);
        if ep.base_url.trim().is_empty() {
            return; // no AI → nothing to retry against
        }
        let pending = list_pending();
        if pending.is_empty() {
            return;
        }
        eprintln!(
            "[overlay-host] memory sweep: {} pending row(s)",
            pending.len()
        );
        for (id, raw, cleaned) in pending {
            if let Err(e) = condense_one(id, &raw, &cleaned, &ep) {
                eprintln!(
                    "[overlay-host] memory sweep: AI offline, stopping ({e:#}) — retry later"
                );
                break; // AI still down — stop; the next trigger picks up the rest
            }
        }
    });
}

/// The `'pending'` notes we can retry: `(id, raw source_text, heuristic-clean fallback)`. Rows with no
/// `source_text` (pre-M1) are skipped — no raw to re-normalize. Read-only; on error → empty (logged).
fn list_pending() -> Vec<(i64, String, String)> {
    let store = match overlay_backend::persistence::open_default_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[overlay-host] memory sweep: store open failed: {e:#}");
            return Vec::new();
        }
    };
    // include_archived: an archived row still deserves its clean text; cap is a sanity bound (5000
    // offline saves is absurd — it just stops a pathological unbounded scan).
    match store.list_memory_items("default", true, 5000) {
        Ok(items) => items
            .into_iter()
            .filter(|it| it.norm_status == "pending")
            .filter_map(|it| it.source_text.map(|raw| (it.id, raw, it.text)))
            .collect(),
        Err(e) => {
            eprintln!("[overlay-host] memory sweep: list failed: {e:#}");
            Vec::new()
        }
    }
}

/// A2 (ТЗ 2026-07-02, redesign) — wire the per-block MARK → save flow on a tile.
/// `toggle-block-marked(i)` flips block i's `marked` in the live blocks model and
/// maintains `marked-count` (seeding `capture-text` when exactly one is marked, for
/// the trim-before-save editor). `save-marked` writes every marked block as an
/// approved note (single-marked uses the edited `capture-text`), then clears marks.
/// `clear-marks` cancels. Wired via `wire_tile_drag` (the universal per-tile hook).
pub(crate) fn wire_block_capture(tile: &TileWindow) {
    {
        let weak = tile.as_weak();
        tile.on_toggle_block_marked(move |idx, shift| {
            let Some(t) = weak.upgrade() else { return };
            let blocks = t.get_blocks();
            let Some(vm) = blocks.as_any().downcast_ref::<VecModel<MarkdownBlock>>() else {
                return;
            };
            let Ok(i) = usize::try_from(idx) else { return };
            if i >= vm.row_count() {
                return;
            }
            // SHIFT+click with a live anchor → mark the whole contiguous range anchor..=i (ADD to the
            // marked set, don't clear others); anchor stays put so repeated shift-clicks re-extend from
            // the same origin. Otherwise a plain click toggles this block and (re)sets the anchor here.
            // The anchor is a Slint prop reset by `changed blocks` alongside the marks, so a streaming
            // model swap can't leave a stale anchor that mis-marks a range on the fresh answer.
            let shift_anchor = if shift {
                usize::try_from(t.get_mark_anchor()).ok()
            } else {
                None
            }
            .filter(|&a| a < vm.row_count());
            if let Some(a) = shift_anchor {
                let (lo, hi) = if a <= i { (a, i) } else { (i, a) };
                for j in lo..=hi {
                    if let Some(mut r) = vm.row_data(j) {
                        if !r.marked {
                            r.marked = true;
                            vm.set_row_data(j, r);
                        }
                    }
                }
            } else {
                if let Some(mut row) = vm.row_data(i) {
                    row.marked = !row.marked;
                    vm.set_row_data(i, row);
                }
                t.set_mark_anchor(i32::try_from(i).unwrap_or(-1));
            }
            // Recompute the count + the sole-marked index (when exactly one).
            let mut count = 0_i32;
            let mut single_idx = -1_i32;
            let mut single_text = SharedString::default();
            for j in 0..vm.row_count() {
                if let Some(r) = vm.row_data(j) {
                    if r.marked {
                        count += 1;
                        single_idx = i32::try_from(j).unwrap_or(-1);
                        single_text = r.text.clone();
                    }
                }
            }
            t.set_marked_count(count);
            // Switching to the block-mark flow cancels any pending mouse-selection
            // capture (they share the one bottom editor bar — keep them exclusive).
            t.set_capture_pending(false);
            // Seed the edit buffer ONLY when the SOLE-marked block CHANGES (review
            // I-1): an in-progress edit then survives marking/un-marking OTHER blocks
            // (sole block unchanged → no re-seed), while switching which single block
            // is marked re-seeds to the new block's text — so the buffer never
            // mismatches the marked block.
            if count == 1 && single_idx != t.get_capture_block_index() {
                t.set_capture_text(single_text);
                t.set_capture_block_index(single_idx);
            }
        });
    }
    {
        let weak = tile.as_weak();
        tile.on_save_marked(move || {
            let Some(t) = weak.upgrade() else { return };
            let blocks = t.get_blocks();
            let Some(vm) = blocks.as_any().downcast_ref::<VecModel<MarkdownBlock>>() else {
                return;
            };
            // Single mark → the (possibly edited) buffer. Multiple → ONE joined record
            // (G2a, ТЗ part 2): related facts stop fragmenting into separate rows — the
            // «z14-backup → имя / подсеть / IP = 3 rows» complaint. Refine later in
            // Настройки → Память (A1). "; " sep keeps the joined note single-line-editable.
            if t.get_marked_count() == 1 {
                insert_approved_note(t.get_capture_text().as_str());
            } else {
                insert_approved_note(&join_marked_text(vm));
            }
            clear_all_marks(vm);
            t.set_marked_count(0);
            t.set_capture_block_index(-1);
        });
    }
    {
        let weak = tile.as_weak();
        tile.on_clear_marks(move || {
            let Some(t) = weak.upgrade() else { return };
            let blocks = t.get_blocks();
            if let Some(vm) = blocks.as_any().downcast_ref::<VecModel<MarkdownBlock>>() {
                clear_all_marks(vm);
            }
            t.set_marked_count(0);
            t.set_capture_block_index(-1);
        });
    }
    // ТЗ 2026-07-03 — mouse-selection capture: slice the block's text by the
    // selection's byte offsets (the drag may run either way, so order them), seed the
    // edit buffer, and open the pending editor. Marks are cleared so the single bottom
    // bar is unambiguous. The user editing/confirming the span IS the approval consent.
    {
        let weak = tile.as_weak();
        tile.on_capture_selection(move |idx, a, c| {
            let Some(t) = weak.upgrade() else { return };
            // Source: the clicked block's text. Owned so the `&str` slice below outlives the
            // temporary model borrow.
            let blocks = t.get_blocks();
            let Some(vm) = blocks.as_any().downcast_ref::<VecModel<MarkdownBlock>>() else {
                return;
            };
            let Ok(i) = usize::try_from(idx) else { return };
            let src = match vm.row_data(i) {
                Some(row) => row.text,
                None => return,
            };
            let text = src.as_str();
            let (lo, hi) = if a <= c { (a, c) } else { (c, a) };
            let lo = char_boundary(text, usize::try_from(lo).unwrap_or(0));
            let hi = char_boundary(text, usize::try_from(hi).unwrap_or(0));
            let span = text.get(lo..hi).unwrap_or("").trim();
            if span.is_empty() {
                return;
            }
            // Clear any block marks so only the selection editor shows.
            if let Some(vm) = t
                .get_blocks()
                .as_any()
                .downcast_ref::<VecModel<MarkdownBlock>>()
            {
                clear_all_marks(vm);
            }
            t.set_marked_count(0);
            t.set_capture_block_index(-1);
            t.set_capture_text(span.into());
            t.set_capture_pending(true);
        });
    }
    {
        // P5 — «Копировать» on the mark bar: join every marked block (block order, "; " — the same
        // join as save-marked) and write it to the clipboard. Non-destructive: the marks stay.
        let weak = tile.as_weak();
        tile.on_copy_marked(move || {
            let Some(t) = weak.upgrade() else { return };
            let blocks = t.get_blocks();
            let Some(vm) = blocks.as_any().downcast_ref::<VecModel<MarkdownBlock>>() else {
                return;
            };
            // Mirror save-marked EXACTLY: a single mark copies the (possibly edited) trim buffer, so
            // Copy never diverges from what the editor shows / To-memory would store; N>1 joins.
            let text = if t.get_marked_count() == 1 {
                t.get_capture_text().to_string()
            } else {
                join_marked_text(vm)
            };
            if text.trim().is_empty() {
                return;
            }
            if let Err(e) = clipboard_win::set_clipboard_string(&text) {
                eprintln!("[overlay-host] copy marked failed: {e}");
            }
        });
    }
    {
        let weak = tile.as_weak();
        tile.on_save_capture(move || {
            let Some(t) = weak.upgrade() else { return };
            insert_approved_note(t.get_capture_text().as_str());
            t.set_capture_pending(false);
            t.set_capture_text(SharedString::default());
        });
    }
    {
        let weak = tile.as_weak();
        tile.on_cancel_capture(move || {
            let Some(t) = weak.upgrade() else { return };
            t.set_capture_pending(false);
            t.set_capture_text(SharedString::default());
        });
    }
}

/// Clamp a byte index down to the nearest char boundary ≤ `i` (and ≤ len). Slint's
/// selection byte-offsets already land on boundaries; this is a defensive guard so
/// slicing the source text can never panic on a multibyte (Cyrillic) string. Shared
/// with the transcript selection-capture in `aux_windows`.
pub(crate) fn char_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Un-mark every block in the model (shared by save + cancel).
fn clear_all_marks(vm: &VecModel<MarkdownBlock>) {
    for j in 0..vm.row_count() {
        if let Some(mut r) = vm.row_data(j) {
            if r.marked {
                r.marked = false;
                vm.set_row_data(j, r);
            }
        }
    }
}

/// G2a (ТЗ part 2) — join every marked block's text into ONE record (in block order,
/// `"; "` separated, single-line-editable) so a multi-⭐ save is one coherent fact, not
/// N fragmented rows. Pure → tested.
fn join_marked_text(vm: &VecModel<MarkdownBlock>) -> String {
    let mut out = String::new();
    for j in 0..vm.row_count() {
        if let Some(r) = vm.row_data(j) {
            if r.marked {
                if !out.is_empty() {
                    out.push_str("; ");
                }
                out.push_str(r.text.as_str());
            }
        }
    }
    out
}

/// Text for the 🔊 read-aloud: the LATEST assistant answer only — never the user
/// prompts / transcript / earlier turns. (The 📋 copy deliberately includes the
/// whole labelled thread; read-aloud must NOT, or it speaks your own questions
/// back at you — the bug the tester hit.) Falls back to the rendered body
/// mid-stream, before an assistant turn is recorded.
pub(crate) fn convo_speak_text(bridge: &OverlayBarBridge, convo_id: i32) -> String {
    let convos = bridge
        .conversations
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    match convos.get(&convo_id) {
        Some(c) => speak_answer_text(&c.messages, &c.rendered),
        None => String::new(),
    }
}

/// Pure: the latest assistant turn's text, or the rendered body if none yet.
pub(crate) fn speak_answer_text(messages: &[ai::ChatMessage], rendered: &str) -> String {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| message_text(&m.content).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| rendered.trim().to_string())
}

// Which tile is currently being read aloud. TTS is process-global +
// one-utterance-at-a-time, so we remember the convo_id that started the current
// speech: closing THAT tile (or the app) stops it; a new speak re-points it.
static SPEAKING_CONVO: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);

/// Record that `convo_id` started the current read-aloud.
pub(crate) fn mark_speaking(convo_id: i32) {
    SPEAKING_CONVO.store(convo_id as i64, std::sync::atomic::Ordering::Release);
}

/// Stop the read-aloud iff `convo_id` is the tile currently being spoken — called
/// from each tile's close handler so closing the speaking tile silences it.
pub(crate) fn stop_if_speaking(convo_id: i32) {
    if SPEAKING_CONVO.load(std::sync::atomic::Ordering::Acquire) == convo_id as i64 {
        overlay_backend::tts::stop();
        SPEAKING_CONVO.store(i64::MIN, std::sync::atomic::Ordering::Release);
    }
}

/// The convo_id currently being read aloud, or -1 if none.
pub(crate) fn current_speaking_convo() -> i32 {
    let v = SPEAKING_CONVO.load(std::sync::atomic::Ordering::Acquire);
    if v == i64::MIN {
        -1
    } else {
        v as i32
    }
}

// Process-global pause latch, shared by the tile ⏯ button AND the Shift+Alt+3
// hotkey so they stay coherent (TTS is global + one-at-a-time). false = playing.
static SPEAK_PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Toggle pause/resume of the current read-aloud; returns the NEW paused state.
pub(crate) fn toggle_pause() -> bool {
    let now_paused = !SPEAK_PAUSED.fetch_xor(true, std::sync::atomic::Ordering::SeqCst);
    if now_paused {
        overlay_backend::tts::pause();
    } else {
        overlay_backend::tts::resume();
    }
    now_paused
}

/// Reset the latch to "playing" — called when a fresh utterance starts.
pub(crate) fn reset_pause() {
    SPEAK_PAUSED.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// Read-aloud — wire a tile's 🔊 «Озвучить» + ⏯ pause/resume controls to the
/// process-global neural TTS sidecar. Speaks ONLY the latest answer (never the
/// prompts / earlier turns). Purely local — no network egress — so it stays safe
/// under screen-share / stealth.
pub(crate) fn wire_speak(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) {
    tile.set_can_speak(true);
    let bridge_speak = bridge.clone();
    {
        let weak = tile.as_weak();
        tile.on_speak_clicked(move || {
            let text = convo_speak_text(&bridge_speak, convo_id);
            if text.trim().is_empty() {
                return;
            }
            reset_pause();
            if let Some(t) = weak.upgrade() {
                t.set_speak_paused(false);
            }
            mark_speaking(convo_id);
            overlay_backend::tts::speak(&text);
        });
    }
    let weak_p = tile.as_weak();
    tile.on_speak_pause_clicked(move || {
        // Shared global latch so this ⏯ button and the Shift+Alt+3 hotkey stay
        // coherent (one TTS engine, one utterance at a time).
        let now_paused = toggle_pause();
        if let Some(t) = weak_p.upgrade() {
            t.set_speak_paused(now_paused);
        }
    });
}

#[cfg(test)]
mod copy_tests {
    //! Locks the 📋-copy text derivation — the exact area the user hit live:
    //! copy pulling in the raw Mic/System transcript, and follow-ups being
    //! re-answered as the original question. Pure: no bridge, no UI, no network.
    use super::*;

    #[test]
    fn char_boundary_clamps_into_multibyte() {
        // Cyrillic: each letter is 2 bytes (а = 0..2, б = 2..4). A mouse selection's
        // byte offsets must never slice mid-char (would panic).
        let s = "аб";
        assert_eq!(char_boundary(s, 0), 0);
        assert_eq!(char_boundary(s, 1), 0, "mid-char clamps down");
        assert_eq!(char_boundary(s, 2), 2);
        assert_eq!(char_boundary(s, 3), 2, "mid-char clamps down");
        assert_eq!(char_boundary(s, 4), 4);
        assert_eq!(char_boundary(s, 99), 4, "past end clamps to len");
        // The span a selection would take is always a valid slice.
        assert_eq!(&s[char_boundary(s, 1)..char_boundary(s, 3)], "а");
    }

    #[test]
    fn join_marked_text_combines_marked_only_in_order() {
        let mk = |text: &str, marked: bool| MarkdownBlock {
            kind: 0,
            text: text.into(),
            lang: "".into(),
            marked,
        };
        let vm = VecModel::from(vec![
            mk("Имя: z14-4443-backup", true),
            mk("не отмечено", false),
            mk("Подсеть: 10.255.28.96/27", true),
            mk("IP: 10.255.28.116", true),
        ]);
        // G2a — ONE record, "; "-joined, marked-only, in block order (un-fragmenting the
        // z14-backup «имя / подсеть / IP = 3 rows» example).
        assert_eq!(
            join_marked_text(&vm),
            "Имя: z14-4443-backup; Подсеть: 10.255.28.96/27; IP: 10.255.28.116"
        );
    }

    #[test]
    fn transcript_copy_format() {
        use overlay_backend::persistence::Utterance;
        let start = 1000_i64;
        let utts = vec![
            Utterance {
                session_id: "s".into(),
                unix_ms: start + 29_000, // finalized ~00:29 into the session (≈ its end)
                source: "system".into(),
                text: "привет  мир".into(), // double space collapses
                audio_ms: None,
            },
            Utterance {
                session_id: "s".into(),
                unix_ms: start + 135_000,
                source: "mic".into(),
                text: "да".into(),
                audio_ms: None,
            },
        ];
        // Default: "Спикер: текст", no timecodes, all lines, no trailing newline.
        assert_eq!(
            format_transcript_for_copy(&utts, Some(start), None, false),
            "Система: привет мир\nМикрофон: да"
        );
        // With timecodes — F1: a line's START = the PREVIOUS line's timestamp; the
        // FIRST line is 00:00 (NOT its own finalize time 00:29), so line 2 starts
        // where line 1 ended (00:29).
        assert_eq!(
            format_transcript_for_copy(&utts, Some(start), None, true),
            "[00:00] Система: привет мир\n[00:29] Микрофон: да"
        );
        // Selected subset (only row 1), chronological order.
        let mut sel = std::collections::HashSet::new();
        sel.insert(1_usize);
        assert_eq!(
            format_transcript_for_copy(&utts, Some(start), Some(&sel), false),
            "Микрофон: да"
        );
        // Empty transcript → empty string.
        assert_eq!(
            format_transcript_for_copy(&[], Some(start), None, false),
            ""
        );
        // with_timecodes but no session start → no prefix.
        assert_eq!(
            format_transcript_for_copy(&utts[..1], None, None, true),
            "Система: привет мир"
        );
    }

    fn msg(role: &str, text: &str) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.to_string(),
            content: ai::MessageContent::Text(text.to_string()),
        }
    }
    fn parts_msg(role: &str, texts: &[&str]) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.to_string(),
            content: ai::MessageContent::Parts(
                texts
                    .iter()
                    .map(|t| ai::ContentPart::Text {
                        text: (*t).to_string(),
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn message_text_text_and_parts() {
        assert_eq!(
            message_text(&ai::MessageContent::Text("plain".into())),
            "plain"
        );
        // Parts: text parts are joined (image parts, when present, contribute
        // nothing — exercised here with two text parts).
        let m = parts_msg("user", &["hello", "world"]);
        assert_eq!(message_text(&m.content), "hello\nworld");
    }

    #[test]
    fn copy_question_strips_transcript_wrapper() {
        let raw = "Транскрипт последних реплик:\n[СОБЕСЕДНИК] arc raiders?\n\n\
                   Помоги ответить: что такое arc raiders";
        assert_eq!(user_question_for_copy(raw), "что такое arc raiders");
    }

    #[test]
    fn conversations_evict_keys_drops_oldest_half_keeps_newest() {
        // FIX #8 — at the cap, the lowest-id half (oldest tiles) is evicted,
        // and the highest ids (newest / currently-open tiles) are kept.
        let keys: Vec<i32> = (0..256).collect();
        let evicted = conversations_evict_keys(&keys, 256);
        assert_eq!(evicted.len(), 128, "evicts exactly half the cap");
        assert_eq!(evicted.first(), Some(&0), "oldest id is evicted");
        assert_eq!(evicted.last(), Some(&127), "eviction stops at the midpoint");
        assert!(
            !evicted.contains(&255),
            "the newest id (an open tile) is never evicted"
        );
        // Unsorted input is handled (HashMap key order is arbitrary).
        let shuffled = [50, 3, 200, 7, 99];
        let mut e = conversations_evict_keys(&shuffled, 4); // max/2 = 2 → drop 2 lowest
        e.sort_unstable();
        assert_eq!(
            e,
            vec![3, 7],
            "drops the two lowest ids regardless of order"
        );
    }

    #[test]
    fn copy_question_drops_transcript_only_ask() {
        let raw = "Транскрипт последних реплик:\n[СОБЕСЕДНИК] что-то сказал";
        assert_eq!(user_question_for_copy(raw), "");
    }

    #[test]
    fn copy_question_strips_followup_directive() {
        let raw = format!("{FOLLOWUP_DIRECTIVE}а что дальше?");
        assert_eq!(user_question_for_copy(&raw), "а что дальше?");
    }

    #[test]
    fn copy_question_drops_canned_vision_prompt() {
        assert_eq!(user_question_for_copy(vision::DEFAULT_VISION_PROMPT), "");
    }

    #[test]
    fn copy_question_drops_translate_vision_prompt() {
        // Feature #3 — a translate tile's first turn is the canned translate
        // prompt, not user-typed text → drop it (both phonetics states; the ON
        // variant is base+suffix, so starts_with the base still matches).
        assert_eq!(user_question_for_copy(vision::TRANSLATE_VISION_PROMPT), "");
        assert_eq!(user_question_for_copy(&vision::translate_prompt(true)), "");
    }

    #[test]
    fn copy_question_passes_plain_text_trimmed() {
        assert_eq!(user_question_for_copy("  привет  "), "привет");
    }

    #[test]
    fn single_turn_copies_only_the_answer() {
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg("user", "Помоги ответить: что такое Rust"),
            msg("assistant", "Rust — системный язык."),
        ];
        assert_eq!(
            format_convo_copy(&msgs, "RENDERED"),
            "Rust — системный язык."
        );
    }

    #[test]
    fn multi_turn_copies_labelled_thread_without_transcript() {
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg(
                "user",
                "Транскрипт последних реплик:\n[СОБЕСЕДНИК] x\n\nПомоги ответить: вопрос 1",
            ),
            msg("assistant", "ответ 1"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос 2")),
            msg("assistant", "ответ 2"),
        ];
        let out = format_convo_copy(&msgs, "RENDERED");
        assert_eq!(
            out,
            "You: вопрос 1\n\nAssistant: ответ 1\n\nYou: вопрос 2\n\nAssistant: ответ 2"
        );
        // The raw Mic/System transcript must never reach the clipboard.
        assert!(!out.contains("СОБЕСЕДНИК"));
    }

    #[test]
    fn multi_turn_vision_skips_canned_prompt() {
        let msgs = vec![
            parts_msg("user", &[vision::DEFAULT_VISION_PROMPT]),
            msg("assistant", "на экране код"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}а на каком языке?")),
            msg("assistant", "на Rust"),
        ];
        let out = format_convo_copy(&msgs, "RENDERED");
        assert_eq!(
            out,
            "Assistant: на экране код\n\nYou: а на каком языке?\n\nAssistant: на Rust"
        );
    }

    #[test]
    fn empty_conversation_falls_back_to_rendered() {
        assert_eq!(format_convo_copy(&[], "RENDERED"), "RENDERED");
    }

    #[test]
    fn speak_reads_latest_answer_only_not_prompts_or_old_turns() {
        // The tester bug: 🔊 on a multi-turn tile read the prompts + every
        // message. Read-aloud must speak ONLY the latest answer.
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg("user", "Помоги ответить: вопрос 1"),
            msg("assistant", "ответ один"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос 2")),
            msg("assistant", "ответ два"),
        ];
        let spoken = speak_answer_text(&msgs, "RENDERED");
        assert_eq!(spoken, "ответ два");
        assert!(!spoken.contains("вопрос"));
        assert!(!spoken.contains("ответ один"));
    }

    #[test]
    fn speak_falls_back_to_rendered_before_any_answer() {
        let msgs = vec![msg("user", "Помоги ответить: q")];
        assert_eq!(
            speak_answer_text(&msgs, "частичный ответ"),
            "частичный ответ"
        );
        assert_eq!(speak_answer_text(&[], "RENDERED"), "RENDERED");
    }

    #[test]
    fn strip_directives_cleans_user_turns_only() {
        let mut msgs = [
            msg("system", &format!("{FOLLOWUP_DIRECTIVE}sys")),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос")),
            msg("assistant", &format!("{FOLLOWUP_DIRECTIVE}ответ")),
            msg("user", "уже чистый"),
        ];
        strip_followup_directives(&mut msgs);
        // system + assistant turns are untouched (only user turns get cleaned).
        assert_eq!(
            message_text(&msgs[0].content),
            format!("{FOLLOWUP_DIRECTIVE}sys")
        );
        assert_eq!(
            message_text(&msgs[2].content),
            format!("{FOLLOWUP_DIRECTIVE}ответ")
        );
        // user turns are stripped; an already-clean one is unchanged.
        assert_eq!(message_text(&msgs[1].content), "вопрос");
        assert_eq!(message_text(&msgs[3].content), "уже чистый");
    }

    #[test]
    fn strip_all_but_last_preserves_reasked_turn() {
        // Mirrors fire_regenerate's `&mut messages[..len-1]`: prior turns are
        // cleaned, but the last (re-asked) turn keeps whatever framing it had.
        let mut msgs = [
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}старый вопрос")),
            msg("assistant", "старый ответ"),
            msg(
                "user",
                &format!("{FOLLOWUP_DIRECTIVE}перезапрашиваемый вопрос"),
            ),
        ];
        let n = msgs.len() - 1;
        strip_followup_directives(&mut msgs[..n]);
        // Prior user turn is cleaned…
        assert_eq!(message_text(&msgs[0].content), "старый вопрос");
        // …but the last (re-asked) turn keeps its direct-question framing.
        assert_eq!(
            message_text(&msgs[2].content),
            format!("{FOLLOWUP_DIRECTIVE}перезапрашиваемый вопрос")
        );
    }
}
