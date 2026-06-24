//! Auxiliary on-demand overlay windows split out of the `overlay_host.rs`
//! composition root (P2 of `docs/overlay-host-gaps-and-next-checks.md`): the
//! "✏ Написать" text-ask window (`open_text_ask`), the 🆘 Help window
//! (`open_help`), and the F4 KB palette (`open_palette`) plus the palette's pure
//! helpers (`results_index`, `kb_to_palette_results`, `PaletteResultExt`). These
//! were the last large blocks of host-side window wiring still inlined in the
//! binary's file; `overlay_host.rs` now reaches them through the
//! `use aux_windows::*;` re-export, so the call sites (F1 / F4 / ✏ dispatch +
//! the 🆘 chip in `main`) resolve unchanged.
//!
//! SECURITY (unchanged by this mechanical move): every window is parked
//! off-screen + WDA-stealthed via `present_window_stealth_aware` before its
//! first on-screen frame, and skipped from the taskbar / Alt-Tab, so it never
//! leaks onto a screen-share while open. The palette renders only KB text — no
//! bearer / base_url / transcript ever reaches its scope.
//!
//! NOTE (§7): the parent crate-root symbols are imported explicitly below;
//! `diag!` is reached by textual macro scope (defined before the `mod` decl).
use super::{
    apply_scheme_palette, apply_scheme_text_ask, apply_tile_hwnd_with_monitor, clamp_scheme,
    drag_begin, drag_update, fire_f9_ask, focus_window, global_scheme, grab_hwnd, kb, markdown,
    present_tile_window, present_window_stealth_aware, refresh_open_tiles, toggle_tile_maximize,
    ui, wire_tile_drag, Arc, ArchiveRow, ArchiveWindow, AskRoute, ComponentHandle, HelpWindow,
    MarkdownBlock, ModelRc, OverlayBarBridge, OverlayBarWindow, PaletteResult, PaletteWindow, Rc,
    RefCell, RuntimeEvents, SharedSlintRuntime, SharedString, TextAskWindow, TileWindow,
    TileWindows, VecModel,
};
use overlay_backend::persistence::{
    open_default_store, AiTurn, SearchHit, Session, Store, Utterance,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// v0.14.0 — PROCESS-GLOBAL one-job-at-a-time guard for archive re-transcription.
///
/// The per-window `retranscribe-busy` Slint property drives this window's UI
/// (button hidden + progress shown), but it dies with the window: closing the
/// archive mid-job then re-opening it builds a FRESH `ArchiveWindow` whose
/// property starts `false`, which would let a second `retranscribe_and_summarize`
/// spawn while the first still runs (N× the ~230 MB/channel WAV load + a
/// duplicate Summary tile). This static outlives any single window, so a
/// close+reopen still sees the running job. One `try_acquire` pairs with exactly
/// one `release` in the worker's completion path. (Same pattern as `MIC_BUSY`.)
static RETRANSCRIBE_BUSY: AtomicBool = AtomicBool::new(false);

/// RAII release for the retranscribe latch. Dropping it — on ANY exit of the
/// spawned task, including a panic unwinding the awaited re-STT/Summary future —
/// frees the latch, so a panic in the heaviest job in the app can't leave the
/// "↻ Summary" button dead for the rest of the process (audit Q1). The `()`
/// field is private so only `try_acquire_retranscribe` can mint one.
struct RetranscribeGuard(());

impl Drop for RetranscribeGuard {
    fn drop(&mut self) {
        RETRANSCRIBE_BUSY.store(false, Ordering::Release);
    }
}

fn try_acquire_retranscribe() -> Option<RetranscribeGuard> {
    RETRANSCRIBE_BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
        .then(|| RetranscribeGuard(()))
}

/// v0.10.1 — format the active profile/persona for the text-ask header so the
/// user sees which profile will shape the typed answer. The profile applies to
/// a typed question the SAME as to a voice one (both go through `fire_f9_ask` →
/// `cfg.read().meeting_context` → `ai::build_request`); this label just makes it
/// visible. Read LIVE so a profile switch in Settings is reflected even on a
/// reused window.
fn text_ask_profile_label(cfg: &overlay_backend::config::SharedConfig) -> String {
    let c = cfg.read();
    match c.active_profile.as_deref() {
        Some(n) if !n.trim().is_empty() => format!("Профиль: {n}"),
        _ if !c.meeting_context.trim().is_empty() => "Профиль: свой контекст".to_string(),
        _ => "Профиль: не задан".to_string(),
    }
}

/// V0.8.3 — "Написать": open (or re-focus) the small text-input window. On
/// submit it routes the typed text through `fire_f9_ask(.., Some(text))`, so the
/// whole tile-create + stream + cost + journal + follow-up pipeline is reused →
/// the answer lands in a standard tile. Stealth (WDA) + on-screen placement come
/// from `present_window_stealth_aware`; the decorate closure also grabs keyboard
/// focus so the user can type immediately. Esc (or submit) hides + drops it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_text_ask(
    slot_ref: &Rc<RefCell<Option<TextAskWindow>>>,
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    {
        let slot = slot_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            // Refresh the profile label in case it changed since this window was
            // first opened (reused windows keep their original handlers).
            existing.set_active_profile(SharedString::from(text_ask_profile_label(cfg)));
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) {
                focus_window(hwnd);
            }
            return;
        }
    }
    let win = match TextAskWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] TextAskWindow::new failed: {e}");
            return;
        }
    };
    apply_scheme_text_ask(&win, global_scheme());
    win.set_active_profile(SharedString::from(text_ask_profile_label(cfg)));
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let bridge_c = bridge.clone();
        let events_c = events.clone();
        let cfg_c = cfg.clone();
        let rt_c = slint_rt.clone();
        let rth = rt_handle.clone();
        let tiles_c = tiles.clone();
        let wov = weak_overlay.clone();
        win.on_submitted(move |q| {
            let q = q.trim().to_string();
            if !q.is_empty() {
                fire_f9_ask(
                    &bridge_c,
                    &events_c,
                    &cfg_c,
                    &rt_c,
                    &rth,
                    &tiles_c,
                    &wov,
                    AskRoute::Text,
                    Some(q),
                );
            }
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    present_window_stealth_aware(&win, |hwnd| {
        // Keep these transient overlay windows out of the taskbar + Alt-Tab,
        // like the bar/tiles — otherwise under stealth they leak an existence
        // entry while open (content is WDA-hidden, but the window button isn't).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window) — same as archive.
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

/// V0.8.4 — 🆘 Help (F1 / 🆘 chip): a read-only reference window (bar icons,
/// hotkeys, record gestures). Created on demand like open_text_ask —
/// scheme-themed, stealth-aware, Esc / "X" to close. Re-opening re-focuses it.
pub(crate) fn open_help(
    slot_ref: &Rc<RefCell<Option<HelpWindow>>>,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
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
    let win = match HelpWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] HelpWindow::new failed: {e}");
            return;
        }
    };
    win.global::<ui::Theme>()
        .set_scheme(clamp_scheme(global_scheme()));
    // Light up the bar's 🆘 chip while help is open (same as ⚙ for Settings).
    if let Some(o) = overlay_weak.upgrade() {
        o.set_help_open(true);
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let ow = overlay_weak.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
            if let Some(o) = ow.upgrade() {
                o.set_help_open(false);
            }
        });
    }
    // Frameless drag (cursor-delta, same as Settings) — the header is the handle.
    {
        let weak = win.as_weak();
        win.on_drag_start_requested(move || {
            if let Some(w) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_begin(hwnd);
                }
            }
        });
        let weak_move = win.as_weak();
        win.on_drag_moved(move || {
            if let Some(w) = weak_move.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_update(hwnd);
                }
            }
        });
    }
    present_window_stealth_aware(&win, |hwnd| {
        // Keep these transient overlay windows out of the taskbar + Alt-Tab,
        // like the bar/tiles — otherwise under stealth they leak an existence
        // entry while open (content is WDA-hidden, but the window button isn't).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window can't get them from
        // an inner border-radius) — same as the archive window.
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

/// Open (or reuse) the KB palette window. Auto-spawn a tile when
/// the user activates a result, mimicking the React palette flow.
pub(crate) fn open_palette(
    palette_ref: &Rc<RefCell<Option<PaletteWindow>>>,
    tiles_ref: &TileWindows,
    state: &slint_replay::app_state::SharedState,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    let mut slot = palette_ref.borrow_mut();
    if let Some(existing) = slot.as_ref() {
        let _ = existing.show();
        return;
    }
    let win = match PaletteWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] PaletteWindow::new failed: {e}");
            return;
        }
    };
    // Seed the palette's Theme global from the live scheme (the palette is
    // ephemeral — spawned per F4 — so it just reads at construction).
    apply_scheme_palette(&win, global_scheme());

    // Phase C — wire palette to real overlay_backend::kb::search.
    // Initial load: show top 20 entries (popular/first in cache).
    let initial = kb_to_palette_results(&kb::search("", 20));
    win.set_results(slint::ModelRc::new(slint::VecModel::from(initial)));

    let weak_self_q = win.as_weak();
    win.on_query_changed(move |q| {
        let Some(p) = weak_self_q.upgrade() else {
            return;
        };
        let hits = kb::search(q.as_str(), 20);
        let model = kb_to_palette_results(&hits);
        p.set_results(slint::ModelRc::new(slint::VecModel::from(model)));
    });

    let weak_close = win.as_weak();
    let palette_close = palette_ref.clone();
    win.on_close_requested(move || {
        if let Some(w) = weak_close.upgrade() {
            let _ = w.hide();
        }
        *palette_close.borrow_mut() = None;
    });

    let s_ref = state.clone();
    let tiles_ref2 = tiles_ref.clone();
    let weak_overlay2 = weak_overlay.clone();
    let palette_after = palette_ref.clone();
    let weak_self = win.as_weak();
    win.on_result_activated(move |idx| {
        let Some(p) = weak_self.upgrade() else { return };
        let results = p.get_results();
        let Some(result) = results_index(&results, idx) else {
            return;
        };

        // Spawn a read-only tile with the result content via the shared helper
        // (also used by the session archive). Phase C — wire to real kb::get for
        // the full body; fall back to the preview if the key isn't found
        // (defensive — the result came from kb::search).
        let body = kb::get(result.key.as_str())
            .map_or_else(|| result.preview.to_string(), |e| e.body.clone());
        let md = format!("# {}\n\n{body}\n", result.heading_or_key());
        spawn_content_tile(
            result.title.as_str(),
            &format!("kb · {}", result.source),
            &md,
            &tiles_ref2,
            &s_ref,
            &weak_overlay2,
        );
        // Close palette after activation.
        if let Some(p) = weak_self.upgrade() {
            let _ = p.hide();
        }
        *palette_after.borrow_mut() = None;
    });

    // #111 + review M1 — exclude the palette from capture WITHOUT a flash:
    // park off-screen before show, apply WDA, then reveal centred. No extra
    // HWND decoration for the palette (it's an opaque window).
    present_window_stealth_aware(&win, |hwnd| {
        // Keep the palette out of the taskbar/Alt-Tab too (stealth existence
        // leak — same as help/text-ask/wizard above).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window) — same as archive.
        slint_replay::win32::set_round_corners(hwnd);
    });
    *slot = Some(win);
}

