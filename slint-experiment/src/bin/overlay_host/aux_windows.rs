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
use super::transcript_player;
use super::{
    apply_scheme_palette, apply_scheme_text_ask, apply_tile_hwnd_with_monitor, clamp_scheme,
    drag_begin, drag_update, fire_f9_ask, focus_window, global_scheme, grab_hwnd, kb, markdown,
    present_tile_window, present_window_stealth_aware, refresh_open_tiles, toggle_tile_maximize,
    ui, wire_tile_drag, Arc, ArchiveRow, ArchiveWindow, AskRoute, ComponentHandle, HelpWindow,
    MarkdownBlock, ModelRc, OverlayBarBridge, OverlayBarWindow, PaletteResult, PaletteWindow, Rc,
    RefCell, RuntimeEvents, SharedSlintRuntime, SharedString, TextAskWindow, TileWindow,
    TileWindows, TranscriptLine, TranscriptWindow, VecModel,
};
use overlay_backend::persistence::{
    open_default_store, AiTurn, SearchHit, Session, Store, Utterance,
};
// `Model` brings row_data / set_row_data / row_count for the transcript VecModel.
use slint::Model;
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
    transcript_slot: &Rc<RefCell<Option<TranscriptWindow>>>,
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
            existing.set_confirm_delete_index(-1); // F2: never reopen onto a stale confirm overlay
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
            // Re-snapshot conspect ids fresh each (re)build so a row flips to
            // "Просмотреть" the moment its summary lands (A2) — one cheap dir read.
            let conspects = overlay_backend::conspect::session_ids();
            let debriefs = overlay_backend::conspect::debrief_session_ids();
            let rows: Vec<ArchiveRow> = sessions
                .iter()
                .map(|s| session_to_row(s, &recordings, &conspects, &debriefs))
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
            // Fresh conspect snapshot per rebuild (A2): after a summary completes we
            // invoke_query_changed, and this re-read flips the row to "Просмотреть".
            let conspects = overlay_backend::conspect::session_ids();
            let debriefs = overlay_backend::conspect::debrief_session_ids();
            let rows: Vec<ArchiveRow> = if trimmed.is_empty() {
                store_rc
                    .borrow()
                    .list_sessions()
                    .unwrap_or_default()
                    .iter()
                    .map(|s| session_to_row(s, &recordings_q, &conspects, &debriefs))
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
                        .map(|h| hit_to_row(h, &recordings_q, &conspects, &debriefs))
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

    // ТЗ2a / F2 — 🗑 delete. The 🗑 button only VALIDATES (active-session guard) and
    // shows the in-app confirm overlay (a native rfd::MessageDialog crashed nested in
    // the Slint event loop — the tester hit it). The hard-delete itself runs in
    // `delete-confirmed`; `delete-cancelled` just dismisses. The backend never
    // half-deletes, so a locked-file failure keeps the row listed for an idempotent
    // retry.
    {
        let weak = win.as_weak();
        let active_del = active_id.clone();
        win.on_delete_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            if row.id.is_empty() {
                return;
            }
            if active_del.as_deref() == Some(row.id.as_str()) {
                p.set_retranscribe_status(SharedString::from("Активную сессию удалить нельзя"));
                return;
            }
            // Show the in-app confirm overlay; the actual delete is in delete-confirmed.
            p.set_confirm_delete_title(row.title);
            p.set_confirm_delete_index(idx);
        });
    }
    {
        let weak = win.as_weak();
        let store_d = store.clone();
        let active_del = active_id.clone();
        win.on_delete_confirmed(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            p.set_confirm_delete_index(-1); // dismiss the overlay
            let Some(store_rc) = store_d.as_ref() else {
                return;
            };
            // Re-fetch + re-validate by index (the list can't change behind the modal
            // scrim, but stay defensive).
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            if sid.is_empty() || active_del.as_deref() == Some(sid.as_str()) {
                return;
            }
            // CRITICAL: drop the store borrow BEFORE rebuilding. invoke_query_changed
            // dispatches the search handler SYNCHRONOUSLY (Slint callbacks run inline),
            // and that handler borrows the SAME RefCell — holding borrow_mut across it
            // double-borrows → panic. (Latent in the original ТЗ2a code; the native
            // modal crashed first, so this path was never reached.)
            let outcome = {
                let mut st = store_rc.borrow_mut();
                overlay_backend::session_admin::delete_session_everywhere(&mut st, &sid)
            };
            match outcome {
                Ok(()) => {
                    // (debrief sidecar cleanup lives in delete_session_everywhere)
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
    {
        let weak = win.as_weak();
        win.on_delete_cancelled(move || {
            if let Some(p) = weak.upgrade() {
                p.set_confirm_delete_index(-1);
            }
        });
    }

    // ТЗ1 — 📄 opens the structured read-only transcript window for a row's
    // session. The slot is process-lifetime (passed in + registry-held) so the
    // transcript survives the archive closing and is re-stealthed on a toggle.
    {
        let weak = win.as_weak();
        let store_t = store.clone();
        let tslot = transcript_slot.clone();
        win.on_transcript_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let Some(store_rc) = store_t.as_ref() else {
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
            let (session, utts) = {
                let st = store_rc.borrow();
                (
                    st.get_session(&sid).ok().flatten(),
                    st.session_utterances(&sid).unwrap_or_default(),
                )
            };
            open_transcript(&tslot, session.as_ref(), &utts);
        });
    }

    // v0.14.0 — "↻ Summary": re-transcribe a session's saved recordings OFFLINE
    // (unconstrained by real-time → a better transcript than the live one) and
    // run the meeting summary over it. ONE job at a time; the header shows
    // progress; run_meeting_summary spawns its own Summary tile, and the archive
    // stays open. A transcribe failure (no recordings / STT down) shows a generic
    // (non-leaking) error tile.
    // F3 — if a summary was already built (a conspect sidecar exists on disk), the
    // ↻ click first asks for confirmation before overwriting; with no prior summary
    // it runs straight away. The job itself is factored into `start_resummary` so
    // both the direct path and the post-confirm path share it verbatim.
    let start_resummary: std::rc::Rc<dyn Fn(i32)> = {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        let events_c = events.clone();
        let rt = rt_handle.clone();
        let store_s = store.clone();
        std::rc::Rc::new(move |idx: i32| {
            let Some(p) = weak.upgrade() else {
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
            // PROCESS-GLOBAL latch (not the per-window property): blocks a second
            // job even after this archive was closed+reopened mid-run (a fresh
            // window's `retranscribe-busy` starts false). Silently no-op while a
            // job runs, like MIC_BUSY. RAII guard moved into the task below frees
            // it on every exit incl. an awaited-future panic.
            let Some(rt_guard) = try_acquire_retranscribe() else {
                return;
            };
            p.set_retranscribe_busy(true);
            // ТЗ3 — a session with NO saved recordings can't be re-STT'd, so
            // summarize from the saved catalog transcript, else the journal's
            // ai_request prompts (summary_source). run_meeting_summary spawns its
            // own Summary tile, exactly like the re-STT path below. Additive: the
            // has-recordings path past this branch is unchanged.
            if !row.has_recordings {
                let src = store_s
                    .as_ref()
                    .and_then(|s| overlay_backend::summary_source::from_catalog(&s.borrow(), &sid))
                    .or_else(|| overlay_backend::summary_source::from_jsonl_prompts(&sid));
                let Some(transcript) = src else {
                    drop(rt_guard);
                    p.set_retranscribe_busy(false);
                    p.set_retranscribe_status(SharedString::from("Недостаточно данных для сводки"));
                    return;
                };
                p.set_retranscribe_status(SharedString::from("Building summary…"));
                let weak_done = weak.clone();
                let cfg_job = cfg_c.clone();
                let events_job = events_c.clone();
                rt.spawn(async move {
                    let _guard = rt_guard; // RAII: latch freed on task end (incl. panic)
                    overlay_backend::runtime::run_meeting_summary(
                        events_job, cfg_job, transcript, sid, true,
                    )
                    .await;
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(win) = weak_done.upgrade() {
                            win.set_retranscribe_busy(false);
                            win.set_retranscribe_status(SharedString::from(""));
                            // A2: refresh rows so this one flips to "Просмотреть" now.
                            win.invoke_query_changed(win.get_query());
                        }
                    });
                });
                return;
            }
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
                        // A2: refresh rows so this one flips to "Просмотреть" now.
                        win.invoke_query_changed(win.get_query());
                    }
                });
            });
        })
    };
    {
        let weak = win.as_weak();
        let sr = start_resummary.clone();
        win.on_retranscribe_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
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
            let title = row.title.clone();
            // F3 — overwriting an existing summary asks first; with no prior summary
            // (no conspect on disk) it runs straight away.
            if overlay_backend::conspect::exists(&sid) {
                p.set_confirm_resummary_index(idx);
                p.set_confirm_resummary_title(title);
                return;
            }
            sr(idx);
        });
    }
    {
        let weak = win.as_weak();
        let sr = start_resummary.clone();
        win.on_resummary_confirmed(move |idx| {
            if let Some(p) = weak.upgrade() {
                p.set_confirm_resummary_index(-1);
            }
            sr(idx);
        });
    }
    {
        let weak = win.as_weak();
        win.on_resummary_cancelled(move || {
            if let Some(p) = weak.upgrade() {
                p.set_confirm_resummary_index(-1);
            }
        });
    }
    // "Просмотреть" — re-show a session's SAVED summary as a tile (NO AI call),
    // reusing the normal summary-tile rendering (markdown + copy). An absent/empty
    // recap → a brief status (the row's ↻ regenerates).
    {
        let weak = win.as_weak();
        let events_c = events.clone();
        let cfg_c = cfg.clone();
        win.on_view_summary_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
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
            match overlay_backend::conspect::load(&sid).and_then(|c| c.final_summary) {
                Some(text) if !text.trim().is_empty() => {
                    let stealth = cfg_c.read().stealth_enabled;
                    let _ = events_c.spawn_tile_full(
                        overlay_backend::events::TileSpec {
                            question: format!("Сводка · {}", row.title),
                            answer: text,
                            source: "summary".into(),
                            is_translation: false,
                            highlights: vec![],
                            summary_session: Some(sid),
                        },
                        overlay_backend::events::MonitorHint::Auto,
                        stealth,
                        overlay_backend::events::TileKind::Summary,
                    );
                }
                _ => {
                    p.set_retranscribe_status(SharedString::from("Сводка пуста — нажмите ↻"));
                }
            }
        });
    }

    // D — "Коучинг": re-show the saved post-meeting debrief read-only as a tile
    // (no AI), mirroring view-summary. The button shows only when a debrief was
    // persisted (ArchiveRow.has_debrief), so load_debrief is normally Some.
    {
        let weak = win.as_weak();
        let events_c = events.clone();
        let cfg_c = cfg.clone();
        win.on_view_debrief_requested(move |idx| {
            let Some(p) = weak.upgrade() else {
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
            if let Some(text) =
                overlay_backend::conspect::load_debrief(&sid).filter(|t| !t.trim().is_empty())
            {
                let stealth = cfg_c.read().stealth_enabled;
                let _ = events_c.spawn_tile_full(
                    overlay_backend::events::TileSpec {
                        question: format!("🎯 Debrief · {}", row.title),
                        answer: text,
                        source: "debrief".into(),
                        is_translation: false,
                        highlights: vec![],
                        summary_session: None,
                    },
                    overlay_backend::events::MonitorHint::Auto,
                    stealth,
                    overlay_backend::events::TileKind::Debrief,
                );
            }
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
fn session_to_row(
    s: &Session,
    recordings: &std::collections::HashSet<String>,
    conspects: &std::collections::HashSet<String>,
    debriefs: &std::collections::HashSet<String>,
) -> ArchiveRow {
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
        // F4 / D1 — "Summary" needs a RELIABLE source: a saved recording (re-STT) or
        // indexed transcript lines (catalog). AI-Q&A-only sessions (ai_turns>0 but no
        // recording/transcript) were counted before, yet in practice they yield no
        // usable summary (the from_jsonl_prompts fallback is too thin — the tester saw
        // a "Сформировать" that then failed), so they now read "Недостаточно данных".
        has_data: recordings.contains(&s.id) || s.transcript_lines > 0,
        has_summary: conspects.contains(&s.id),
        has_debrief: debriefs.contains(&s.id),
    }
}

/// Map an FTS [`SearchHit`] to an archive list row: the session label + a
/// whitespace-collapsed, length-capped snippet of the matched body, tagged with
/// the hit kind (question · answer · utterance).
fn hit_to_row(
    h: &SearchHit,
    recordings: &std::collections::HashSet<String>,
    conspects: &std::collections::HashSet<String>,
    debriefs: &std::collections::HashSet<String>,
) -> ArchiveRow {
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
        // An FTS hit exists only because transcript / AI text matched → always
        // has a summary source.
        has_data: true,
        has_summary: conspects.contains(&h.session_id),
        has_debrief: debriefs.contains(&h.session_id),
    }
}

/// Format a session-relative offset (ms) as `mm:ss`, or `h:mm:ss` past an hour.
/// `pub(crate)` so `tile_copy::format_transcript_for_copy` reuses the SAME
/// formatter the transcript view uses (ТЗ1) — body unchanged.
pub(crate) fn fmt_offset(offset_ms: i64) -> String {
    let secs = (offset_ms / 1000).max(0);
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Copy `text` to the LOCAL clipboard (no egress → stealth-safe) and flash the
/// window's 1.5s "copied" badge. Empty `text` is a no-op (nothing selected).
fn copy_to_clipboard_and_flash(w: &TranscriptWindow, text: &str) {
    if text.is_empty() {
        return;
    }
    match clipboard_win::set_clipboard_string(text) {
        Ok(()) => {
            w.set_copied(true);
            let w2 = w.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                if let Some(w) = w2.upgrade() {
                    w.set_copied(false);
                }
            });
        }
        Err(e) => eprintln!("[overlay-host] transcript copy failed: {e}"),
    }
}

/// Wire the transcript window's copy + selection actions (ТЗ1, decision #7):
/// "Copy all" / "Copy selected" (the latter honours the per-line checkboxes), the
/// per-line toggle, and "select all". The selection lives IN the row model
/// (`checked` per row), mutated via `set_row_data` so it survives scrolling, and
/// is read back at copy time. Re-wired on EVERY open (fresh model + utterances)
/// so a reused window always acts on the session currently shown. Copy formats
/// via the pure `tile_copy::format_transcript_for_copy` and is purely local.
fn wire_transcript_actions(
    win: &TranscriptWindow,
    model: &Rc<VecModel<TranscriptLine>>,
    utts: &[Utterance],
    session_start: Option<i64>,
) {
    // Reset transient UI state — the window is reused across sessions, so a fresh
    // open must not inherit the prior session's "select all" tick or "copied"
    // flash (the fresh model already starts all-unchecked).
    win.set_all_selected(false);
    win.set_copied(false);
    let utts_owned: Vec<Utterance> = utts.to_vec();

    // Toggle one line; keep "select all" in sync (ON iff EVERY row is checked).
    {
        let m = model.clone();
        let weak = win.as_weak();
        win.on_toggle_line(move |idx| {
            let i = idx.max(0) as usize;
            let Some(mut row) = m.row_data(i) else {
                return;
            };
            row.checked = !row.checked;
            m.set_row_data(i, row);
            if let Some(w) = weak.upgrade() {
                let all = m.row_count() > 0
                    && (0..m.row_count()).all(|j| m.row_data(j).is_some_and(|r| r.checked));
                w.set_all_selected(all);
            }
        });
    }

    // Select / deselect every line.
    {
        let m = model.clone();
        let weak = win.as_weak();
        win.on_toggle_all(move |on| {
            for j in 0..m.row_count() {
                if let Some(mut row) = m.row_data(j) {
                    row.checked = on;
                    m.set_row_data(j, row);
                }
            }
            if let Some(w) = weak.upgrade() {
                w.set_all_selected(on);
            }
        });
    }

    // Copy ALL lines (selected = None).
    {
        let utts_c = utts_owned.clone();
        let weak = win.as_weak();
        win.on_copy_all_requested(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if utts_c.is_empty() {
                return;
            }
            let with_tc = w.get_with_timecodes();
            let text =
                super::tile_copy::format_transcript_for_copy(&utts_c, session_start, None, with_tc);
            copy_to_clipboard_and_flash(&w, &text);
        });
    }

    // Copy only the CHECKED lines (no-op when nothing is selected). The model is
    // built 1:1 with `utts`, so a checked row index is exactly the utterance index.
    {
        let m = model.clone();
        let utts_c = utts_owned;
        let weak = win.as_weak();
        win.on_copy_selected_requested(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let mut sel = std::collections::HashSet::new();
            for j in 0..m.row_count() {
                if m.row_data(j).is_some_and(|r| r.checked) {
                    sel.insert(j);
                }
            }
            if sel.is_empty() {
                return;
            }
            let with_tc = w.get_with_timecodes();
            let text = super::tile_copy::format_transcript_for_copy(
                &utts_c,
                session_start,
                Some(&sel),
                with_tc,
            );
            copy_to_clipboard_and_flash(&w, &text);
        });
    }
}

