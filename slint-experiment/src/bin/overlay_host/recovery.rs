//! Crash-recovery: the recovered-context string composition + the on-demand
//! recovery-offer window (Phase 4 of the `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.5).
//!
//! This module owns the secret-free recovered-context block (`HEADER … FOOTER`)
//! that gets PREPENDED to `cfg.meeting_context` so STT's whisper prompt + every
//! AI ask pick up what we recovered from an unfinished session: the pure,
//! side-effect-free string core (`build_recovery_block` / `strip_recovery_block`
//! / `compose_recovery_context`, locked by the tests at the bottom) plus the
//! config-persisting `seed_recovery_context` wrapper. The strip-on-reseed guard
//! keeps a crash→recover→crash loop from stacking ever-older Q&A in the live
//! system prompt.
//!
//! It also owns `open_recover_offer`, the crash-recovery offer WINDOW (mirrors
//! `open_text_ask`: scheme-themed, stealth-aware, parked-before-shown). NOTE:
//! the recovery FEATURE is gated off behind the `SLINT_OVERLAY_RECOVERY` env in
//! `main`, but the code is live and moved here UNCHANGED (MECHANICAL move, §2 —
//! no behaviour change, the gating stays exactly as-is in `overlay_host.rs`).
//!
//! Callers that STAY in `overlay_host.rs` — the ask / follow-up path that calls
//! `strip_recovery_block` / `compose_recovery_context`, and `main`'s delayed
//! `open_recover_offer` Timer — resolve through the `use recovery::*;` re-export
//! at crate root.
//!
//! NOTE (§7): the parent crate-root symbols this module references are imported
//! explicitly below.
use super::{
    clamp_scheme, focus_window, global_scheme, grab_hwnd, present_window_stealth_aware,
    slint_session, ui, Arc, ComponentHandle, OverlayBarWindow, Rc, RecoverOfferWindow, RefCell,
    RuntimeEvents, SharedSlintRuntime, SharedString,
};

/// Opening marker that brackets the recovered-context block prepended to
/// `cfg.meeting_context` by [`seed_recovery_context`]. A stable, greppable
/// delimiter so [`strip_recovery_block`] can find + remove exactly this block
/// on reseed (and a future "clear recovered context" affordance could too)
/// without touching the user's own prose.
pub(crate) const RECOVERY_CONTEXT_HEADER: &str = "=== Контекст из прошлой сессии ===";

/// Closing marker paired with [`RECOVERY_CONTEXT_HEADER`]. Kept as its own
/// const (not an inline literal) so [`strip_recovery_block`] can locate the
/// exact end of a previously-seeded block — the two together define the
/// strip-on-reseed contract.
pub(crate) const RECOVERY_CONTEXT_FOOTER: &str = "=== Конец контекста из прошлой сессии ===";

/// Build the secret-free recovered-context block (header … footer) from an
/// [`UnfinishedSession`]. Split out of [`seed_recovery_context`] so the
/// composition is unit-testable without any config / disk side effects. Only
/// user content (last Q&A, recent lines, local summary) goes in — never
/// secrets — matching the secret-free-logging boundary the rest of recovery
/// keeps.
pub(crate) fn build_recovery_block(
    recovered: &overlay_backend::journal::UnfinishedSession,
) -> String {
    let mut block = String::new();
    block.push_str(RECOVERY_CONTEXT_HEADER);
    block.push('\n');
    if let Some((q, a)) = &recovered.last_qa {
        block.push_str("Последний вопрос: ");
        block.push_str(q.trim());
        block.push('\n');
        block.push_str("Последний ответ: ");
        block.push_str(a.trim());
        block.push('\n');
    }
    if !recovered.last_lines.is_empty() {
        block.push_str("Недавние реплики:\n");
        for line in &recovered.last_lines {
            block.push_str("- ");
            block.push_str(line.trim());
            block.push('\n');
        }
    }
    if let Some(s) = &recovered.summary {
        block.push_str("Итог прошлой сессии: ");
        block.push_str(s.trim());
        block.push('\n');
    }
    block.push_str(RECOVERY_CONTEXT_FOOTER);
    block
}

