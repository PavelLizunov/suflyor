use super::*;

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
pub(in super::super) fn open_archive(
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
    win.global::<ui::Platform>()
        .set_is_macos(cfg!(target_os = "macos"));
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
                #[cfg(target_os = "macos")]
                let _ = slint_replay::native::window::begin_drag(w.window());
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
pub(super) fn spawn_content_tile(
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
pub(super) fn session_title(started_at_ms: Option<i64>, id: &str) -> String {
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
}
