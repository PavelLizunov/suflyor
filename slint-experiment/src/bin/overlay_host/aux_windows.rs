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
    present_tile_window, present_window_stealth_aware, present_window_stealth_aware_at,
    refresh_open_tiles, toggle_tile_maximize, ui, wire_tile_drag, Arc, ArchiveRow, ArchiveWindow,
    AskRoute, ComponentHandle, HelpWindow, MarkdownBlock, ModelRc, OverlayBarBridge,
    OverlayBarWindow, PaletteResult, PaletteWindow, Rc, RefCell, RuntimeEvents, SharedSlintRuntime,
    SharedString, SpeakerRow, TextAskWindow, TileWindow, TileWindows, TranscriptLine,
    TranscriptWindow, VecModel, TILE_DISPLAY_SEQ,
};
use overlay_backend::persistence::{
    open_default_store, AiTurn, Diarization, SearchHit, Session, Store, Utterance,
};
// `Model` brings row_data / set_row_data / row_count for the transcript VecModel.
use slint::Model;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The archive's catalog handle, opened lazily OFF the event loop. `None` until
/// the open worker lands — or permanently `None` if the catalog can't be opened,
/// which puts the window in its "unavailable" state. Shared by every archive
/// closure AND the open worker (which publishes the `Send` `Store` back via
/// `invoke_from_event_loop`), so it is an `Arc<Mutex<..>>`. Callers hold the lock
/// only for a single read/write and drop it before re-entrant UI callbacks.
type StoreSlot = Arc<Mutex<Option<Store>>>;

/// Lock the catalog slot, recovering a poisoned lock (the codebase mutex idiom).
/// Contention can't happen in practice — every access runs on the event loop.
fn lock_store(slot: &StoreSlot) -> std::sync::MutexGuard<'_, Option<Store>> {
    match slot.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

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
static DIAR_BUSY: AtomicBool = AtomicBool::new(false);
/// V-1 — same latch for the diarization model download: the install outlives
/// the transcript window, but the per-window `installing-diar-models` property
/// dies with it — a close+reopen would otherwise spawn a second `install_models`
/// racing the first on the same .download/staging/live files. One
/// `try_acquire_busy` pairs with the worker's completion (RAII guard).
static DIAR_INSTALL_BUSY: AtomicBool = AtomicBool::new(false);

/// Shared RAII guard for process-global background-job latches.
struct BusyGuard<'a>(&'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn try_acquire_busy(busy: &AtomicBool) -> Option<BusyGuard<'_>> {
    busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
        .then(|| BusyGuard(busy))
}

/// v0.10.1 — format the active profile/persona for the text-ask header so the
/// user sees which profile will shape the typed answer. The profile applies to
/// a typed question the SAME as to a voice one (both go through `fire_f9_ask` →
/// `cfg.read().meeting_context` → `ai::build_request`); this label just makes it
/// visible. Read LIVE so a profile switch in Settings is reflected even on a
/// reused window.
fn text_ask_profile_label(cfg: &overlay_backend::config::SharedConfig) -> String {
    let c = cfg.read();
    text_ask_profile_label_for(
        c.active_profile.as_deref(),
        !c.meeting_context.trim().is_empty(),
        c.ui_language == "ru",
    )
}

fn text_ask_profile_label_for(
    active_profile: Option<&str>,
    has_custom_context: bool,
    is_ru: bool,
) -> String {
    match (
        active_profile.filter(|name| !name.trim().is_empty()),
        has_custom_context,
        is_ru,
    ) {
        (Some(name), _, true) => format!("Профиль: {name}"),
        (Some(name), _, false) => format!("Profile: {name}"),
        (None, true, true) => "Профиль: свой контекст".to_string(),
        (None, true, false) => "Profile: custom context".to_string(),
        (None, false, true) => "Профиль: не задан".to_string(),
        (None, false, false) => "Profile: not set".to_string(),
    }
}

/// ТЗ 2026-07-06 (C) — persist the text-ask window's current top-left so the
/// next open restores it (`present_window_stealth_aware_at`). Called on submit
/// and cancel (the window is dropped right after, so this is the last chance to
/// read the rect). No-op when the position is unchanged — avoids a config
/// rewrite on every plain open/close.
fn persist_text_ask_pos(win: &TextAskWindow, cfg: &overlay_backend::config::SharedConfig) {
    let Ok(hwnd) = grab_hwnd(win.window()) else {
        return;
    };
    let Ok((x, y, _, _)) = slint_replay::win32::get_window_rect(hwnd) else {
        return;
    };
    let mut c = cfg.write();
    if c.text_ask_pos != Some((x, y)) {
        c.text_ask_pos = Some((x, y));
        let _ = overlay_backend::config::save(&c);
    }
}