/// Strip-on-reseed guard (NIGHT_RUN_PLAN Phase 2). Remove every previously-
/// seeded recovery block (each a `HEADER … FOOTER` span) from `context`,
/// returning the surrounding user prose. Without this, each crash→recover
/// cycle would prepend ANOTHER block: `meeting_context` would grow without
/// bound and the live AI system prompt (which reads it) would accrete ever-
/// older "last question / answer" blocks.
///
/// - No header → returns `context` unchanged (the first-seed path, byte-for-byte).
/// - Header with no matching footer → stops there; never guesses where a
///   half-written block ends, leaving the remainder untouched.
/// - One or more blocks → removes each header..=footer span AND the blank-line
///   separator the seed inserts, so the user's own prose is neither duplicated
///   nor pushed further down on every reseed. (Loops so a config that already
///   stacked blocks under the pre-guard behaviour is collapsed to clean prose.)
///
/// Delimiter-based by design (matching Phase 1's seed): the markers are long,
/// specific sentinels, so the negligible risk that a user's OWN prose holds
/// BOTH literals verbatim — and is therefore mis-stripped — is accepted.
/// Recovered fields are the user's own past transcript, never adversarial.
pub(crate) fn strip_recovery_block(context: &str) -> String {
    let is_newline = |c: char| c == '\n' || c == '\r';
    let mut out = context.to_string();
    while let Some(start) = out.find(RECOVERY_CONTEXT_HEADER) {
        let after_header = start + RECOVERY_CONTEXT_HEADER.len();
        let Some(rel_footer) = out[after_header..].find(RECOVERY_CONTEXT_FOOTER) else {
            break;
        };
        let end = after_header + rel_footer + RECOVERY_CONTEXT_FOOTER.len();
        // Prose on either side of the block. Trim only the line breaks abutting
        // the block (the seed writes "block\n\nprose"; a leftover blank line
        // must not accumulate across reseeds) — the prose itself is preserved.
        let before = out[..start].trim_end_matches(is_newline);
        let after = out[end..].trim_start_matches(is_newline);
        let next = match (before.is_empty(), after.is_empty()) {
            (true, _) => after.to_string(),
            (_, true) => before.to_string(),
            (false, false) => format!("{before}\n\n{after}"),
        };
        out = next;
    }
    out
}

/// Pure core of [`seed_recovery_context`]: strip any prior recovery block(s)
/// out of `existing_context`, then prepend a fresh one built from `recovered`.
/// Returns the new combined context. Side-effect-free (no config lock, no
/// disk) so the "exactly one block after reseed" invariant is unit-testable.
pub(crate) fn compose_recovery_context(
    existing_context: &str,
    recovered: &overlay_backend::journal::UnfinishedSession,
) -> String {
    let block = build_recovery_block(recovered);
    let existing = strip_recovery_block(existing_context);
    if existing.trim().is_empty() {
        block
    } else {
        format!("{block}\n\n{existing}")
    }
}

/// Memory Phase 1 — seed the LIVE meeting context with what we recovered from
/// the unfinished session, so STT's whisper prompt + every AI ask pick it up
/// (they all read `cfg.meeting_context`). We PREPEND a clearly-delimited block
/// and keep the user's existing context intact below it. Before prepending we
/// STRIP any block a PRIOR recovery left (see [`strip_recovery_block`]) so a
/// crash→recover→crash loop can't stack stale blocks. Persisted via
/// `config::save` + mirrored into the active profile (same path the Profile
/// editor uses) so it survives a restart and the picker never drifts; because
/// `save_active_context` rewrites BOTH the live field and the active profile's
/// copy with the recomposed text, the strip cleans the profile mirror too.
///
/// Returns the new total `meeting_context` char count (for the journal /
/// diagnostics). Does NOT touch tiles, mic, screenshots, or any in-flight
/// state — per the spec, only the textual context is carried.
pub(crate) fn seed_recovery_context(
    cfg: &overlay_backend::config::SharedConfig,
    recovered: &overlay_backend::journal::UnfinishedSession,
) -> usize {
    let mut c = cfg.write();
    let combined = compose_recovery_context(&c.meeting_context, recovered);
    // save_active_context updates meeting_context AND mirrors into the active
    // profile, matching the Profile editor's persistence path.
    c.save_active_context(&combined);
    let chars = c.meeting_context.chars().count();
    if let Err(e) = overlay_backend::config::save(&c) {
        // Non-fatal: the in-memory seed already applies to this session; we
        // just couldn't persist it. NEVER log context text (user content).
        eprintln!("[overlay-host] recovery context persist failed (non-fatal): {e:#}");
    }
    chars
}