fn results_index(model: &slint::ModelRc<PaletteResult>, idx: i32) -> Option<PaletteResult> {
    use slint::Model;
    if idx < 0 {
        return None;
    }
    model.row_data(idx as usize)
}

/// Convert overlay_backend::kb::KBEntry rows into the Slint PaletteResult
/// struct that the .slint UI consumes.
fn kb_to_palette_results(entries: &[kb::KBEntry]) -> Vec<PaletteResult> {
    entries
        .iter()
        .map(|e| {
            // First sentence (or first 160 chars) of body for preview.
            let preview = e
                .body
                .split_terminator(['.', '\n'])
                .next()
                .unwrap_or("")
                .chars()
                .take(160)
                .collect::<String>();
            PaletteResult {
                key: SharedString::from(e.key.clone()),
                title: SharedString::from(e.heading.clone()),
                preview: SharedString::from(preview),
                source: SharedString::from(e.source),
            }
        })
        .collect()
}

/// PaletteResult ergonomic extension — `heading_or_key` returns the
/// .heading if non-empty, else falls back to the .key. Stops the
/// tile title from being blank when an entry has just a key.
trait PaletteResultExt {
    fn heading_or_key(&self) -> String;
}

impl PaletteResultExt for PaletteResult {
    fn heading_or_key(&self) -> String {
        if self.title.is_empty() {
            self.key.to_string()
        } else {
            self.title.to_string()
        }
    }
}