/// V0.8.3 — "Написать": open (or re-focus) the small text-input window. On
/// submit it routes the typed text through `fire_f9_ask(.., Some(text))`, so the
/// whole tile-create + stream + cost + journal + follow-up pipeline is reused →
/// the answer lands in a standard tile. Stealth (WDA) + on-screen placement come
/// from `present_window_stealth_aware_at` (restoring the last dragged position,
/// ТЗ 2026-07-06 C); the decorate closure also grabs keyboard focus so the user
/// can type immediately. Esc (or submit) persists the position + hides + drops.
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
                persist_text_ask_pos(&w, &cfg_c);
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let cfg_c = cfg.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                persist_text_ask_pos(&w, &cfg_c);
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    // ТЗ 2026-07-06 (C) — frameless drag (cursor-delta, same as help/wizard);
    // the header row is the handle.
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
    // Restore the last dragged position (validated against visible monitors
    // inside; stale/None → centered as before).
    let saved_pos = cfg.read().text_ask_pos;
    present_window_stealth_aware_at(&win, saved_pos, |hwnd| {
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

    // Frameless drag — the new header owns the grab target; the close button
    // remains a sibling so pointer events never conflict.
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
/// aux windows. The window shows immediately and opens its ONE [`Store`] OFF the
/// event loop (a worker thread also runs the reindex sweep + initial list), then
/// reuses that handle across the list / search / detail queries; if the catalog
/// can't be opened it shows a graceful "unavailable" state instead of a blank panel.
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
    // The catalog open, the initial list, AND the reindex SWEEP (read_dir + parse
    // + index of every new journal) all block, so they run on a worker thread; the
    // window above is already on screen and shows an empty list until the rows land.
    // The handle is published to a shared slot the closures below read lazily, so a
    // click that arrives before the open finishes is a graceful no-op, and a failed
    // open leaves the slot empty → the "unavailable" state.
    let store: StoreSlot = Arc::new(Mutex::new(None));

    // One recordings snapshot for this browse session (v0.17.1 — was a
    // filesystem stat PER ROW per rebuild; see recording_ids_snapshot).
    let recordings = Rc::new(recording_ids_snapshot());

    // Row wording language, snapshotted for this browse session (mirrors the
    // other per-open snapshots; a language switch applies on the next open).
    let ru = cfg.read().ui_language == "ru";

    // Баг5-class guard: the archive results render in an UN-VIRTUALIZED 50px-row
    // `for` inside a ScrollView (archive.slint:299), so cap the DISPLAYED rows —
    // content must stay under the Slint SW-renderer's i16 (32767px) coordinate
    // limit or a rounded-rect row's wrapped coordinate panics/corrupts (like the
    // Memory tab). The DB keeps every session (the count shows the true total);
    // older sessions are reachable via search (itself capped at 60).
    const ARCHIVE_LIST_CAP: usize = 300;
    const _: () = assert!(ARCHIVE_LIST_CAP * 60 + 200 < 32_767);

    {
        let weak_load = win.as_weak();
        let slot_load = store.clone();
        let active_id_load = active_id.clone();
        let archive_enabled = cfg.read().session_archive_enabled;
        std::thread::spawn(move || {
            // Reindex SWEEP first (gated on the archive toggle), so the list below
            // catches anything finished since the launch-time sweep.
            if archive_enabled {
                match overlay_backend::persistence::reindex_default(active_id_load.as_deref()) {
                    Ok(st) => eprintln!(
                        "[overlay-host] archive: reindex on open — {} new, {} skipped, {} failed",
                        st.indexed, st.skipped, st.failed
                    ),
                    Err(e) => eprintln!("[overlay-host] archive: reindex on open failed: {e:#}"),
                }
            }
            // Open the read handle OFF the event loop (it runs migrations + WAL
            // setup) and build the initial rows from it.
            let opened: Option<Store> = match open_default_store() {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("[overlay-host] archive: catalog open failed: {e}");
                    None
                }
            };
            let initial: Option<(Vec<ArchiveRow>, usize)> = opened.as_ref().map(|st| {
                let sessions = st.list_sessions().unwrap_or_default();
                let total = sessions.len();
                let recordings = recording_ids_snapshot();
                let conspects = overlay_backend::conspect::session_ids();
                let debriefs = overlay_backend::conspect::debrief_session_ids();
                let rows = sessions
                    .iter()
                    .take(ARCHIVE_LIST_CAP)
                    .map(|s| session_to_row(s, &recordings, &conspects, &debriefs, ru))
                    .collect();
                (rows, total)
            });
            let _ = slint::invoke_from_event_loop(move || {
                let Some(p) = weak_load.upgrade() else {
                    return;
                };
                match opened {
                    Some(s) => *lock_store(&slot_load) = Some(s),
                    None => p.set_unavailable(true),
                }
                if let Some((rows, total)) = initial {
                    p.set_summary(SharedString::from(total.to_string()));
                    p.set_results(ModelRc::new(VecModel::from(rows)));
                }
            });
        });
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
            let trimmed = q.trim();
            // Fresh conspect snapshot per rebuild (A2): after a summary completes we
            // invoke_query_changed, and this re-read flips the row to "Просмотреть".
            let conspects = overlay_backend::conspect::session_ids();
            let debriefs = overlay_backend::conspect::debrief_session_ids();
            let rows: Vec<ArchiveRow> = {
                let slot = lock_store(&store_q);
                let Some(st) = slot.as_ref() else {
                    return;
                };
                if trimmed.is_empty() {
                    st.list_sessions()
                        .unwrap_or_default()
                        .iter()
                        .take(ARCHIVE_LIST_CAP)
                        .map(|s| session_to_row(s, &recordings_q, &conspects, &debriefs, ru))
                        .collect()
                } else {
                    let fts = fts_query(trimmed);
                    if fts.is_empty() {
                        Vec::new()
                    } else {
                        st.search(&fts, 60)
                            .unwrap_or_default()
                            .iter()
                            .map(|h| hit_to_row(h, &recordings_q, &conspects, &debriefs, ru))
                            .collect()
                    }
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
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            let (session, utts, turns) = {
                let slot = lock_store(&store_a);
                let Some(st) = slot.as_ref() else {
                    return;
                };
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
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            if sid.is_empty() {
                return;
            }
            let lines: Vec<String> = {
                let slot = lock_store(&store_g);
                let Some(st) = slot.as_ref() else {
                    return;
                };
                st.session_utterances(&sid)
                    .unwrap_or_default()
                    .iter()
                    .map(|u| u.text.clone())
                    .collect()
            };
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
            // Dismiss, then re-fetch + re-validate by index (the list can't change
            // behind the modal scrim, but stay defensive).
            p.set_confirm_delete_index(-1);
            let results = p.get_results();
            let Some(row) = archive_row_at(&results, idx) else {
                return;
            };
            let sid = row.id.to_string();
            if sid.is_empty() || active_del.as_deref() == Some(sid.as_str()) {
                return;
            }
            // CRITICAL: drop the store lock BEFORE rebuilding. invoke_query_changed
            // dispatches the search handler SYNCHRONOUSLY (Slint callbacks run inline)
            // and that handler locks the SAME slot — holding the guard across it
            // deadlocks the Mutex.
            let outcome = {
                let mut slot = lock_store(&store_d);
                match slot.as_mut() {
                    Some(st) => overlay_backend::session_admin::delete_session_everywhere(st, &sid),
                    None => return,
                }
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
        let rt_t = rt_handle.clone();
        win.on_transcript_requested(move |idx| {
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
            let (session, utts) = {
                let slot = lock_store(&store_t);
                let Some(st) = slot.as_ref() else {
                    return;
                };
                (
                    st.get_session(&sid).ok().flatten(),
                    st.session_utterances(&sid).unwrap_or_default(),
                )
            };
            open_transcript(&tslot, session.as_ref(), &utts, &store_t, &rt_t);
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
            let Some(rt_guard) = try_acquire_busy(&RETRANSCRIBE_BUSY) else {
                return;
            };
            p.set_retranscribe_busy(true);
            // ТЗ3 — a session with NO saved recordings can't be re-STT'd, so
            // summarize from the saved catalog transcript, else the journal's
            // ai_request prompts (summary_source). run_meeting_summary spawns its
            // own Summary tile, exactly like the re-STT path below. Additive: the
            // has-recordings path past this branch is unchanged.
            if !row.has_recordings {
                let src = {
                    let slot = lock_store(&store_s);
                    slot.as_ref()
                        .and_then(|st| overlay_backend::summary_source::from_catalog(st, &sid))
                        .or_else(|| overlay_backend::summary_source::from_jsonl_prompts(&sid))
                };
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
                let (stealth, ui_is_ru) = {
                    let c = cfg_c.read();
                    (c.stealth_enabled, c.ui_is_ru())
                };
                let _ = events_c.spawn_tile_full(
                    overlay_backend::events::TileSpec {
                        question: format!(
                            "{} · {}",
                            if ui_is_ru { "Разбор" } else { "Debrief" },
                            row.title
                        ),
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
    let display_seq = TILE_DISPLAY_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let Ok(tile) = TileWindow::new() else {
        return;
    };
    tile.set_sequence(display_seq as i32);
    tile.set_tile_title(SharedString::from(title.to_string()));
    tile.set_source_label(SharedString::from(source_label.to_string()));
    wire_tile_drag(&tile);
    let blocks: Vec<MarkdownBlock> = markdown::parse(body_md)
        .into_iter()
        .map(|b| MarkdownBlock {
            kind: b.kind,
            text: SharedString::from(b.text),
            display_text: SharedString::from(b.display_text),
            lang: SharedString::from(b.lang),
            marked: false,
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

/// Status → a short LOCALIZED human label for the row subtitle. The COMPLETED
/// case (the normal 99%) gets NO label so a named/timed row reads clean; only
/// the abnormal states are flagged. The raw DB tokens (`crashed` / `active`)
/// never reach a visible row — the user sees a word in the UI language.
fn status_label(status: &str, ru: bool) -> &'static str {
    match status {
        "crashed" => {
            if ru {
                "Прервана"
            } else {
                "Interrupted"
            }
        }
        "active" => {
            if ru {
                "Идёт сейчас"
            } else {
                "In progress"
            }
        }
        _ => "", // completed / unknown — clean row, no status flag
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

/// Map an indexed [`Session`] to an archive list row. Counts + status use
/// short LOCALIZED labels (`ru` mirrors `cfg.ui_language`, like the other
/// Rust-built strings) so the row reads as human wording — no code-like
/// `lines N · ai N` metadata, no placeholder dash for an unknown model, and
/// no raw `crashed` / `active` token. The cost shows only when non-zero
/// (local runs are $0 → blank).
fn session_to_row(
    s: &Session,
    recordings: &std::collections::HashSet<String>,
    conspects: &std::collections::HashSet<String>,
    debriefs: &std::collections::HashSet<String>,
    ru: bool,
) -> ArchiveRow {
    let time = archive_time_label(s.started_at_ms, &s.id);
    let name = overlay_backend::session_names::get(&s.id);
    // Prefer the session NAME (v0.22.0) as the row title; fall back to the
    // time. The title carries NO status prefix — an abnormal state is flagged
    // in the subtitle instead (localized, next to the counts).
    let title = name.clone().unwrap_or_else(|| time.clone());
    let transcript_word = if ru {
        "Стенограмма"
    } else {
        "Transcript"
    };
    let ai_word = if ru { "ИИ" } else { "AI" };
    // Subtitle = time (when a NAME is the title) · transcript count · AI count
    // · model (only when known) · status (only when abnormal).
    let mut parts: Vec<String> = Vec::new();
    if name.is_some() {
        parts.push(time);
    }
    parts.push(format!("{}: {}", transcript_word, s.transcript_lines));
    parts.push(format!("{}: {}", ai_word, s.ai_turns_count));
    if let Some(model) = s.ai_model.as_deref().filter(|m| !m.is_empty()) {
        parts.push(model.to_string());
    }
    let status = status_label(&s.status, ru);
    if !status.is_empty() {
        parts.push(status.to_string());
    }
    let subtitle = parts.join(" · ");
    let meta = if s.total_cost_microcents > 0 {
        // SessionRow stores i64; the >0 guard makes the checked conversion exact.
        let micro = u64::try_from(s.total_cost_microcents).unwrap_or(0);
        format!("${:.3}", overlay_backend::ai::microcents_to_usd(micro))
    } else {
        String::new()
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
/// a LOCALIZED hit-kind word (the raw journal tags `question` / `answer` /
/// `utterance` never reach the row verbatim).
fn hit_to_row(
    h: &SearchHit,
    recordings: &std::collections::HashSet<String>,
    conspects: &std::collections::HashSet<String>,
    debriefs: &std::collections::HashSet<String>,
    ru: bool,
) -> ArchiveRow {
    // Prefer the session NAME (v0.22.0) so a named session reads the same in
    // search results as in the full list; fall back to the МСК time. Keep the
    // raw name too, to pre-fill the inline rename field.
    let name = overlay_backend::session_names::get(&h.session_id);
    let label = name
        .clone()
        .unwrap_or_else(|| archive_time_label(None, &h.session_id));
    let kind_word = match h.kind.as_str() {
        "question" => {
            if ru {
                "вопрос"
            } else {
                "question"
            }
        }
        "answer" => {
            if ru {
                "ответ"
            } else {
                "answer"
            }
        }
        _ => {
            if ru {
                "строка"
            } else {
                "line"
            }
        }
    };
    let search_word = if ru { "Поиск" } else { "Search" };
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
        title: SharedString::from(format!("{search_word}: {label}")),
        subtitle: SharedString::from(snippet),
        meta: SharedString::from(kind_word),
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
    match slint_replay::native::clipboard::set_text(text) {
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
///
/// Un-mark every transcript line (shared by save-marked + clear-marks + the
/// selection-capture, which keep the ⭐-marks and a text selection mutually exclusive).
fn clear_transcript_marks(m: &VecModel<TranscriptLine>) {
    for j in 0..m.row_count() {
        if let Some(mut r) = m.row_data(j) {
            if r.marked {
                r.marked = false;
                m.set_row_data(j, r);
            }
        }
    }
}

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
    // G2b — reset the ⭐-mark editor state (the window is reused across sessions; the
    // fresh model already starts all-unmarked).
    win.set_marked_count(0);
    win.set_mark_anchor(-1); // R1.2: fresh open → no stale range anchor
    win.set_capture_pending(false);
    win.set_capture_text(slint::SharedString::default());
    win.set_capture_line_index(-1);
    let utts_owned: Vec<Utterance> = utts.to_vec();

    // G2b (2026-07-03) — ⭐ MULTI-mark → save, mirroring the tiles: toggle a line's mark,
    // recompute the count, seed the edit buffer when EXACTLY one is marked (trim-before-save).
    {
        let m = model.clone();
        let weak = win.as_weak();
        win.on_toggle_line_marked(move |idx, shift| {
            let Some(w) = weak.upgrade() else { return };
            let Ok(i) = usize::try_from(idx) else { return };
            if i >= m.row_count() {
                return;
            }
            // R1.2: SHIFT+click with a live anchor marks the whole range anchor..=i (adds to the set);
            // a plain click toggles this line + (re)sets the anchor. Mirrors the tiles' P5 logic.
            let shift_anchor = if shift {
                usize::try_from(w.get_mark_anchor()).ok()
            } else {
                None
            }
            .filter(|&a| a < m.row_count());
            if let Some(a) = shift_anchor {
                let (lo, hi) = if a <= i { (a, i) } else { (i, a) };
                for j in lo..=hi {
                    if let Some(mut r) = m.row_data(j) {
                        if !r.marked {
                            r.marked = true;
                            m.set_row_data(j, r);
                        }
                    }
                }
            } else {
                if let Some(mut row) = m.row_data(i) {
                    row.marked = !row.marked;
                    m.set_row_data(i, row);
                }
                w.set_mark_anchor(i32::try_from(i).unwrap_or(-1));
            }
            let mut count = 0_i32;
            let mut single_idx = -1_i32;
            let mut single = slint::SharedString::default();
            for j in 0..m.row_count() {
                if let Some(r) = m.row_data(j) {
                    if r.marked {
                        count += 1;
                        single_idx = i32::try_from(j).unwrap_or(-1);
                        single = r.text.clone();
                    }
                }
            }
            w.set_marked_count(count);
            w.set_capture_pending(false); // marking cancels a pending text selection
                                          // Re-seed the edit buffer ONLY when the SOLE-marked line changes, so an
                                          // in-progress edit survives marking/unmarking OTHER lines (tile review I-1).
            if count == 1 && single_idx != w.get_capture_line_index() {
                w.set_capture_text(single);
                w.set_capture_line_index(single_idx);
            }
        });
    }
    // «В память (N)»: join every marked line into ONE approved note (N==1 uses the edited
    // buffer), then clear the marks. Same coherent-memory join as the tiles (G2a).
    {
        let m = model.clone();
        let weak = win.as_weak();
        win.on_save_marked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_marked_count() == 1 {
                super::tile_copy::insert_approved_note(w.get_capture_text().as_str());
            } else {
                let mut joined = String::new();
                for j in 0..m.row_count() {
                    if let Some(r) = m.row_data(j) {
                        if r.marked {
                            let line = r.text.as_str().trim();
                            if line.is_empty() {
                                continue;
                            }
                            if !joined.is_empty() {
                                joined.push('\n');
                            }
                            joined.push_str(line);
                        }
                    }
                }
                super::tile_copy::insert_approved_note(&joined);
            }
            clear_transcript_marks(&m);
            w.set_marked_count(0);
            w.set_capture_line_index(-1);
            w.set_capture_text(slint::SharedString::default());
        });
    }
    {
        let m = model.clone();
        let weak = win.as_weak();
        win.on_clear_marks(move || {
            let Some(w) = weak.upgrade() else { return };
            clear_transcript_marks(&m);
            w.set_marked_count(0);
            w.set_capture_line_index(-1);
            w.set_capture_text(slint::SharedString::default());
        });
    }

    // Right-click text-selection capture (P2): slice the displayed line by the selection's
    // byte offsets → the SAME editor via `capture-pending` (mirrors the tile selection path).
    {
        let m = model.clone();
        let weak = win.as_weak();
        win.on_capture_line_selection(move |idx, a, c| {
            let Some(w) = weak.upgrade() else { return };
            let i = idx.max(0) as usize;
            let Some(row) = m.row_data(i) else {
                return;
            };
            let text = row.text.as_str();
            let (lo, hi) = if a <= c { (a, c) } else { (c, a) };
            let lo = super::tile_copy::char_boundary(text, usize::try_from(lo).unwrap_or(0));
            let hi = super::tile_copy::char_boundary(text, usize::try_from(hi).unwrap_or(0));
            let span = text.get(lo..hi).unwrap_or("").trim();
            if span.is_empty() {
                return;
            }
            // Selection + ⭐-marks share the one editor — keep them mutually exclusive.
            clear_transcript_marks(&m);
            w.set_marked_count(0);
            w.set_capture_line_index(-1);
            w.set_capture_text(span.into());
            w.set_capture_pending(true);
        });
    }
    {
        let weak = win.as_weak();
        win.on_save_capture(move || {
            let Some(w) = weak.upgrade() else { return };
            // Saving is explicit approval: keep the selected text verbatim.
            super::tile_copy::insert_approved_note(w.get_capture_text().as_str());
            w.set_capture_pending(false);
            w.set_capture_text(slint::SharedString::default());
        });
    }
    {
        let weak = win.as_weak();
        win.on_cancel_capture(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_capture_pending(false);
            w.set_capture_text(slint::SharedString::default());
        });
    }

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
    // Reused window — reset the speed ComboBox (index 0 = 1×) + volume slider (1×) so
    // the UI matches the fresh player (reset() above dropped any prior session's
    // player, which starts at 1×/1×).
    win.set_speed_index(0);
    win.set_volume(1.0);
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
    // Speed / volume-boost (owner req 2026-07-05). `ensure` first so a value chosen
    // before pressing play loads the player (paused) and the setting sticks — the
    // free fns are no-ops on an unloaded player.
    {
        let id = id.clone();
        win.on_set_speed(move |s| {
            if transcript_player::ensure(&id) {
                transcript_player::set_speed(s);
            }
        });
    }
    {
        let id = id.clone();
        win.on_set_volume(move |v| {
            if transcript_player::ensure(&id) {
                transcript_player::set_volume(v);
            }
        });
    }

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

/// MEMORY sanity bound on transcript rows built into the model: the FIRST N utterances
/// (chronological). This is NO LONGER an i16 bound — transcript.slint now renders the rows in
/// a VIRTUALIZED `ListView` (only visible rows are instantiated), so the list's height no longer
/// hits the SW-renderer's i16 (32767px) coordinate limit and the whole transcript can show. N is
/// kept only so a runaway session (audio left recording) can't build an unbounded model; 2000
/// covers a multi-hour meeting (~10 utt/min). "Copy all" still exports every line, and the footer
/// discloses any tail beyond N. Taking the FIRST N preserves the model-index == utterance-index
/// invariant that copy-selected / play-line / active-line rely on.
const TRANSCRIPT_DISPLAY_CAP: usize = 2000;

thread_local! {
    // The «Определить говорящих» result-poll timer — held so it outlives the run
    // closure; the timer clears itself on completion (and on window close).
    static DIAR_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
    // The model-INSTALL poll timer (V-1). Kept SEPARATE from DIAR_TIMER so a
    // reopen — which drops DIAR_TIMER to re-attach the result poll — can't abort
    // an in-flight model download, and the install poll's self-clear can't drop
    // the result poll. The two flows are mutually exclusive by gating
    // (install ⇄ needs_diar_models, run ⇄ can_diarize), but distinct timers make
    // that independence structural rather than incidental.
    static DIAR_INSTALL_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
    // suflyor H2 — the live job's shared slots (worker → UI poll) + its session
    // id, so a window closed + reopened mid-run RE-ATTACHES a poll to the running
    // job instead of spawning a second sidecar. UI-thread-only (like DIAR_TIMER).
    static DIAR_JOB: RefCell<Option<DiarJobHandles>> = const { RefCell::new(None) };
    // V-1 — the live model install's shared result slot (worker → UI poll), so a
    // close+reopen mid-download RE-ATTACHES a poll to the running install instead
    // of spawning a second one. UI-thread-only (like DIAR_JOB).
    static DIAR_INSTALL_JOB: RefCell<Option<DiarInstallSlot>> = const { RefCell::new(None) };
}

/// suflyor H2 — the running diarization job's cross-thread slots. The worker
/// (spawn_blocking) writes them; the UI-thread poll ([`start_diar_poll`]) reads
/// them. Cloned between the run callback, `DIAR_JOB`, and the poll, so a
/// re-attached poll after a close+reopen consumes the SAME job.
#[derive(Clone)]
struct DiarJobHandles {
    /// The terminal outcome (`None` while the job runs).
    slot: Arc<Mutex<Option<Result<Diarization, DiarFailure>>>>,
    /// The latest sidecar step message (taken once by the poll → status line).
    progress: Arc<Mutex<Option<String>>>,
    /// The session the job runs for — a re-attached poll paints ONLY when the
    /// window still shows this session (it may have been repurposed mid-run).
    session_id: String,
}

/// V-1 — the running model install's shared result slot. The worker
/// (spawn_blocking) writes the terminal outcome (`None` while it downloads);
/// the UI-thread poll ([`start_diar_install_poll`]) takes it. Cloned between
/// the install callback, `DIAR_INSTALL_JOB`, and the poll, so a re-attached
/// poll after a close+reopen consumes the SAME install.
type DiarInstallSlot = Arc<Mutex<Option<Result<(), String>>>>;

/// suflyor H3 — how a finished job failed, so the UI never paints a result that
/// was NOT persisted. `Run` = the sidecar/parse failed (path-safe reason via
/// `diarize::friendly_error`); `Save` = the run succeeded but the catalog write
/// failed (details go to the log ONLY — the store error chain can carry the
/// catalog path, so the shown line is a fixed generic).
enum DiarFailure {
    Run(String),
    Save,
}

/// A user-facing line for a rename/save failure (Rust-built RU is allowed; the
/// `.slint` strings stay English `@tr`). Shared const so the rename callback can
/// recognize — and clear — its OWN failure text on a later successful keystroke
/// without touching an in-flight job's progress text.
const DIAR_SAVE_FAILED_MSG: &str = "Не удалось сохранить результат.";
const DIAR_RENAME_FAILED_MSG: &str = "Не удалось сохранить имя говорящего.";

/// A distinct colour per speaker id (cycled), vivid enough on light + dark surfaces.
fn speaker_palette(id: i32) -> slint::Color {
    const P: &[(u8, u8, u8)] = &[
        (0x4F, 0x8A, 0xF7),
        (0xE8, 0x6A, 0x5C),
        (0x3F, 0xB0, 0x6B),
        (0xC9, 0x7A, 0xE0),
        (0xE0, 0x9B, 0x3A),
        (0x2E, 0xB5, 0xC0),
        (0xD1, 0x5E, 0x9C),
        (0x8A, 0x8F, 0x3A),
    ];
    let (r, g, b) = P[(id.max(0) as usize) % P.len()];
    slint::Color::from_rgb_u8(r, g, b)
}

/// «Вы» / unattributed colour — neutral grey, distinct from the coloured speakers.
fn neutral_speaker_color() -> slint::Color {
    slint::Color::from_rgb_u8(0x8A, 0x8A, 0x8A)
}

/// Display label for a system speaker: the user's rename, else «Говорящий N».
fn speaker_label(id: i32, names: &std::collections::BTreeMap<i32, String>) -> String {
    names
        .get(&id)
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("Говорящий {}", id + 1))
}

/// Reset every row to the role view («Микрофон»/«Система»); the Slint side uses the
/// theme accent for the colour in role view, so `speaker_color` is left as-is.
fn apply_role_labels(model: &Rc<VecModel<TranscriptLine>>, utts: &[Utterance]) {
    for i in 0..model.row_count() {
        let Some(mut row) = model.row_data(i) else {
            continue;
        };
        let mic = utts.get(i).map(|u| u.source == "mic").unwrap_or(false);
        row.speaker = SharedString::from(if mic {
            "Микрофон"
        } else {
            "Система"
        });
        model.set_row_data(i, row);
    }
}

/// Relabel every row for «По голосам»: mic → «Вы»; a system line → every
/// significantly overlapping speaker (one long STT block can contain a turn change);
/// an unattributed system line → «Система».
fn apply_voice_labels(
    model: &Rc<VecModel<TranscriptLine>>,
    utts: &[Utterance],
    diar: &Diarization,
) {
    let align = overlay_backend::diarize::align_all_speakers(utts, &diar.segments);
    for i in 0..model.row_count() {
        let Some(mut row) = model.row_data(i) else {
            continue;
        };
        let mic = utts.get(i).map(|u| u.source == "mic").unwrap_or(false);
        if mic {
            row.speaker = SharedString::from("Вы");
            row.speaker_color = neutral_speaker_color();
        } else if let Some(ids) = align.get(i).filter(|ids| !ids.is_empty()) {
            let labels: Vec<String> = ids
                .iter()
                .map(|id| speaker_label(*id, &diar.speaker_names))
                .collect();
            row.speaker = SharedString::from(labels.join(" + "));
            row.speaker_color = if ids.len() == 1 {
                speaker_palette(ids[0])
            } else {
                neutral_speaker_color()
            };
        } else {
            row.speaker = SharedString::from("Система");
            row.speaker_color = neutral_speaker_color();
        }
        model.set_row_data(i, row);
    }
}

/// The rename-list rows: the display speakers that actually attribute ≥1 system line.
fn speaker_rows(diar: &Diarization, utts: &[Utterance]) -> Vec<SpeakerRow> {
    let align = overlay_backend::diarize::align_all_speakers(utts, &diar.segments);
    let mut ids: Vec<i32> = align.into_iter().flatten().collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .map(|id| SpeakerRow {
            id,
            label: SharedString::from(speaker_label(id, &diar.speaker_names)),
            color: speaker_palette(id),
        })
        .collect()
}

fn set_speaker_list(win: &TranscriptWindow, diar: &Diarization, utts: &[Utterance]) {
    let rows = speaker_rows(diar, utts);
    win.set_speakers(ModelRc::from(Rc::new(VecModel::from(rows))));
}

/// suflyor H2 — start (replacing any prior) the UI-thread poll that consumes the
/// running diarization job for display: progress step → status line, then the
/// terminal outcome. The WORKER already persisted a successful result, so the Ok
/// arm only PAINTS (re-reading the authoritative state back from the catalog);
/// `paint_session_id` is the session the window shows now — a job that finishes
/// for a DIFFERENT session (the window was repurposed mid-run) just clears the
/// busy state instead of mislabelling another session's lines. Panic backstop:
/// a worker that freed the latch without posting an outcome (it unwound) fails
/// the UI clean instead of a forever-busy button.
fn start_diar_poll(
    weak: slint::Weak<TranscriptWindow>,
    store: StoreSlot,
    diar: Rc<RefCell<Option<Diarization>>>,
    model: Rc<VecModel<TranscriptLine>>,
    utts: Rc<Vec<Utterance>>,
    handles: DiarJobHandles,
    paint_session_id: String,
) {
    let poll = slint::Timer::default();
    let slot = handles.slot;
    let progress = handles.progress;
    let job_sid = handles.session_id;
    poll.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            if let Some(message) = progress.lock().ok().and_then(|mut value| value.take()) {
                if let Some(w) = weak.upgrade() {
                    w.set_diar_status(SharedString::from(message));
                }
            }
            let done = slot.lock().ok().and_then(|mut g| g.take());
            let Some(result) = done else {
                // Panic backstop: the latch is free but no outcome was posted —
                // the worker unwound. Fail clean (the RAII guard already freed
                // the latch, so a new job is possible once this clears).
                if !DIAR_BUSY.load(Ordering::Acquire) {
                    DIAR_TIMER.with(|t| *t.borrow_mut() = None); // stop + drop self
                    DIAR_JOB.with(|j| *j.borrow_mut() = None);
                    if let Some(w) = weak.upgrade() {
                        w.set_diarizing(false);
                        w.set_diar_status(SharedString::from("Не удалось определить говорящих."));
                    }
                }
                return;
            };
            DIAR_TIMER.with(|t| *t.borrow_mut() = None); // stop + drop self
            DIAR_JOB.with(|j| *j.borrow_mut() = None);
            let Some(w) = weak.upgrade() else {
                // Window closed — nothing to paint; the worker already persisted
                // the result, and the next open reads it from the catalog.
                return;
            };
            w.set_diarizing(false);
            match result {
                Ok(d) if job_sid == paint_session_id => {
                    // Re-read the persisted state so the UI can never diverge
                    // from the catalog (falls back to the worker's value if the
                    // read races a maintenance vacuum — same data, no failure).
                    let d = lock_store(&store)
                        .as_ref()
                        .and_then(|st| st.get_diarization(&job_sid).ok().flatten())
                        .unwrap_or(d);
                    apply_voice_labels(&model, &utts, &d);
                    set_speaker_list(&w, &d, &utts);
                    *diar.borrow_mut() = Some(d);
                    w.set_has_diarization(true);
                    w.set_by_voice(true);
                    // F — a fresh result carries no custom names; clear the guard and the
                    // confirm that may have triggered this re-run.
                    w.set_has_speaker_names(false);
                    w.set_confirm_rediar(false);
                    w.set_diar_status(SharedString::default());
                }
                Ok(_) => {
                    // The job was for another session (repurposed window): the
                    // worker persisted it; just drop the busy state here.
                    w.set_diar_status(SharedString::default());
                }
                Err(DiarFailure::Run(e)) => {
                    // I-5: surface the specific, path-safe reason (>3h / no speech)
                    // instead of a bare generic line.
                    w.set_diar_status(SharedString::from(
                        overlay_backend::diarize::friendly_error(&e),
                    ));
                }
                Err(DiarFailure::Save) => {
                    // suflyor H3 — the result was NOT saved: a generic line (the
                    // detail chain is in the log and can carry the catalog path),
                    // and NO voice labels / has-diarization flip.
                    w.set_diar_status(SharedString::from(DIAR_SAVE_FAILED_MSG));
                }
            }
        },
    );
    DIAR_TIMER.with(|t| *t.borrow_mut() = Some(poll));
}

/// V-1 — start (replacing any prior) the UI-thread poll that consumes the
/// running model install for display. The worker only downloads — the models
/// land on disk window-independently — so the Ok arm re-checks the fs and flips
/// can_diarize only when BOTH really landed (a partial install keeps the prompt
/// up). A close+reopen re-attaches this poll to the SAME slot via
/// `DIAR_INSTALL_JOB`; the worker's latch guard makes a second installer
/// impossible regardless of the per-window property. Panic backstop: a worker
/// that freed the latch without posting an outcome (it unwound) fails the UI
/// clean instead of a forever-busy button.
fn start_diar_install_poll(weak: slint::Weak<TranscriptWindow>, slot: DiarInstallSlot) {
    let poll = slint::Timer::default();
    poll.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(300),
        move || {
            let done = slot.lock().ok().and_then(|mut g| g.take());
            let Some(result) = done else {
                // Panic backstop (same as the diarization poll): the latch is
                // free but no outcome was posted — the worker unwound.
                if !DIAR_INSTALL_BUSY.load(Ordering::Acquire) {
                    DIAR_INSTALL_TIMER.with(|t| *t.borrow_mut() = None); // stop + drop self
                    DIAR_INSTALL_JOB.with(|j| *j.borrow_mut() = None);
                    if let Some(w) = weak.upgrade() {
                        w.set_installing_diar_models(false);
                        w.set_diar_status(SharedString::from("Не удалось скачать модели"));
                    }
                }
                return;
            };
            DIAR_INSTALL_TIMER.with(|t| *t.borrow_mut() = None); // stop + drop self
            DIAR_INSTALL_JOB.with(|j| *j.borrow_mut() = None);
            let Some(w) = weak.upgrade() else {
                // Window closed — nothing to paint; the models are on disk and
                // the next open recomputes readiness from the fs.
                return;
            };
            w.set_installing_diar_models(false);
            match result {
                Ok(()) => {
                    // Re-check the fs rather than assume — only enable detect if BOTH
                    // models really landed (a partial install keeps the prompt up).
                    let ready = overlay_backend::diarize::models_ready();
                    w.set_can_diarize(ready);
                    w.set_needs_diar_models(!ready);
                    w.set_diar_status(SharedString::default());
                }
                Err(e) => {
                    w.set_diar_status(SharedString::from("Не удалось скачать модели"));
                    log::warn!("diar model install failed: {e}");
                }
            }
        },
    );
    DIAR_INSTALL_TIMER.with(|t| *t.borrow_mut() = Some(poll));
}

/// I (2026-07-05) — case-insensitive substring match (full Unicode lowercasing, so Cyrillic
/// folds too). Returns the indices of `texts` containing `query`; empty/blank query → none.
/// Pure — the search's only non-trivial logic, so it carries the unit test.
fn transcript_search_hits(texts: &[&str], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    texts
        .iter()
        .enumerate()
        .filter(|(_, t)| t.to_lowercase().contains(q.as_str()))
        .map(|(i, _)| i)
        .collect()
}

/// I — next hit index from cursor `cur` (-1 = fresh) stepping `dir` (±1) over `n` hits (n > 0).
/// Fresh: › → 0 (first), ‹ → n-1 (last). Positioned: wrap both ways. Pure — unit-tested.
fn next_hit_index(cur: i32, dir: i32, n: i32) -> i32 {
    if cur < 0 {
        if dir >= 0 {
            0
        } else {
            n - 1
        }
    } else {
        (cur + dir).rem_euclid(n)
    }
}

/// I (2026-07-05, owner top priority) — in-transcript word search. `search-edited` re-flags the
/// matching rows (`matched`) in ONE model pass and collects their indices; `search-jump(±1)` moves a
/// cursor over the hits and reuses the existing `play-line` (seek + play the moment) plus the
/// `scroll-to-line` fn (visual). Per-window state; reset on every (re)open (the window is reused).
fn wire_transcript_search(win: &TranscriptWindow, model: &Rc<VecModel<TranscriptLine>>) {
    win.set_search_query(SharedString::default());
    win.set_search_count(0);
    win.set_search_pos(0);

    let hits: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let cursor: Rc<std::cell::Cell<i32>> = Rc::new(std::cell::Cell::new(-1));

    {
        let weak = win.as_weak();
        let m = model.clone();
        let hits = hits.clone();
        let cursor = cursor.clone();
        win.on_search_edited(move |query| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            // Row texts → the pure (unit-tested) helper computes the ascending hit indices; then
            // apply `matched` (write only on a flip — the virtualized list repaints just changed rows).
            let texts: Vec<String> = (0..m.row_count())
                .map(|j| {
                    m.row_data(j)
                        .map(|r| r.text.to_string())
                        .unwrap_or_default()
                })
                .collect();
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            let found = transcript_search_hits(&refs, query.as_str());
            for j in 0..m.row_count() {
                if let Some(mut r) = m.row_data(j) {
                    let is_m = found.binary_search(&j).is_ok();
                    if r.matched != is_m {
                        r.matched = is_m;
                        m.set_row_data(j, r);
                    }
                }
            }
            w.set_search_count(found.len() as i32);
            w.set_search_pos(0);
            *hits.borrow_mut() = found;
            cursor.set(-1);
        });
    }
    {
        let weak = win.as_weak();
        let hits = hits.clone();
        let cursor = cursor.clone();
        win.on_search_jump(move |dir| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let list = hits.borrow();
            let n = list.len() as i32;
            if n == 0 {
                return;
            }
            let next = next_hit_index(cursor.get(), dir, n);
            cursor.set(next);
            let row = list[next as usize] as i32;
            w.set_search_pos(next + 1);
            w.invoke_scroll_to_line(row); // visual (approximate)
            w.invoke_play_line(row); // audio — seek to the moment + play (exact)
        });
    }
}

/// D3 — «По голосам» diarization wiring: the role/voice toggle, the «Определить
/// говорящих» run (worker → poll-timer → persist + repaint), and speaker rename.
/// Relabelling MUTATES the shared line model in place (same rows, only the speaker
/// label + colour), so the copy / player / star wiring stays valid.
#[allow(clippy::too_many_arguments)]
fn wire_transcript_diarization(
    win: &TranscriptWindow,
    store: &StoreSlot,
    rt_handle: &tokio::runtime::Handle,
    session_id: &str,
    session_finished: bool,
    utts_display: &[Utterance],
    model: &Rc<VecModel<TranscriptLine>>,
) {
    // suflyor H2 — a live job (this window's or a closed one's) outlives this
    // wiring: grab its handles BEFORE dropping the poll timer so the job's poll
    // is re-attached below (a close+reopen keeps consuming the running job
    // instead of spawning a second sidecar). No live job → the timer being
    // dropped is just a stale one from a prior session's reused window.
    let live_job = DIAR_BUSY
        .load(Ordering::Acquire)
        .then(|| DIAR_JOB.with(|j| j.borrow().clone()))
        .flatten();
    let job_busy = live_job.is_some();
    DIAR_TIMER.with(|t| *t.borrow_mut() = None);
    // V-1 — same for the model install: grab the running download's slot BEFORE
    // dropping its poll timer so the poll re-attaches below. Latch free + a slot
    // still present = the install finished while the window was closed: readiness
    // is recomputed from disk below, so just clear the stale handle — the new
    // window must not show a busy button over an already-landed download.
    let live_install = DIAR_INSTALL_BUSY
        .load(Ordering::Acquire)
        .then(|| DIAR_INSTALL_JOB.with(|j| j.borrow().clone()))
        .flatten();
    let install_busy = live_install.is_some();
    DIAR_INSTALL_TIMER.with(|t| *t.borrow_mut() = None);
    if !install_busy {
        DIAR_INSTALL_JOB.with(|j| *j.borrow_mut() = None);
    }

    let has_sys_audio_ms = utts_display
        .iter()
        .any(|u| u.source == "system" && u.audio_ms.is_some());
    // Diarizable EXCEPT for the models: a finished session with aligned system audio +
    // a saved recording. Split out so we can offer to install the models (V-1) rather
    // than hide the whole feature when they're absent.
    let session_diarizable = session_finished
        && has_sys_audio_ms
        && overlay_backend::session_audio::session_has_recordings(session_id);
    let models_ready = overlay_backend::diarize::models_ready();
    let can_diarize = session_diarizable && models_ready;
    let needs_diar_models = session_diarizable && !models_ready;
    // I-1: warn when the recording predates the wall-clock-padding fix — system.wav
    // shorter than the transcript span means audio_ms→sample drifts and speaker labels
    // can land on the wrong lines.
    let timeline_unreliable = session_diarizable
        && !overlay_backend::diarize::timeline_reliable(
            overlay_backend::session_audio::system_recording_ms(session_id),
            utts_display,
        );

    let diar: Rc<RefCell<Option<Diarization>>> = Rc::new(RefCell::new(
        lock_store(store)
            .as_ref()
            .and_then(|st| st.get_diarization(session_id).ok().flatten()),
    ));
    let utts_rc: Rc<Vec<Utterance>> = Rc::new(utts_display.to_vec());

    win.set_can_diarize(can_diarize);
    win.set_needs_diar_models(needs_diar_models);
    // V-1 — honest busy state: a download that outlived the window shows as
    // downloading on (re)open; the re-attached poll clears it when it lands.
    win.set_installing_diar_models(install_busy);
    win.set_timeline_unreliable(timeline_unreliable);
    win.set_has_diarization(diar.borrow().is_some());
    // F — arm the re-detect guard iff the stored result already has custom names; reset the
    // confirm on this REUSED window so it never opens stale on a fresh open (CLAUDE.md gotcha #1).
    win.set_has_speaker_names(
        diar.borrow()
            .as_ref()
            .is_some_and(|d| d.speaker_names.values().any(|n| !n.trim().is_empty())),
    );
    win.set_confirm_rediar(false);
    win.set_by_voice(false);
    // suflyor H2 — honest busy state: a job still running (from a closed window
    // or a rep here) shows as running on (re)open; the re-attached poll below
    // clears it when the job lands. The run callback's latch gate makes a second
    // sidecar impossible regardless of what this property says.
    win.set_diarizing(job_busy);
    win.set_diar_status(SharedString::from(if job_busy {
        "Определение говорящих…"
    } else {
        ""
    }));
    let default_count = diar
        .borrow()
        .as_ref()
        .map(|d| (d.num_speakers.max(1) as i32).clamp(1, 8))
        .unwrap_or(0);
    win.set_speaker_count(default_count);
    // Reset the rename list too (the window is reused) — masked while by-voice=false,
    // but defends against a stale prior-session list if the gating ever changes.
    win.set_speakers(ModelRc::from(Rc::new(VecModel::<SpeakerRow>::default())));
    apply_role_labels(model, &utts_rc);

    // suflyor H2 — re-attach the result poll to a job that outlived the window:
    // the worker persists the result itself, and this poll consumes it for the
    // UI (painting only if this session is still the one on screen).
    if let Some(handles) = live_job {
        start_diar_poll(
            win.as_weak(),
            store.clone(),
            diar.clone(),
            model.clone(),
            utts_rc.clone(),
            handles,
            session_id.to_string(),
        );
    }
    // V-1 — re-attach the install poll to a download that outlived the window
    // (same lifecycle as above; the worker commits on disk window-independently).
    if let Some(slot) = live_install {
        start_diar_install_poll(win.as_weak(), slot);
    }

    // Toggle role ↔ voice.
    {
        let weak = win.as_weak();
        let model_c = model.clone();
        let diar_c = diar.clone();
        let utts_c = utts_rc.clone();
        win.on_toggle_by_voice(move |v| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if v {
                if let Some(d) = diar_c.borrow().as_ref() {
                    apply_voice_labels(&model_c, &utts_c, d);
                    set_speaker_list(&w, d, &utts_c);
                }
            } else {
                apply_role_labels(&model_c, &utts_c);
            }
            w.set_by_voice(v);
        });
    }

    // Rename → persist → reload → repaint (stays in voice view).
    {
        let weak = win.as_weak();
        let store_c = store.clone();
        let diar_c = diar.clone();
        let model_c = model.clone();
        let utts_c = utts_rc.clone();
        let sid = session_id.to_string();
        win.on_rename_speaker(move |id, name| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            // suflyor H3 — the rename Result is authoritative: on failure the
            // name was NOT saved, so the transcript is NOT relabelled from a
            // state that never reached the catalog and no success is painted.
            let renamed = {
                let slot = lock_store(&store_c);
                match slot.as_ref() {
                    Some(st) => st.rename_speaker(&sid, id, name.as_str()),
                    None => return,
                }
            };
            if let Err(e) = renamed {
                log::warn!("diar: rename speaker {id} NOT persisted: {e:#}");
                w.set_diar_status(SharedString::from(DIAR_RENAME_FAILED_MSG));
                return;
            }
            *diar_c.borrow_mut() = lock_store(&store_c)
                .as_ref()
                .and_then(|st| st.get_diarization(&sid).ok().flatten());
            if let Some(d) = diar_c.borrow().as_ref() {
                // F-fix (fable): the rename now commits per-keystroke (`edited`), so do NOT rebuild
                // the speaker list here — that recreates the focused LineEdit on every keystroke and
                // makes typing impossible. The field already shows the typed text; relabel the
                // transcript rows live, and keep the re-detect guard in sync below.
                apply_voice_labels(&model_c, &utts_c, d);
                let has_names = d.speaker_names.values().any(|n| !n.trim().is_empty());
                w.set_has_speaker_names(has_names);
                // Clear OUR failure text once a keystroke saved again — but never
                // an in-flight job's progress line (it only re-posts on a new step).
                if w.get_diar_status().as_str() == DIAR_RENAME_FAILED_MSG {
                    w.set_diar_status(SharedString::default());
                }
                log::debug!("diar: rename speaker {id} (has_custom_names={has_names})");
            }
        });
    }

    // Run diarization: spawn the blocking sidecar off-thread behind the process-
    // global latch (suflyor H2 — one job process-wide; a close+reopen re-attaches
    // instead of spawning a second sidecar). The WORKER persists the result
    // through its own catalog handle (WAL + busy_timeout make that safe) BEFORE
    // releasing the latch, so a completed result survives the window closing and
    // a failed save is never painted as a success (suflyor H3). The UI-thread
    // poll ([`start_diar_poll`]) only consumes the outcome for display.
    {
        let weak = win.as_weak();
        let store_c = store.clone();
        let diar_c = diar.clone();
        let model_c = model.clone();
        let utts_c = utts_rc.clone();
        let rt = rt_handle.clone();
        let sid = session_id.to_string();
        win.on_run_diarization(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let Some(guard) = try_acquire_busy(&DIAR_BUSY) else {
                // A job from another (possibly closed) window is still running:
                // honest busy state, NO second sidecar. The running job's poll
                // (re-attached on reopen) clears this when it lands.
                w.set_diarizing(true);
                w.set_diar_status(SharedString::from("Определение говорящих уже выполняется…"));
                // Dismiss the re-detect confirm (if this run came through it) so
                // the busy line is visible and the card doesn't linger.
                w.set_confirm_rediar(false);
                return;
            };
            let count = w.get_speaker_count().clamp(0, 8);
            w.set_diarizing(true);
            w.set_diar_status(SharedString::from("Определение говорящих…"));

            let handles = DiarJobHandles {
                slot: Arc::new(Mutex::new(None)),
                progress: Arc::new(Mutex::new(None)),
                session_id: sid.clone(),
            };
            DIAR_JOB.with(|j| *j.borrow_mut() = Some(handles.clone()));
            {
                let slot_w = handles.slot.clone();
                let progress_w = handles.progress.clone();
                let sid_w = sid.clone();
                let utts_owned: Vec<Utterance> = utts_c.as_ref().clone();
                rt.spawn_blocking(move || {
                    // Held until the outcome is posted — on EVERY exit, including
                    // a panic unwinding the sidecar run (RAII; the latch can't be
                    // leaked and wedge the feature until restart).
                    let guard = guard;
                    let outcome = match overlay_backend::diarize::run_diarization(
                        &sid_w,
                        count,
                        &utts_owned,
                        &|overlay_backend::diarize::Progress::Step(message)| {
                            if let Ok(mut value) = progress_w.lock() {
                                *value = Some(message);
                            }
                        },
                    ) {
                        Ok(d) => {
                            // Persist HERE, off the window: a worker-owned catalog
                            // handle (the archive window does the same). The poll's
                            // Ok arm only paints what this write committed.
                            match open_default_store().and_then(|st| st.put_diarization(&d)) {
                                Ok(()) => Ok(d),
                                Err(e) => {
                                    log::warn!("diarization result NOT persisted: {e:#}");
                                    Err(DiarFailure::Save)
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("diarization failed: {e:#}");
                            Err(DiarFailure::Run(format!("{e:#}")))
                        }
                    };
                    if let Ok(mut g) = slot_w.lock() {
                        *g = Some(outcome);
                        // Publish and release the latch while still holding the
                        // slot lock: the poll cannot consume a terminal result
                        // and race a final, still-busy latch state.
                        drop(guard);
                    }
                });
            }
            start_diar_poll(
                weak.clone(),
                store_c.clone(),
                diar_c.clone(),
                model_c.clone(),
                utts_c.clone(),
                handles,
                sid.clone(),
            );
        });
    }

    // V-1 — install the diarization models off-thread behind the process-global
    // latch (the download outlives the window; a close+reopen re-attaches the
    // poll via DIAR_INSTALL_JOB instead of spawning a second `install_models`
    // on the same .download/staging/live files). The WORKER only downloads —
    // the models commit on disk window-independently — and the UI-thread poll
    // ([`start_diar_install_poll`]) consumes the outcome for display.
    {
        let weak = win.as_weak();
        let rt = rt_handle.clone();
        win.on_install_diar_models(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let Some(guard) = try_acquire_busy(&DIAR_INSTALL_BUSY) else {
                // An install from another (possibly closed) window is still
                // running: honest busy state, NO second worker. The running
                // install's poll (re-attached on reopen) clears this when it lands.
                w.set_installing_diar_models(true);
                w.set_diar_status(SharedString::default());
                return;
            };
            w.set_installing_diar_models(true);
            w.set_diar_status(SharedString::default());

            let slot: DiarInstallSlot = Arc::new(Mutex::new(None));
            DIAR_INSTALL_JOB.with(|j| *j.borrow_mut() = Some(slot.clone()));
            {
                let slot_w = slot.clone();
                rt.spawn_blocking(move || {
                    // Held until the outcome is posted — on EVERY exit, including
                    // a panic unwinding the download (RAII; the latch can't be
                    // leaked and wedge the feature until restart).
                    let guard = guard;
                    let cancel = AtomicBool::new(false);
                    // ponytail: no per-file progress marshalling — the button's
                    // "Downloading…" state is enough for a one-time ~30 MB fetch.
                    let r = overlay_backend::diar_install::install_models(&cancel, &|_| {})
                        .map_err(|e| format!("{e:#}"));
                    if let Ok(mut g) = slot_w.lock() {
                        *g = Some(r);
                        // Keep result publication + latch release ordered for
                        // the poll (same contract as the diarization worker).
                        drop(guard);
                    }
                });
            }
            start_diar_install_poll(weak.clone(), slot);
        });
    }
}

/// Open a READ-ONLY structured transcript window for a session (ТЗ1). Reuses the
/// slot if already open; otherwise builds the per-line model from the session's
/// utterances and presents the window stealth-aware. The model is (re)built on
/// EVERY open so a reused window never shows a prior session.
pub(crate) fn open_transcript(
    slot: &Rc<RefCell<Option<TranscriptWindow>>>,
    session: Option<&Session>,
    utts: &[Utterance],
    store: &StoreSlot,
    rt_handle: &tokio::runtime::Handle,
) {
    // Build the model once — shared by the reuse + first-open paths.
    let session_start = session.and_then(|s| s.started_at_ms).filter(|&ms| ms > 0);
    // D3 — a finished session is a precondition to diarize (a live session's WAV is
    // unfinalized). `crashed`/`active` sessions can't.
    let session_finished = session.map(|s| s.status == "completed").unwrap_or(false);
    let heading = session
        .map(|s| session_title(s.started_at_ms, &s.id))
        .unwrap_or_default();
    // C1/P2 — compute each utterance's display offset in ORIGINAL order (the F1 fallback reads the
    // PREVIOUS utterance), THEN sort utterances + rows TOGETHER by the displayed clock (audio start).
    // Rows arrive in STT-FINALIZE order (unix_ms) but the shown timecode is the audio START; STT
    // latency (~1-25s) makes those orders differ → out-of-order timecodes at the tail. Sorting one
    // SHARED set keeps display, "Copy all" and copy-selected consistent AND preserves the
    // model-index == utterance-index invariant the copy path relies on. Diagnosis: latency (H2), NOT
    // capture-epoch skew (per-source wall-audio deltas matched ~6s), so no audio.rs change. Stable,
    // and a no-op for old (audio_ms-less) sessions whose fallback offsets are already monotonic.
    let mut indexed: Vec<(Option<i64>, &Utterance)> = utts
        .iter()
        .enumerate()
        .take(TRANSCRIPT_DISPLAY_CAP) // memory sanity bound (not i16 — ListView virtualizes)
        .map(|(i, u)| {
            (
                overlay_backend::session_audio::line_start_offset_ms(utts, i, session_start),
                u,
            )
        })
        .collect();
    indexed.sort_by_key(|(off, _)| off.unwrap_or(0));
    // The sorted utterances that BACK the rows — pass THESE (not raw `utts`) to the copy/selection
    // wiring so a checked row index maps to the right utterance.
    let utts_display: Vec<Utterance> = indexed.iter().map(|(_, u)| (*u).clone()).collect();
    let lines: Vec<TranscriptLine> = indexed
        .iter()
        .map(|(off, u)| TranscriptLine {
            offset_label: off.map(fmt_offset).unwrap_or_default().into(),
            speaker: SharedString::from(if u.source == "mic" {
                "Микрофон"
            } else {
                "Система"
            }),
            // Role view uses the theme accent (Slint side); this is only read in the
            // «По голосам» view, where `rebuild_speaker_labels` overwrites it.
            speaker_color: slint::Color::from_rgb_u8(0, 0, 0),
            text: SharedString::from(overlay_backend::text::collapse_ws(&u.text)),
            display_text: SharedString::from(slint_replay::math_display::normalize_math_display(
                &overlay_backend::text::collapse_ws(&u.text),
            )),
            checked: false,
            marked: false,
            start_ms: off.unwrap_or(0) as i32,
            matched: false,
        })
        .collect();
    // Utterances hidden by the display cap — the footer discloses this (Copy all
    // still exports every line). 0 when the whole transcript fits under the cap.
    let overflow = utts.len().saturating_sub(TRANSCRIPT_DISPLAY_CAP) as i32;
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
        win.set_overflow_count(overflow);
        wire_transcript_actions(win, &model, &utts_display, session_start);
        wire_transcript_player(win, &session_id, &model);
        wire_transcript_search(win, &model);
        wire_transcript_diarization(
            win,
            store,
            rt_handle,
            &session_id,
            session_finished,
            &utts_display,
            &model,
        );
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
    win.set_overflow_count(overflow);
    wire_transcript_actions(&win, &model, &utts_display, session_start);
    wire_transcript_player(&win, &session_id, &model);
    wire_transcript_search(&win, &model);
    wire_transcript_diarization(
        &win,
        store,
        rt_handle,
        &session_id,
        session_finished,
        &utts_display,
        &model,
    );

    {
        let slot_c = slot.clone();
        let weak = win.as_weak();
        win.on_close_requested(move || {
            transcript_player::reset(); // stop audio + poll timer when the window closes
                                        // suflyor H2 / V-1 — dropping the polls only stops the cosmetic
                                        // consumption: a running job holds its process-global latch, lands
                                        // the outcome window-independently (the diar worker persists from
                                        // its own catalog handle; the install commits on disk), and a
                                        // reopen re-attaches a poll via DIAR_JOB / DIAR_INSTALL_JOB — so
                                        // closing neither loses the result nor frees the latch for a
                                        // second worker.
            DIAR_TIMER.with(|t| *t.borrow_mut() = None);
            DIAR_INSTALL_TIMER.with(|t| *t.borrow_mut() = None);
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
/// a heading (label + human counts), the transcript, then the AI Q&A.
/// Pure → unit-tested.
fn build_session_markdown(
    session: Option<&Session>,
    utterances: &[Utterance],
    ai_turns: &[AiTurn],
) -> String {
    let mut out = String::new();
    if let Some(s) = session {
        out.push_str(&format!("# {}\n\n", session_title(s.started_at_ms, &s.id)));
        // The SAME human wording as the archive rows (the body is Russian,
        // like the rest of this markdown) — no code-like `lines N · ai N`
        // metadata and no "—" dash for an unknown model.
        out.push_str(&format!("Стенограмма: {}", s.transcript_lines));
        out.push_str(&format!(" · ИИ: {}", s.ai_turns_count));
        if let Some(model) = s.ai_model.as_deref().filter(|m| !m.is_empty()) {
            out.push_str(&format!(" · {model}"));
        }
        if s.total_cost_microcents > 0 {
            // SessionRow stores i64; the >0 guard makes the checked conversion exact.
            let micro = u64::try_from(s.total_cost_microcents).unwrap_or(0);
            out.push_str(&format!(
                " · ${:.3}",
                overlay_backend::ai::microcents_to_usd(micro)
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
            let text = overlay_backend::text::collapse_ws(&u.text);
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
    fn text_ask_profile_label_follows_ui_language() {
        assert_eq!(
            text_ask_profile_label_for(Some("Interview"), false, false),
            "Profile: Interview"
        );
        assert_eq!(
            text_ask_profile_label_for(None, true, false),
            "Profile: custom context"
        );
        assert_eq!(
            text_ask_profile_label_for(None, false, true),
            "Профиль: не задан"
        );
    }

    /// Pins the one-job contract on a private atomic, so parallel tests never
    /// touch either process-global latch.
    #[test]
    fn diar_latch_is_a_single_job_gate() {
        let latch = AtomicBool::new(false);
        assert!(!latch.load(Ordering::Acquire), "a fresh latch is free");

        let g1 = try_acquire_busy(&latch);
        assert!(g1.is_some(), "first acquire on a free latch must succeed");
        assert!(latch.load(Ordering::Acquire));

        let g2 = try_acquire_busy(&latch);
        assert!(g2.is_none(), "a second acquire while held must fail");
        assert!(
            latch.load(Ordering::Acquire),
            "a FAILED acquire must NOT free the held latch (then vs then_some)"
        );

        drop(g1);
        assert!(
            !latch.load(Ordering::Acquire),
            "dropping the guard releases the latch"
        );

        let g3 = try_acquire_busy(&latch);
        assert!(g3.is_some(), "the latch is reusable after release");
        assert!(latch.load(Ordering::Acquire));
        drop(g3);
        assert!(!latch.load(Ordering::Acquire));
    }

    #[test]
    fn diar_failure_messages_are_distinct_and_non_empty() {
        assert!(!DIAR_SAVE_FAILED_MSG.trim().is_empty());
        assert!(!DIAR_RENAME_FAILED_MSG.trim().is_empty());
        assert_ne!(DIAR_SAVE_FAILED_MSG, DIAR_RENAME_FAILED_MSG);
    }

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
    fn transcript_search_hits_case_insensitive_incl_cyrillic() {
        let texts = [
            "Обсудили бюджет на квартал",
            "Потом про найм",
            "Вернулись к БЮДЖЕТУ и срокам",
        ];
        // Case-insensitive substring; Cyrillic folds → rows 0 and 2 match «бюджет».
        assert_eq!(transcript_search_hits(&texts, "бюджет"), vec![0, 2]);
        assert_eq!(transcript_search_hits(&texts, "  НАЙМ  "), vec![1]); // trims + folds case
        assert_eq!(transcript_search_hits(&texts, "xyz"), Vec::<usize>::new()); // no match
        assert_eq!(transcript_search_hits(&texts, "   "), Vec::<usize>::new()); // blank → none
    }

    #[test]
    fn next_hit_index_fresh_and_wrap() {
        // Fresh (cur -1): › → first (0), ‹ → last (n-1) — NOT n-2.
        assert_eq!(next_hit_index(-1, 1, 5), 0);
        assert_eq!(next_hit_index(-1, -1, 5), 4);
        // Positioned: step forward/back.
        assert_eq!(next_hit_index(0, 1, 5), 1);
        assert_eq!(next_hit_index(2, -1, 5), 1);
        // Wrap both ends.
        assert_eq!(next_hit_index(4, 1, 5), 0);
        assert_eq!(next_hit_index(0, -1, 5), 4);
        // Single hit — every move stays on 0.
        assert_eq!(next_hit_index(-1, -1, 1), 0);
        assert_eq!(next_hit_index(0, 1, 1), 0);
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
    fn session_row_uses_localized_human_counts() {
        let row = session_to_row(
            &sample_session(),
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            true,
        );
        // v0.17.2 — the label is МСК wall-clock from started_at_ms, not the UTC id.
        // v0.22.0 — a COMPLETED session has no status prefix (the "done " was
        // dropped so a named/timed title reads clean).
        assert!(
            row.title.as_str().starts_with("24.05.2026 03:00:00 (МСК)"),
            "got {:?}",
            row.title
        );
        // UX-clarity — short LOCALIZED labels instead of code-like metadata.
        assert!(row.subtitle.as_str().contains("Стенограмма: 12"));
        assert!(row.subtitle.as_str().contains("ИИ: 3"));
        assert!(row.subtitle.as_str().contains("gemma"));
        assert_eq!(row.meta.as_str(), ""); // zero cost → blank meta

        let en = session_to_row(
            &sample_session(),
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            false,
        );
        assert!(en.subtitle.as_str().contains("Transcript: 12"));
        assert!(en.subtitle.as_str().contains("AI: 3"));
    }

    /// Regression guard (UX-clarity): no code-like row metadata or raw internal
    /// state token may reach a visible archive row — in EITHER language.
    #[test]
    fn session_row_never_shows_raw_metadata_or_state() {
        for ru in [true, false] {
            for status in ["completed", "crashed", "active"] {
                let mut s = sample_session();
                s.status = status.into();
                s.ai_model = None; // the old code showed a "—" dash here
                let row =
                    session_to_row(&s, &no_recordings(), &no_recordings(), &no_recordings(), ru);
                let visible = format!("{} | {} | {}", row.title, row.subtitle, row.meta);
                for raw in ["lines ", "ai ", "—", "crashed", "active"] {
                    assert!(!visible.contains(raw), "raw token {raw:?} in {visible:?}");
                }
            }
        }
    }

    #[test]
    fn session_row_flags_abnormal_status_with_a_localized_word() {
        let mut s = sample_session();
        s.status = "crashed".into();
        let ru = session_to_row(
            &s,
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            true,
        );
        // The title stays clean; the status reads as a word in the subtitle.
        assert!(ru.title.as_str().starts_with("24.05.2026 03:00:00 (МСК)"));
        assert!(
            ru.subtitle.as_str().ends_with("Прервана"),
            "got {:?}",
            ru.subtitle
        );

        s.status = "active".into();
        let en = session_to_row(
            &s,
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            false,
        );
        assert!(
            en.subtitle.as_str().ends_with("In progress"),
            "got {:?}",
            en.subtitle
        );
    }

    #[test]
    fn session_row_shows_cost_when_nonzero() {
        let mut s = sample_session();
        s.total_cost_microcents = 2_400_000; // $0.024
        let row = session_to_row(
            &s,
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            true,
        );
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
        let row = hit_to_row(
            &h,
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            true,
        );
        // Hits carry only the UTC id stamp → parsed + shifted to МСК (+3h).
        assert!(
            row.title
                .as_str()
                .starts_with("Поиск: 04.06.2026 12:30:00 (МСК)"),
            "got {:?}",
            row.title
        );
        assert_eq!(row.meta.as_str(), "ответ"); // localized hit kind
        assert_eq!(row.subtitle.as_str(), "a key value structure"); // whitespace collapsed

        let en = hit_to_row(
            &h,
            &no_recordings(),
            &no_recordings(),
            &no_recordings(),
            false,
        );
        assert!(
            en.title
                .as_str()
                .starts_with("Search: 04.06.2026 12:30:00 (МСК)"),
            "got {:?}",
            en.title
        );
        assert_eq!(en.meta.as_str(), "answer");
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