/// Memory Phase 1 — the crash-recovery offer. Mirrors `open_text_ask`:
/// scheme-themed, stealth-aware, parked-before-shown, ASCII-X / Esc / Dismiss
/// to close. Built ON DEMAND from a detected [`UnfinishedSession`] (only when
/// `find_unfinished_session` returned `Some`). The window shows the recovered
/// last Q&A + a couple transcript lines (the user's OWN content — fine in
/// THEIR window, which is WDA-stealthed so it never reaches a screen-share).
///
/// Recover: seed `cfg.meeting_context` with the recovered text, then start a
/// session whose `SessionStart` records `recovered_from_session_id` (the link
/// on disk). Dismiss / Esc / X: just close (the old journal stays on disk;
/// pruning is the journal's job). Per the spec we DO NOT auto-recover tiles,
/// streaming, mic, screenshot payloads, or in-flight network state.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_recover_offer(
    slot_ref: &Rc<RefCell<Option<RecoverOfferWindow>>>,
    recovered: overlay_backend::journal::UnfinishedSession,
    cfg: &overlay_backend::config::SharedConfig,
    events: &Arc<dyn RuntimeEvents>,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    // Single-instance: if it's somehow already open, just refocus it.
    {
        let slot = slot_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) {
                focus_window(hwnd);
            }
            return;
        }
    }
    let win = match RecoverOfferWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] RecoverOfferWindow::new failed: {e}");
            return;
        }
    };
    win.global::<ui::Theme>()
        .set_scheme(clamp_scheme(global_scheme()));

    // Populate the (secret-free) recovered content.
    if let Some((q, a)) = &recovered.last_qa {
        win.set_has_qa(true);
        win.set_last_question(SharedString::from(q.as_str()));
        win.set_last_answer(SharedString::from(a.as_str()));
    } else {
        win.set_has_qa(false);
    }
    win.set_transcript_preview(SharedString::from(recovered.last_lines.join("\n")));

    // Dismiss / Esc / X — close + drop, nothing else (old JSONL stays on disk).
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        win.on_dismissed(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }

    // Recover — seed context, then start a session linked to the recovered one.
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let cfg_c = cfg.clone();
        let events_c = events.clone();
        let rt_c = slint_rt.clone();
        let rth = rt_handle.clone();
        let state_c = state.clone();
        let ow = overlay_weak.clone();
        let session_id = recovered.session_id.clone();
        let recovered_for_seed = recovered.clone();
        win.on_recover_accepted(move || {
            // 1) Seed the live + persisted meeting context (secret-free).
            let chars = seed_recovery_context(&cfg_c, &recovered_for_seed);
            eprintln!(
                "[overlay-host] recovery accepted; context seeded ({chars} chars) — starting linked session"
            );

            // 2) Close the offer window.
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;

            // 3) Flip the bar's session timer ON (mirror the timer-toggle
            //    "start" branch) so the UI reflects the running session.
            {
                let mut st = match state_c.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.timer_active = true;
                st.session_secs = 0;
            }
            if let Some(o) = ow.upgrade() {
                o.set_timer_active(true);
            }

            // 4) Start the session linked to the recovered one. On failure,
            //    revert the timer UI exactly like the timer-toggle path.
            let events_s = events_c.clone();
            let cfg_s = cfg_c.clone();
            let rt_s = rt_c.clone();
            let state_revert = state_c.clone();
            let ow_revert = ow.clone();
            let link_id = session_id.clone();
            rth.spawn(async move {
                if let Err(e) = slint_session::start_session_with_recovery(
                    events_s, cfg_s, rt_s, link_id,
                ) {
                    eprintln!("[overlay-host] recovery start_session failed: {e:#}");
                    let _ = slint::invoke_from_event_loop(move || {
                        {
                            let mut st = match state_revert.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            st.timer_active = false;
                            st.session_secs = 0;
                        }
                        if let Some(o) = ow_revert.upgrade() {
                            o.set_timer_active(false);
                            o.set_status_text(SharedString::from("start failed"));
                            o.set_status_color(slint::Color::from_rgb_u8(0xe5, 0x4b, 0x4b));
                        }
                    });
                }
            });
        });
    }

    present_window_stealth_aware(&win, |hwnd| {
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

#[cfg(test)]
mod tests {
    //! Locks the strip-on-reseed guard (NIGHT_RUN_PLAN Phase 2): repeated
    //! crash→recover cycles must NOT stack recovery blocks in
    //! `meeting_context` (otherwise the live AI system prompt accretes ever-
    //! older Q&A). Pure string composition — no config, no disk, no UI.
    use super::*;

    fn recovered() -> overlay_backend::journal::UnfinishedSession {
        overlay_backend::journal::UnfinishedSession {
            session_id: "2026-06-03_10-00-00_test".to_string(),
            path: std::path::PathBuf::from("ignored.jsonl"),
            started_unix_ms: 0,
            last_lines: vec!["🎤 привет".to_string(), "🔊 как дела".to_string()],
            last_qa: Some((
                "что такое actor model".to_string(),
                "модель акторов — это…".to_string(),
            )),
            summary: Some("обсудили конкуренцию".to_string()),
        }
    }

    fn header_count(s: &str) -> usize {
        s.matches(RECOVERY_CONTEXT_HEADER).count()
    }
    fn footer_count(s: &str) -> usize {
        s.matches(RECOVERY_CONTEXT_FOOTER).count()
    }

    // (1) Seeding twice yields EXACTLY ONE recovery block, not two — the core
    // regression this guard fixes — and stays at one across many crash cycles.
    #[test]
    fn reseeding_yields_exactly_one_block() {
        let rec = recovered();
        let once = compose_recovery_context("", &rec);
        assert_eq!(header_count(&once), 1);
        assert_eq!(footer_count(&once), 1);

        let twice = compose_recovery_context(&once, &rec);
        assert_eq!(header_count(&twice), 1, "reseed must not stack a 2nd block");
        assert_eq!(footer_count(&twice), 1);

        let mut ctx = twice;
        for _ in 0..5 {
            ctx = compose_recovery_context(&ctx, &rec);
            assert_eq!(header_count(&ctx), 1);
            assert_eq!(footer_count(&ctx), 1);
        }
    }

    // (2) The user's own prose below the block survives reseeding verbatim
    // (internal blank line included) and is never duplicated.
    #[test]
    fn reseeding_preserves_user_prose_verbatim() {
        let rec = recovered();
        let prose = "Меня зовут Нини.\n\nГотовлюсь к собеседованию на Rust.";
        let once = compose_recovery_context(prose, &rec);
        assert!(once.ends_with(prose), "first seed must keep prose intact");
        assert!(once.starts_with(RECOVERY_CONTEXT_HEADER));

        let twice = compose_recovery_context(&once, &rec);
        assert_eq!(header_count(&twice), 1);
        assert!(twice.ends_with(prose), "reseed must keep prose verbatim");
        // Prose appears exactly once — not duplicated by the strip + prepend.
        assert_eq!(twice.matches("Меня зовут Нини.").count(), 1);
    }

    // (3) Seeding with NO prior block behaves exactly as before the guard:
    // empty/blank context → the block alone; non-empty prose → "block\n\nprose"
    // byte-for-byte the legacy format.
    #[test]
    fn seeding_with_no_prior_block_matches_legacy() {
        let rec = recovered();
        let block = build_recovery_block(&rec);

        assert_eq!(compose_recovery_context("", &rec), block);
        assert_eq!(compose_recovery_context("   \n  ", &rec), block); // blank-only

        let prose = "Контекст собеседования: backend, Rust, async.";
        assert_eq!(
            compose_recovery_context(prose, &rec),
            format!("{block}\n\n{prose}")
        );
    }

    // strip_recovery_block edge cases, isolated from the prepend step.
    #[test]
    fn strip_no_block_is_identity() {
        let plain = "просто мои заметки\nбез всякого блока";
        assert_eq!(strip_recovery_block(plain), plain);
        assert_eq!(strip_recovery_block(""), "");
    }

    #[test]
    fn strip_collapses_leftover_blank_lines() {
        let block = build_recovery_block(&recovered());
        // Extra blank lines between footer and prose (the "leftover trailing
        // blank line" case) are collapsed, not preserved.
        let messy = format!("{block}\n\n\n\nмои заметки");
        assert_eq!(strip_recovery_block(&messy), "мои заметки");
    }

    #[test]
    fn strip_collapses_stacked_blocks_from_pre_guard_config() {
        let rec = recovered();
        let block = build_recovery_block(&rec);
        // A config that accumulated 3 blocks before this guard existed.
        let stacked = format!("{block}\n\n{block}\n\n{block}\n\nхвост");
        assert_eq!(strip_recovery_block(&stacked), "хвост");
        // …and a fresh compose over it leaves exactly one block + the prose.
        let composed = compose_recovery_context(&stacked, &rec);
        assert_eq!(header_count(&composed), 1);
        assert!(composed.ends_with("хвост"));
    }

    #[test]
    fn strip_keeps_text_when_footer_missing() {
        // Defensive: a header with no closing footer is left untouched (we
        // never guess where a half-written block ends).
        let half = format!("{RECOVERY_CONTEXT_HEADER}\nоборванный блок без футера");
        assert_eq!(strip_recovery_block(&half), half);
    }
}