/// Index of the line whose timecode contains `pos_ms` (the last line whose start
/// is ≤ pos); -1 before the first line. Lines are chronological, so stop at the
/// first start past `pos`.
fn active_line_for_ms(model: &Rc<VecModel<TranscriptLine>>, pos_ms: i64) -> i32 {
    let mut active = -1_i32;
    for j in 0..model.row_count() {
        let Some(row) = model.row_data(j) else {
            break;
        };
        if i64::from(row.start_ms) <= pos_ms {
            active = j as i32;
        } else {
            break;
        }
    }
    active
}

/// Wire the ТЗ2b mini-player: play/pause, click-line → seek+play, seek-bar, and a
/// 200 ms poll pushing position / active-line / play-state into the window.
/// Re-wired on EVERY open with the current session id + model (the window is
/// reused; `open_transcript` already reset any prior session's player). The audio
/// engine lives in the `transcript_player` UI-thread thread-local.
fn wire_transcript_player(
    win: &TranscriptWindow,
    session_id: &str,
    model: &Rc<VecModel<TranscriptLine>>,
) {
    let has_audio = overlay_backend::session_audio::session_has_recordings(session_id);
    win.set_has_audio(has_audio);
    win.set_playing(false);
    win.set_progress(0.0);
    win.set_time_text(SharedString::default());
    win.set_active_line(-1);
    if !has_audio {
        return;
    }

    let id = session_id.to_string();
    {
        let weak = win.as_weak();
        let id = id.clone();
        win.on_toggle_play(move || {
            if transcript_player::ensure(&id) {
                transcript_player::toggle();
                if let Some(w) = weak.upgrade() {
                    w.set_playing(transcript_player::is_playing());
                }
            }
        });
    }
    {
        let weak = win.as_weak();
        let id = id.clone();
        let m = model.clone();
        win.on_play_line(move |idx| {
            let Some(row) = m.row_data(idx.max(0) as usize) else {
                return;
            };
            if transcript_player::ensure(&id) {
                transcript_player::seek_and_play(i64::from(row.start_ms));
                if let Some(w) = weak.upgrade() {
                    w.set_playing(transcript_player::is_playing());
                }
            }
        });
    }
    win.on_seek_fraction(transcript_player::seek_fraction);

    // 200 ms position poll → seek-bar / time / active-line / play state.
    let weak = win.as_weak();
    let m = model.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if let Some((progress, pos_ms, total_ms, playing)) = transcript_player::snapshot() {
                w.set_progress(progress);
                w.set_playing(playing);
                w.set_time_text(SharedString::from(format!(
                    "{} / {}",
                    fmt_offset(pos_ms),
                    fmt_offset(total_ms)
                )));
                w.set_active_line(active_line_for_ms(&m, pos_ms));
            }
        },
    );
    transcript_player::set_poll_timer(timer);
}

