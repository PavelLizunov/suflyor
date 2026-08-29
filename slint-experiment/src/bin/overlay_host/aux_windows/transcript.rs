use super::*;

static DIAR_BUSY: AtomicBool = AtomicBool::new(false);
/// V-1 — same latch for the diarization model download: the install outlives
/// the transcript window, but the per-window `installing-diar-models` property
/// dies with it — a close+reopen would otherwise spawn a second `install_models`
/// racing the first on the same .download/staging/live files. One
/// `try_acquire_busy` pairs with the worker's completion (RAII guard).
static DIAR_INSTALL_BUSY: AtomicBool = AtomicBool::new(false);

/// Format a session-relative offset (ms) as `mm:ss`, or `h:mm:ss` past an hour.
/// `pub(crate)` so `tile_copy::format_transcript_for_copy` reuses the SAME
/// formatter the transcript view uses (ТЗ1) — body unchanged.
pub(in super::super) fn fmt_offset(offset_ms: i64) -> String {
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
pub(in super::super) fn open_transcript(
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
    win.global::<ui::Platform>()
        .set_is_macos(cfg!(target_os = "macos"));
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
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot.borrow_mut() = Some(win);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

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
    fn fmt_offset_mm_ss_and_h_mm_ss() {
        assert_eq!(fmt_offset(0), "00:00");
        assert_eq!(fmt_offset(135_000), "02:15");
        assert_eq!(fmt_offset(3_661_000), "1:01:01");
        assert_eq!(fmt_offset(-5), "00:00"); // negative clamps
    }
}