// ============================================================================
// Session archive (Phase 3a) — browse + FTS-search the SQLite catalog.
// ============================================================================

/// Phase 3a — open (or re-focus) the 🗄 session-archive browser (F7 / 🗄 chip).
/// Lists every indexed session newest-first and full-text-searches their
/// transcript + AI Q&A over the SQLite catalog; activating a row spawns a
/// read-only tile with that session's content (via [`spawn_content_tile`], the
/// same path the KB palette uses). Stealth-aware + skip-taskbar like the other
/// aux windows. The window holds ONE [`Store`] (opened here) for its lifetime,
/// reused across the list / search / detail queries; if the catalog can't be
/// opened it shows a graceful "unavailable" state instead of a blank panel.
///
/// SECURITY: renders ONLY the user's own transcript + AI answers — no bearer /
/// base_url / config secret ever reaches its scope (like the palette).
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_archive(
    archive_ref: &Rc<RefCell<Option<ArchiveWindow>>>,
    tiles_ref: &TileWindows,
    state: &slint_replay::app_state::SharedState,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
    cfg: &overlay_backend::config::SharedConfig,
    events: &Arc<dyn RuntimeEvents>,
    rt_handle: &tokio::runtime::Handle,
    slint_rt: &SharedSlintRuntime,
) {
    {
        let slot = archive_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) {
                focus_window(hwnd);
            }
            return;
        }
    }
    let win = match ArchiveWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] ArchiveWindow::new failed: {e}");
            return;
        }
    };
    win.global::<ui::Theme>()
        .set_scheme(clamp_scheme(global_scheme()));
    // Egress signpost: warn (in the header) that "↻ Summary" re-uploads saved
    // audio when STT is the cloud (Groq). Local backends stay one-click, no note.
    win.set_stt_is_cloud(!cfg.read().stt_is_local());

    // v0.17.2 (тестер P0.1) — reindex BEFORE listing. The catalog used to be
    // populated only by the launch-time sweep, so sessions finished in the
    // current run (and everything, if that sweep failed) were invisible —
    // the tester's "архив показывает 0 и 0 / старые сессии пропали".
    // Idempotent + cheap when there is nothing new (one read_dir + one id-set
    // query); the LIVE session's still-growing journal is skipped so its row
    // can't be frozen mid-write as "crashed".
    // Gated on the same toggle as the launch sweep + stop-index, so disabling
    // the archive in Settings really stops ALL catalog writes (review #2).
    // Live session id — skipped by reindex AND guarded from delete (ТЗ2a). One
    // compute, reused by both.
    let active_id = slint_replay::runtime_state::lock(slint_rt)
        .journal
        .as_ref()
        .and_then(overlay_backend::journal::Journal::current_path)
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()));
    if cfg.read().session_archive_enabled {
        match overlay_backend::persistence::reindex_default(active_id.as_deref()) {
            Ok(st) => eprintln!(
                "[overlay-host] archive: reindex on open — {} new, {} skipped, {} failed",
                st.indexed, st.skipped, st.failed
            ),
            Err(e) => eprintln!("[overlay-host] archive: reindex on open failed: {e:#}"),
        }
    }

    // Open ONE catalog handle for this browse session (reused by the closures
    // below). On failure the window degrades to an "unavailable" state.
    let store: Option<Rc<RefCell<Store>>> = match open_default_store() {
        Ok(s) => Some(Rc::new(RefCell::new(s))),
        Err(e) => {
            eprintln!("[overlay-host] archive: catalog open failed: {e}");
            None
        }
    };

    // One recordings snapshot for this browse session (v0.17.1 — was a
    // filesystem stat PER ROW per rebuild; see recording_ids_snapshot).
    let recordings = Rc::new(recording_ids_snapshot());

    match store.as_ref() {
        Some(store_rc) => {
            let sessions = store_rc.borrow().list_sessions().unwrap_or_default();
            // v0.17.1 — plain count (the 🗄 now lives in the header SVG icon).
            win.set_summary(SharedString::from(sessions.len().to_string()));
            let rows: Vec<ArchiveRow> = sessions
                .iter()
                .map(|s| session_to_row(s, &recordings))
                .collect();
            win.set_results(ModelRc::new(VecModel::from(rows)));
        }
        None => {
            win.set_unavailable(true);
        }
    }

    // Search-as-you-type: empty query → full list; else an FTS5 prefix search
    // over utterances + AI questions/answers.
    {
        let weak = win.as_weak();
        let store_q = store.clone();
        let recordings_q = recordings.clone();
        win.on_query_changed(move |q| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let Some(store_rc) = store_q.as_ref() else {
                return;
            };
            let trimmed = q.trim();
            let rows: Vec<ArchiveRow> = if trimmed.is_empty() {
                store_rc
                    .borrow()
                    .list_sessions()
                    .unwrap_or_default()
                    .iter()
                    .map(|s| session_to_row(s, &recordings_q))
                    .collect()
            } else {
                let fts = fts_query(trimmed);
                if fts.is_empty() {
                    Vec::new()
                } else {
                    store_rc
                        .borrow()
                        .search(&fts, 60)
                        .unwrap_or_default()
                        .iter()
                        .map(|h| hit_to_row(h, &recordings_q))
                        .collect()
                }
            };
            // v0.22.0 — a list rebuild invalidates the index-keyed rename state,
            // so cancel any in-progress edit (else ✓ would persist to whatever
            // session now occupies that row index — a silent mis-rename).
            p.set_renaming_index(-1);
            p.set_results(ModelRc::new(VecModel::from(rows)));
        });
    }

    // Activate a row → spawn a read-only tile with that session's full content.
    // The archive stays OPEN so several sessions can be opened in a row.
    {
        let weak = win.as_weak();
        let store_a = store.clone();
        let tiles_c = tiles_ref.clone();
        let state_c = state.clone();
        let wov = weak_overlay.clone();
        win.on_result_activated(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let Some(store_rc) = store_a.as_ref() else {
                return;
            };
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            let (session, utts, turns) = {
                let st = store_rc.borrow();
                (
                    st.get_session(&sid).ok().flatten(),
                    st.session_utterances(&sid).unwrap_or_default(),
                    st.session_ai_turns(&sid).unwrap_or_default(),
                )
            };
            let title = session_title(session.as_ref().and_then(|s| s.started_at_ms), &sid);
            let md = build_session_markdown(session.as_ref(), &utts, &turns);
            spawn_content_tile(&title, "archive", &md, &tiles_c, &state_c, &wov);
        });
    }

    // v0.22.0 — inline rename: ✎ pre-fills the field from the row's current
    // name; ✓ / Enter persists to the session_names sidecar + refreshes the
    // list; ✗ cancels. Clearing the field reverts the row to the time label.
    {
        let weak = win.as_weak();
        win.on_rename_requested(move |idx, name| {
            if let Some(p) = weak.upgrade() {
                p.set_renaming_index(idx);
                p.set_rename_text(name);
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_rename_cancelled(move || {
            if let Some(p) = weak.upgrade() {
                p.set_renaming_index(-1);
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_rename_confirmed(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let new_name = p.get_rename_text().trim().to_string();
            let results = p.get_results();
            if let Some(row) = archive_row_at(&results, idx) {
                let sid = row.id.to_string();
                if !sid.is_empty() {
                    overlay_backend::session_names::set(
                        &sid,
                        &new_name,
                        overlay_backend::journal::now_unix_ms(),
                    );
                }
            }
            p.set_renaming_index(-1);
            // Re-run the query handler so the row title reflects the new name.
            let q = p.get_query();
            p.invoke_query_changed(q);
        });
    }

    // v0.22.0 — ↻ regen: re-ask the LOCAL model for a fresh title from the
    // session's saved transcript, persist + refresh. Local-only + best-effort
    // (a cloud-only config simply does nothing — no egress, no cost).
    {
        let weak = win.as_weak();
        let store_g = store.clone();
        let cfg_g = cfg.clone();
        let rth_g = rt_handle.clone();
        win.on_regen_name_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let Some(store_rc) = store_g.as_ref() else {
                return;
            };
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            if sid.is_empty() {
                return;
            }
            let lines: Vec<String> = store_rc
                .borrow()
                .session_utterances(&sid)
                .unwrap_or_default()
                .iter()
                .map(|u| u.text.clone())
                .collect();
            if lines.is_empty() {
                return;
            }
            let ep = cfg_g.read().ai_endpoint(true);
            if !ep.is_local {
                return; // local-only naming — never spend cloud money on a title
            }
            let weak2 = weak.clone();
            rth_g.spawn(async move {
                let Some(name) = slint_replay::session_namer::generate_name(&ep, &lines).await
                else {
                    return;
                };
                overlay_backend::session_names::set(
                    &sid,
                    &name,
                    overlay_backend::journal::now_unix_ms(),
                );
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(p) = weak2.upgrade() {
                        let q = p.get_query();
                        p.invoke_query_changed(q);
                    }
                });
            });
        });
    }

    // ТЗ2a — 🗑 delete: confirm via a native modal, then hard-delete every
    // artifact (journal / audio / conspect / catalog row, cascading) and refresh
    // the list. The live session is guarded; a locked-file failure keeps the row
    // listed for an idempotent retry (the backend never half-deletes).
    {
        let weak = win.as_weak();
        let store_d = store.clone();
        let active_del = active_id.clone();
        win.on_delete_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let Some(store_rc) = store_d.as_ref() else {
                return;
            };
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            if sid.is_empty() {
                return;
            }
            if active_del.as_deref() == Some(sid.as_str()) {
                p.set_retranscribe_status(SharedString::from("Активную сессию удалить нельзя"));
                return;
            }
            let title = row.title.to_string();
            let confirmed = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Удалить сессию?")
                .set_description(format!(
                    "«{title}» будет удалена безвозвратно (стенограмма, аудио, сводка)."
                ))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if confirmed != rfd::MessageDialogResult::Yes {
                return;
            }
            match overlay_backend::session_admin::delete_session_everywhere(
                &mut store_rc.borrow_mut(),
                &sid,
            ) {
                Ok(()) => {
                    // Rebuild the list (the row is gone); also resets edit-state.
                    let q = p.get_query();
                    p.invoke_query_changed(q);
                }
                Err(e) => {
                    eprintln!("[overlay-host] archive: delete {sid} failed: {e:#}");
                    p.set_retranscribe_status(SharedString::from(
                        "Удаление не удалось (файл занят?) — повторите",
                    ));
                }
            }
        });
    }

    // v0.14.0 — "↻ Summary": re-transcribe a session's saved recordings OFFLINE
    // (unconstrained by real-time → a better transcript than the live one) and
    // run the meeting summary over it. ONE job at a time; the header shows
    // progress; run_meeting_summary spawns its own Summary tile, and the archive
    // stays open. A transcribe failure (no recordings / STT down) shows a generic
    // (non-leaking) error tile.
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        let events_c = events.clone();
        let rt = rt_handle.clone();
        win.on_retranscribe_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            if !row.has_recordings {
                return;
            }
            // PROCESS-GLOBAL latch (not the per-window property): blocks a second
            // job even after this archive was closed+reopened mid-run (a fresh
            // window's `retranscribe-busy` starts false). Silently no-op while a
            // job runs, like MIC_BUSY. RAII guard moved into the task below frees
            // it on every exit incl. an awaited-future panic.
            let Some(rt_guard) = try_acquire_retranscribe() else {
                return;
            };
            let sid = row.id.to_string();
            p.set_retranscribe_busy(true);
            p.set_retranscribe_status(SharedString::from("starting…"));
            let weak_job = weak.clone();
            let cfg_job = cfg_c.clone();
            let events_job = events_c.clone();
            let events_err = events_c.clone();
            let stealth = cfg_c.read().stealth_enabled;
            rt.spawn(async move {
                let weak_prog = weak_job.clone();
                // Progress is Send-safe: it only carries a String + the Send
                // slint::Weak, re-upgraded on the UI thread.
                let on_progress = move |prog: overlay_backend::re_transcribe::Progress| {
                    let overlay_backend::re_transcribe::Progress::Step(msg) = prog;
                    let w = weak_prog.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(win) = w.upgrade() {
                            win.set_retranscribe_status(SharedString::from(msg));
                        }
                    });
                };
                let result = overlay_backend::re_transcribe::retranscribe_and_summarize(
                    events_job,
                    cfg_job,
                    &sid,
                    &on_progress,
                )
                .await;
                if let Err(e) = &result {
                    // Log the chain locally; show a GENERIC tile (no leak).
                    eprintln!("[overlay-host] re-transcribe failed: {e:#}");
                    let _ = events_err.spawn_tile_full(
                        overlay_backend::events::TileSpec {
                            question: "Ре-Summary из архива".to_string(),
                            answer: "Не удалось перетранскрибировать запись этой сессии. \
                                     Проверьте, что запись на месте и STT настроен \
                                     (Настройки → STT), и попробуйте ещё раз."
                                .to_string(),
                            source: "summary".into(),
                            is_translation: false,
                            highlights: vec![],
                            // Re-STT failed before any conspect — nothing to resume.
                            summary_session: None,
                        },
                        overlay_backend::events::MonitorHint::Auto,
                        stealth,
                        overlay_backend::events::TileKind::Error,
                    );
                }
                // Release the PROCESS-GLOBAL latch first (survives even if this
                // window was closed mid-job and the weak upgrade below fails).
                // RAII: also released if the awaited future above panicked.
                drop(rt_guard);
                let weak_done = weak_job.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(win) = weak_done.upgrade() {
                        win.set_retranscribe_busy(false);
                        win.set_retranscribe_status(SharedString::from(""));
                    }
                });
            });
        });
    }

    {
        let weak = win.as_weak();
        let slot = archive_ref.clone();
        let wov = weak_overlay.clone();
        win.on_close_requested(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
            if let Some(o) = wov.upgrade() {
                o.set_archive_open(false);
            }
        });
    }

    // v0.17.1 — drag the frameless window by its header (mirror of the tile
    // drag: pointer-down anchors, moved-while-pressed moves the HWND).
    {
        let weak = win.as_weak();
        win.on_drag_start_requested(move || {
            if let Some(w) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_begin(hwnd);
                }
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_drag_moved(move || {
            if let Some(w) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_update(hwnd);
                }
            }
        });
    }

    present_window_stealth_aware(&win, |hwnd| {
        // Keep the archive out of the taskbar / Alt-Tab too (stealth existence
        // leak — same as palette / help / text-ask).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // v0.17.1 — OS-level rounded corners (opaque frameless window can't get
        // them from an inner border-radius; see win32::set_round_corners).
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    // Light the 🗄 bar chip while the archive is open (like 🆘 / ⚙). Cleared
    // by the F7 toggle + the in-window close handler.
    if let Some(o) = weak_overlay.upgrade() {
        o.set_archive_open(true);
    }
    *archive_ref.borrow_mut() = Some(win);
}

/// Spawn a standard read-only content tile (shared by the KB palette + the
/// session archive): a `TileWindow` with markdown `body_md`, wired for
/// close / pin / maximize / drag, placed on the right monitor and registered in
/// `tiles`. Bumps the session tile counter exactly as the palette did.
pub(crate) fn spawn_content_tile(
    title: &str,
    source_label: &str,
    body_md: &str,
    tiles: &TileWindows,
    state: &slint_replay::app_state::SharedState,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    let seq = {
        let mut st = match state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        st.tiles_spawned += 1;
        st.tiles_spawned
    };
    if let Some(o) = weak_overlay.upgrade() {
        o.set_tiles_spawned(seq as i32);
    }
    let Ok(tile) = TileWindow::new() else {
        return;
    };
    tile.set_sequence(seq as i32);
    tile.set_tile_title(SharedString::from(title.to_string()));
    tile.set_source_label(SharedString::from(source_label.to_string()));
    wire_tile_drag(&tile);
    let blocks: Vec<MarkdownBlock> = markdown::parse(body_md)
        .into_iter()
        .map(|b| MarkdownBlock {
            kind: b.kind,
            text: SharedString::from(b.text),
            lang: SharedString::from(b.lang),
        })
        .collect();
    tile.set_blocks(ModelRc::new(VecModel::from(blocks)));

    let weak_tile = tile.as_weak();
    let vec_for_close = tiles.clone();
    let weak_overlay_close = weak_overlay.clone();
    tile.on_close_clicked(move || {
        if let Some(t) = weak_tile.upgrade() {
            let close_hwnd = grab_hwnd(t.window()).ok();
            let _ = t.hide();
            slint_replay::win32::force_hide(t.window());
            if let Some(target) = close_hwnd {
                vec_for_close
                    .borrow_mut()
                    .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
                refresh_open_tiles(&weak_overlay_close, &vec_for_close);
            }
        }
    });
    let weak_pin = tile.as_weak();
    tile.on_pin_clicked(move || {
        if let Some(t) = weak_pin.upgrade() {
            let new = !t.get_pinned();
            t.set_pinned(new);
        }
    });
    let weak_max = tile.as_weak();
    tile.on_maximize_clicked(move || {
        if let Some(t) = weak_max.upgrade() {
            let Ok(hwnd) = grab_hwnd(t.window()) else {
                return;
            };
            toggle_tile_maximize(hwnd, &t);
        }
    });

    present_tile_window(&tile);
    apply_tile_hwnd_with_monitor(&tile);
    tiles.borrow_mut().push(tile);
    refresh_open_tiles(weak_overlay, tiles);
}

/// Bounds-checked row lookup into the archive results model (mirror of the
/// palette's `results_index`).
fn archive_row_at(model: &slint::ModelRc<ArchiveRow>, idx: i32) -> Option<ArchiveRow> {
    use slint::Model;
    if idx < 0 {
        return None;
    }
    model.row_data(idx as usize)
}

/// Turn a free-text archive query into a safe FTS5 MATCH expression: split on
/// every non-alphanumeric char (whitespace, hyphen, punctuation — matching the
/// `unicode61` tokenizer), then append `*` to each token so it becomes a PREFIX
/// match (incremental "search as you type"). An all-punctuation query collapses
/// to `""` — the caller then shows no rows rather than passing FTS5 a string it
/// would reject. Keeps the SQL/FTS surface entirely inside `persistence`.
fn fts_query(raw: &str) -> String {
    raw.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("{t}*"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Human label from a session id (the JSONL file stem, e.g.
/// `2026-06-04_10-00-00_ab12`) → `2026-06-04 10:00:00`. The stem already
/// encodes the start time, so no date/time crate is needed. Falls back to the
/// raw id when it doesn't match the `date_time_suffix` shape.
fn pretty_session_label(id: &str) -> String {
    let parts: Vec<&str> = id.splitn(3, '_').collect();
    if parts.len() >= 2 && parts[0].len() == 10 && parts[1].len() == 8 {
        format!("{} {}", parts[0], parts[1].replace('-', ":"))
    } else {
        id.to_string()
    }
}

/// v0.17.2 (тестер P0.2) — Moscow wall-clock label for archive rows. The
/// session id is a UTC stamp ([`journal::chrono_like_stamp`]), and the old
/// label re-formatted it verbatim — so an МСК user saw every call 3 hours
/// early. Prefer the indexed `started_at_ms` (the true session_start time);
/// fall back to parsing the id stamp (old rows, FTS hits — which carry only
/// the id). Both paths convert at DISPLAY time, so ALREADY-RECORDED sessions
/// show МСК retroactively; ids/dirs stay UTC (opaque join keys).
fn archive_time_label(started_at_ms: Option<i64>, id: &str) -> String {
    if let Some(ms) = started_at_ms.filter(|ms| *ms > 0) {
        return overlay_backend::journal::format_msk_label(ms);
    }
    match overlay_backend::journal::stamp_to_unix_secs(id) {
        Some(secs) => overlay_backend::journal::format_msk_label((secs as i64) * 1000),
        // Not a stamp-shaped id — show it as before rather than guessing.
        None => pretty_session_label(id),
    }
}

/// Status → a compact prefix for the row title. The COMPLETED case (the normal
/// 99%) gets NO prefix so a named/timed title reads clean; only the abnormal
/// states are flagged (so "done Обзор функций" → just "Обзор функций").
fn status_glyph(status: &str) -> &'static str {
    match status {
        "crashed" => "crashed",
        "active" => "active",
        _ => "", // completed / unknown — clean title, no prefix
    }
}

/// v0.17.1 (мега-аудит) — snapshot the recordings dir ONCE per archive open.
/// The per-row `is_dir()` probe ran for EVERY row on EVERY list rebuild —
/// with 160+ sessions that was 160+ filesystem stats per keystroke in the
/// search box, all on the UI thread. One `read_dir` at open replaces them;
/// a recording created while the archive stays open shows its button after
/// a reopen (acceptable — recordings appear at session start, not mid-browse).
fn recording_ids_snapshot() -> std::collections::HashSet<String> {
    overlay_backend::recorder::recordings_dir()
        .ok()
        .and_then(|root| std::fs::read_dir(root).ok())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The archive display title for a session: the persisted session NAME (v0.22.0
/// `session_names` sidecar) if any, else the МСК time label.
fn session_title(started_at_ms: Option<i64>, id: &str) -> String {
    overlay_backend::session_names::get(id).unwrap_or_else(|| archive_time_label(started_at_ms, id))
}

/// Map an indexed [`Session`] to an archive list row. Counts are emoji-coded
/// as plain text counts so the row needs no per-language string;
/// the cost shows only when non-zero (local runs are $0 → blank).
fn session_to_row(s: &Session, recordings: &std::collections::HashSet<String>) -> ArchiveRow {
    let time = archive_time_label(s.started_at_ms, &s.id);
    let name = overlay_backend::session_names::get(&s.id);
    // Prefer the session NAME (v0.22.0) as the row title; fall back to the time.
    let label = name.clone().unwrap_or_else(|| time.clone());
    let model = s.ai_model.as_deref().unwrap_or("—");
    // When a name is the title, keep the time visible in the subtitle.
    let subtitle = if name.is_some() {
        format!(
            "{time} · lines {} · ai {} · {model}",
            s.transcript_lines, s.ai_turns_count
        )
    } else {
        format!(
            "lines {} · ai {} · {model}",
            s.transcript_lines, s.ai_turns_count
        )
    };
    let meta = if s.total_cost_microcents > 0 {
        format!("${:.3}", (s.total_cost_microcents as f64) / 100_000_000.0)
    } else {
        String::new()
    };
    let glyph = status_glyph(&s.status);
    let title = if glyph.is_empty() {
        label
    } else {
        format!("{glyph} {label}")
    };
    ArchiveRow {
        id: SharedString::from(s.id.clone()),
        title: SharedString::from(title),
        subtitle: SharedString::from(subtitle),
        meta: SharedString::from(meta),
        has_recordings: recordings.contains(&s.id),
        name: SharedString::from(name.unwrap_or_default()),
    }
}

/// Map an FTS [`SearchHit`] to an archive list row: the session label + a
/// whitespace-collapsed, length-capped snippet of the matched body, tagged with
/// the hit kind (question · answer · utterance).
fn hit_to_row(h: &SearchHit, recordings: &std::collections::HashSet<String>) -> ArchiveRow {
    // Prefer the session NAME (v0.22.0) so a named session reads the same in
    // search results as in the full list; fall back to the МСК time. Keep the
    // raw name too, to pre-fill the inline rename field.
    let name = overlay_backend::session_names::get(&h.session_id);
    let label = name
        .clone()
        .unwrap_or_else(|| archive_time_label(None, &h.session_id));
    let kind_glyph = match h.kind.as_str() {
        "question" => "question",
        "answer" => "answer",
        _ => "line",
    };
    let snippet: String = h
        .body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect();
    ArchiveRow {
        id: SharedString::from(h.session_id.clone()),
        title: SharedString::from(format!("search {label}")),
        subtitle: SharedString::from(snippet),
        meta: SharedString::from(kind_glyph),
        has_recordings: recordings.contains(&h.session_id),
        name: SharedString::from(name.unwrap_or_default()),
    }
}

/// Format a session-relative offset (ms) as `mm:ss`, or `h:mm:ss` past an hour.
fn fmt_offset(offset_ms: i64) -> String {
    let secs = (offset_ms / 1000).max(0);
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Render a session's content as the markdown body of a read-only tile:
/// a heading (status + label + counts), the transcript, then the AI Q&A.
/// Pure → unit-tested.
fn build_session_markdown(
    session: Option<&Session>,
    utterances: &[Utterance],
    ai_turns: &[AiTurn],
) -> String {
    let mut out = String::new();
    if let Some(s) = session {
        out.push_str(&format!("# {}\n\n", session_title(s.started_at_ms, &s.id)));
        let model = s.ai_model.as_deref().unwrap_or("—");
        out.push_str(&format!(
            "lines {} · ai {} · {model}",
            s.transcript_lines, s.ai_turns_count
        ));
        if s.total_cost_microcents > 0 {
            out.push_str(&format!(
                " · ${:.3}",
                (s.total_cost_microcents as f64) / 100_000_000.0
            ));
        }
        out.push_str("\n\n");
    }
    // Transcript region — chronological, with a session-relative timecode
    // (derived from the session start) and the two-way channel label.
    let session_start = session.and_then(|s| s.started_at_ms).filter(|&ms| ms > 0);
    if utterances.is_empty() {
        out.push_str("_Транскрипт не сохранён_\n\n");
    } else {
        for u in utterances {
            let label = if u.source == "mic" {
                "Микрофон"
            } else {
                "Система"
            };
            // Collapse internal whitespace/newlines so one utterance = one line.
            let text = u.text.split_whitespace().collect::<Vec<_>>().join(" ");
            match session_start {
                Some(start) => {
                    let off = fmt_offset((u.unix_ms - start).max(0));
                    out.push_str(&format!("[{off}] {label}: {text}\n\n"));
                }
                None => out.push_str(&format!("{label}: {text}\n\n")),
            }
        }
    }
    if !ai_turns.is_empty() {
        out.push_str("---\n\n");
        for t in ai_turns {
            if !t.question.trim().is_empty() {
                out.push_str(&format!("Question: **{}**\n\n", t.question.trim()));
            }
            if !t.answer.trim().is_empty() {
                out.push_str(&format!("Answer: {}\n\n", t.answer.trim()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn pretty_label_parses_stem() {
        assert_eq!(
            pretty_session_label("2026-06-04_10-00-00_ab12"),
            "2026-06-04 10:00:00"
        );
    }

    #[test]
    fn pretty_label_falls_back_on_odd_id() {
        assert_eq!(pretty_session_label("weird"), "weird");
        assert_eq!(pretty_session_label(""), "");
    }

    #[test]
    fn archive_time_label_prefers_started_at_then_id_then_raw() {
        // Real start time wins (UTC ms → МСК).
        assert_eq!(
            archive_time_label(Some(1_779_580_800_000), "2026-06-04_09-30-00_zz"),
            "24.05.2026 03:00:00 (МСК)"
        );
        // No indexed time (old rows / FTS hits) → parse the UTC id stamp.
        assert_eq!(
            archive_time_label(None, "2026-06-04_09-30-00_zz"),
            "04.06.2026 12:30:00 (МСК)"
        );
        // Zero/garbage started_at_ms falls through to the id.
        assert_eq!(
            archive_time_label(Some(0), "2026-06-04_09-30-00_zz"),
            "04.06.2026 12:30:00 (МСК)"
        );
        // Non-stamp id → raw, as before.
        assert_eq!(archive_time_label(None, "weird"), "weird");
    }

    #[test]
    fn fts_query_prefixes_tokens_and_drops_punctuation() {
        assert_eq!(fts_query("hash map"), "hash* map*");
        // unicode61 splits on the hyphen, so the two halves become two prefixes.
        assert_eq!(fts_query("хеш-таблицу"), "хеш* таблицу*");
        assert_eq!(fts_query("   "), "");
        assert_eq!(fts_query("!?.,"), "");
    }

    fn sample_session() -> Session {
        Session {
            id: "2026-06-04_09-30-00_zz".into(),
            journal_path: "C:/sessions/x.jsonl".into(),
            // 2026-05-24 00:00:00 UTC → 03:00:00 МСК in the row label.
            started_at_ms: Some(1_779_580_800_000),
            finished_at_ms: Some(1_779_580_800_002),
            status: "completed".into(),
            ai_model: Some("gemma".into()),
            transcript_lines: 12,
            ai_turns_count: 3,
            total_cost_microcents: 0,
            indexed_at_ms: 0,
        }
    }

    fn no_recordings() -> std::collections::HashSet<String> {
        std::collections::HashSet::new()
    }

    #[test]
    fn session_row_uses_plain_counts() {
        let row = session_to_row(&sample_session(), &no_recordings());
        // v0.17.2 — the label is МСК wall-clock from started_at_ms, not the UTC id.
        // v0.22.0 — a COMPLETED session has no status prefix (the "done " was
        // dropped so a named/timed title reads clean).
        assert!(
            row.title.as_str().starts_with("24.05.2026 03:00:00 (МСК)"),
            "got {:?}",
            row.title
        );
        assert!(row.subtitle.as_str().contains("lines 12"));
        assert!(row.subtitle.as_str().contains("ai 3"));
        assert_eq!(row.meta.as_str(), ""); // zero cost → blank meta
    }

    #[test]
    fn session_row_shows_cost_when_nonzero() {
        let mut s = sample_session();
        s.total_cost_microcents = 2_400_000; // $0.024
        let row = session_to_row(&s, &no_recordings());
        assert_eq!(row.meta.as_str(), "$0.024");
    }

    #[test]
    fn hit_row_tags_kind_and_caps_snippet() {
        let h = SearchHit {
            session_id: "2026-06-04_09-30-00_zz".into(),
            kind: "answer".into(),
            unix_ms: 5,
            body: "a   key value   structure".into(),
            rank: -1.0,
        };
        let row = hit_to_row(&h, &no_recordings());
        // Hits carry only the UTC id stamp → parsed + shifted to МСК (+3h).
        assert!(
            row.title
                .as_str()
                .starts_with("search 04.06.2026 12:30:00 (МСК)"),
            "got {:?}",
            row.title
        );
        assert_eq!(row.meta.as_str(), "answer");
        assert_eq!(row.subtitle.as_str(), "a key value structure"); // whitespace collapsed
    }

    #[test]
    fn session_markdown_has_transcript_and_qa() {
        let utts = vec![Utterance {
            session_id: "s".into(),
            unix_ms: 1,
            source: "mic".into(),
            text: "hello there".into(),
        }];
        let turns = vec![AiTurn {
            session_id: "s".into(),
            unix_ms: 2,
            purpose: "ask".into(),
            model: "m".into(),
            question: "what is it?".into(),
            answer: "an answer.".into(),
            latency_ms: None,
            attached_screenshot: false,
        }];
        let md = build_session_markdown(None, &utts, &turns);
        assert!(md.contains("Микрофон: hello there")); // session None → no timecode
        assert!(md.contains("Question: **what is it?**"));
        assert!(md.contains("Answer: an answer."));
    }

    #[test]
    fn session_markdown_transcript_has_timecodes_and_ru_labels() {
        let s = sample_session(); // started_at_ms = Some(1_779_580_800_000)
        let start = 1_779_580_800_000_i64;
        let utts = vec![
            Utterance {
                session_id: "s".into(),
                unix_ms: start,
                source: "system".into(),
                text: "привет".into(),
            },
            Utterance {
                session_id: "s".into(),
                unix_ms: start + 135_000, // 02:15
                source: "mic".into(),
                text: "да   слышу".into(), // internal whitespace collapses to one space
            },
        ];
        let md = build_session_markdown(Some(&s), &utts, &[]);
        assert!(md.contains("[00:00] Система: привет"), "got: {md}");
        assert!(md.contains("[02:15] Микрофон: да слышу"), "got: {md}");
    }

    #[test]
    fn session_markdown_empty_shows_not_saved_notice() {
        let md = build_session_markdown(None, &[], &[]);
        assert!(md.contains("Транскрипт не сохранён"), "got: {md}");
    }

    #[test]
    fn fmt_offset_mm_ss_and_h_mm_ss() {
        assert_eq!(fmt_offset(0), "00:00");
        assert_eq!(fmt_offset(135_000), "02:15");
        assert_eq!(fmt_offset(3_661_000), "1:01:01");
        assert_eq!(fmt_offset(-5), "00:00"); // negative clamps
    }
}
