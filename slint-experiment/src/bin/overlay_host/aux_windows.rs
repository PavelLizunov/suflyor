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

use super::{tile_copy, transcript_player};
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
use slint::Model;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

mod archive;
mod help_palette;
mod text_ask;
mod transcript;

use archive::{session_title, spawn_content_tile};
pub(super) use archive::open_archive;
pub(super) use help_palette::{open_help, open_palette};
pub(super) use text_ask::open_text_ask;
pub(super) use transcript::{fmt_offset, open_transcript};

/// The archive's catalog handle, opened lazily OFF the event loop. `None` until
/// the open worker lands — or permanently `None` if the catalog can't be opened,
/// which puts the window in its "unavailable" state. Shared by every archive
/// closure AND the open worker (which publishes the `Send` `Store` back via
/// `invoke_from_event_loop`), so it is an `Arc<Mutex<..>>`. Callers hold the lock
/// only for a single read/write and drop it before re-entrant UI callbacks.
pub(super) type StoreSlot = Arc<Mutex<Option<Store>>>;

/// Lock the catalog slot, recovering a poisoned lock (the codebase mutex idiom).
/// Contention can't happen in practice — every access runs on the event loop.
pub(super) fn lock_store(slot: &StoreSlot) -> std::sync::MutexGuard<'_, Option<Store>> {
    match slot.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Shared RAII guard for process-global background-job latches.
pub(super) struct BusyGuard<'a>(pub(super) &'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(super) fn try_acquire_busy(busy: &AtomicBool) -> Option<BusyGuard<'_>> {
    busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
        .then(|| BusyGuard(busy))
}
