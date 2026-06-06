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
pub(crate) fn open_archive(
    archive_ref: &Rc<RefCell<Option<ArchiveWindow>>>,
    tiles_ref: &TileWindows,
    state: &slint_replay::app_state::SharedState,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
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

    // Open ONE catalog handle for this browse session (reused by the closures
    // below). On failure the window degrades to an "unavailable" state.
    let store: Option<Rc<RefCell<Store>>> = match open_default_store() {
        Ok(s) => Some(Rc::new(RefCell::new(s))),
        Err(e) => {
            eprintln!("[overlay-host] archive: catalog open failed: {e}");
            None
        }
    };

    match store.as_ref() {
        Some(store_rc) => {
            let sessions = store_rc.borrow().list_sessions().unwrap_or_default();
            win.set_summary(SharedString::from(format!("{} 🗄", sessions.len())));
            let rows: Vec<ArchiveRow> = sessions.iter().map(session_to_row).collect();
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
                    .map(session_to_row)
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
                        .map(hit_to_row)
                        .collect()
                }
            };
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
            let title = pretty_session_label(&sid);
            let md = build_session_markdown(session.as_ref(), &utts, &turns);
            spawn_content_tile(&title, "archive", &md, &tiles_c, &state_c, &wov);
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

    present_window_stealth_aware(&win, |hwnd| {
        // Keep the archive out of the taskbar / Alt-Tab too (stealth existence
        // leak — same as palette / help / text-ask).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
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

/// Status → a compact language-neutral label for the row title.
fn status_glyph(status: &str) -> &'static str {
    match status {
        "completed" => "done",
        "crashed" => "crashed",
        "active" => "active",
        _ => "session",
    }
}

/// Map an indexed [`Session`] to an archive list row. Counts are emoji-coded
/// as plain text counts so the row needs no per-language string;
/// the cost shows only when non-zero (local runs are $0 → blank).
fn session_to_row(s: &Session) -> ArchiveRow {
    let label = pretty_session_label(&s.id);
    let model = s.ai_model.as_deref().unwrap_or("—");
    let subtitle = format!(
        "lines {} · ai {} · {model}",
        s.transcript_lines, s.ai_turns_count
    );
    let meta = if s.total_cost_microcents > 0 {
        format!("${:.3}", (s.total_cost_microcents as f64) / 100_000_000.0)
    } else {
        String::new()
    };
    ArchiveRow {
        id: SharedString::from(s.id.clone()),
        title: SharedString::from(format!("{} {label}", status_glyph(&s.status))),
        subtitle: SharedString::from(subtitle),
        meta: SharedString::from(meta),
    }
}

/// Map an FTS [`SearchHit`] to an archive list row: the session label + a
/// whitespace-collapsed, length-capped snippet of the matched body, tagged with
/// the hit kind (question · answer · utterance).
fn hit_to_row(h: &SearchHit) -> ArchiveRow {
    let label = pretty_session_label(&h.session_id);
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
        out.push_str(&format!(
            "# {} {}\n\n",
            status_glyph(&s.status),
            pretty_session_label(&s.id)
        ));
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
    for u in utterances {
        let label = if u.source == "mic" { "Mic" } else { "System" };
        out.push_str(&format!("{label}: {}\n\n", u.text.trim()));
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
    if utterances.is_empty() && ai_turns.is_empty() {
        out.push_str("—\n");
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
            started_at_ms: Some(1),
            finished_at_ms: Some(2),
            status: "completed".into(),
            ai_model: Some("gemma".into()),
            transcript_lines: 12,
            ai_turns_count: 3,
            total_cost_microcents: 0,
            indexed_at_ms: 0,
        }
    }

    #[test]
    fn session_row_uses_plain_counts() {
        let row = session_to_row(&sample_session());
        assert!(row.title.as_str().starts_with("done 2026-06-04 09:30:00"));
        assert!(row.subtitle.as_str().contains("lines 12"));
        assert!(row.subtitle.as_str().contains("ai 3"));
        assert_eq!(row.meta.as_str(), ""); // zero cost → blank meta
    }

    #[test]
    fn session_row_shows_cost_when_nonzero() {
        let mut s = sample_session();
        s.total_cost_microcents = 2_400_000; // $0.024
        let row = session_to_row(&s);
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
        let row = hit_to_row(&h);
        assert!(row.title.as_str().starts_with("search 2026-06-04 09:30:00"));
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
        assert!(md.contains("Mic: hello there"));
        assert!(md.contains("Question: **what is it?**"));
        assert!(md.contains("Answer: an answer."));
    }

    #[test]
    fn session_markdown_empty_is_graceful() {
        let md = build_session_markdown(None, &[], &[]);
        assert_eq!(md, "—\n");
    }
}