/// Open a READ-ONLY structured transcript window for a session (ТЗ1). Reuses the
/// slot if already open; otherwise builds the per-line model from the session's
/// utterances and presents the window stealth-aware. The model is (re)built on
/// EVERY open so a reused window never shows a prior session.
pub(crate) fn open_transcript(
    slot: &Rc<RefCell<Option<TranscriptWindow>>>,
    session: Option<&Session>,
    utts: &[Utterance],
) {
    // Build the model once — shared by the reuse + first-open paths.
    let session_start = session.and_then(|s| s.started_at_ms).filter(|&ms| ms > 0);
    let heading = session
        .map(|s| session_title(s.started_at_ms, &s.id))
        .unwrap_or_default();
    let lines: Vec<TranscriptLine> = utts
        .iter()
        .enumerate()
        .map(|(i, u)| {
            // F1: prefer the persisted per-line audio offset (audio_ms) for BOTH the
            // shown timecode AND the player seek; old sessions (no audio_ms) fall back
            // to the prev-line wall-clock approximation. See line_start_offset_ms.
            let off = overlay_backend::session_audio::line_start_offset_ms(utts, i, session_start);
            TranscriptLine {
                offset_label: off.map(fmt_offset).unwrap_or_default().into(),
                speaker: SharedString::from(if u.source == "mic" {
                    "Микрофон"
                } else {
                    "Система"
                }),
                text: SharedString::from(u.text.split_whitespace().collect::<Vec<_>>().join(" ")),
                checked: false,
                start_ms: off.unwrap_or(0) as i32,
            }
        })
        .collect();
    let session_id = session.map(|s| s.id.clone()).unwrap_or_default();
    // Drop any prior session's player + poll timer — the window is reused, so a
    // fresh open must not keep the previous session's audio playing (ТЗ2b).
    transcript_player::reset();

    // Reuse if already open — repopulate (a reused window must show THIS session)
    // and re-focus via the borrowed strong handle. Slint handles are NOT `Clone`,
    // so the single strong handle stays in the slot and closures use weak handles.
    if let Some(win) = slot.borrow().as_ref() {
        win.global::<ui::Theme>()
            .set_scheme(clamp_scheme(global_scheme()));
        win.set_heading(SharedString::from(heading));
        win.set_empty(utts.is_empty());
        let model = Rc::new(VecModel::from(lines));
        win.set_lines(ModelRc::from(model.clone()));
        wire_transcript_actions(win, &model, utts, session_start);
        wire_transcript_player(win, &session_id, &model);
        let _ = win.show();
        if let Ok(hwnd) = grab_hwnd(win.window()) {
            focus_window(hwnd);
        }
        return;
    }

    // First open: create, populate, wire (weak closures), present, then store.
    let win = match TranscriptWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] TranscriptWindow::new failed: {e}");
            return;
        }
    };
    win.global::<ui::Theme>()
        .set_scheme(clamp_scheme(global_scheme()));
    win.set_heading(SharedString::from(heading));
    win.set_empty(utts.is_empty());
    let model = Rc::new(VecModel::from(lines));
    win.set_lines(ModelRc::from(model.clone()));
    wire_transcript_actions(&win, &model, utts, session_start);
    wire_transcript_player(&win, &session_id, &model);

    {
        let slot_c = slot.clone();
        let weak = win.as_weak();
        win.on_close_requested(move || {
            transcript_player::reset(); // stop audio + poll timer when the window closes
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot_c.borrow_mut() = None;
        });
    }
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
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot.borrow_mut() = Some(win);
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
        for (i, u) in utterances.iter().enumerate() {
            let label = if u.source == "mic" {
                "Микрофон"
            } else {
                "Система"
            };
            // Collapse internal whitespace/newlines so one utterance = one line.
            let text = u.text.split_whitespace().collect::<Vec<_>>().join(" ");
            // F1: start = previous line's timestamp (first = origin); see session_audio.
            match overlay_backend::session_audio::line_start_offset_ms(utterances, i, session_start)
            {
                Some(off) => {
                    let off = fmt_offset(off);
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
        let row = session_to_row(
            &sample_session(),
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
        );
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
        let row = session_to_row(&s, &no_recordings(), &no_recordings(), &no_recordings());
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
        let row = hit_to_row(&h, &no_recordings(), &no_recordings(), &no_recordings());
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
            audio_ms: None,
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
                unix_ms: start + 29_000, // finalized 00:29 in (≈ its end)
                source: "system".into(),
                text: "привет".into(),
                audio_ms: None,
            },
            Utterance {
                session_id: "s".into(),
                unix_ms: start + 135_000,
                source: "mic".into(),
                text: "да   слышу".into(), // internal whitespace collapses to one space
                audio_ms: None,
            },
        ];
        let md = build_session_markdown(Some(&s), &utts, &[]);
        // F1: a line's START = the PREVIOUS line's timestamp; the FIRST line is 00:00
        // (NOT its own finalize time 00:29), so line 2 starts where line 1 ended (00:29).
        assert!(md.contains("[00:00] Система: привет"), "got: {md}");
        assert!(md.contains("[00:29] Микрофон: да слышу"), "got: {md}");
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
