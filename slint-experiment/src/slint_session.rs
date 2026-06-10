//! Slint-side session orchestrator — analog of src-tauri's
//! `runtime::start_session` + `stop_session`.
//!
//! The Phase B2 plan deferred these as "binary-specific" entry-point
//! orchestrators. This module is the Slint binary's implementation —
//! it uses the same overlay-backend primitives (audio::start_capture,
//! stt::spawn, Journal::open_new_session) but mutates SlintRuntime
//! instead of src-tauri's SharedRuntime and emits via SlintEvents
//! instead of TauriEvents.
//!
//! Threading:
//! - `start_session` is called from the Slint UI thread (e.g. timer
//!   chip click handler).
//! - It synchronously sets up state + spawns 3 tokio tasks:
//!     1. Transcript forwarder — drains STT receiver, updates rt +
//!        UI, calls auto-detector.
//!     2. Health emitter — 2s ticker that snapshots HealthSignals
//!        atomics + emits `health:update`.
//!     3. (Auto-tile detector spawned per transcript line from #1.)
//! - Tasks store their JoinHandles in SlintRuntime for cancellation
//!   on stop_session / restart.
//!
//! All `events.emit("channel", payload)` calls route through the
//! SlintEvents adapter back to UI property setters via
//! `slint::invoke_from_event_loop`.

use crate::runtime_state::{lock, push_transcript_line, SharedSlintRuntime};
use anyhow::{Context, Result};
use overlay_backend::audio::{self, AudioChunk, AudioSource, TranscriptLine};
use overlay_backend::config::SharedConfig;
use overlay_backend::events::{MonitorHint, RuntimeEvents, TileKind, TileSpec};
use overlay_backend::journal::{now_unix_ms, Journal, JournalEvent};
use overlay_backend::stt;
use overlay_backend::{ai, runtime as backend_runtime};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Start an audio→STT→transcript session. Drops any prior session
/// first (aborts old tasks + clears state).
///
/// Wire from the Slint binary's session-start trigger (timer chip
/// click, hotkey, or auto-on-launch). Caller MUST be inside a tokio
/// runtime context (the function spawns tasks via tokio::spawn).
///
/// # Errors
/// Returns Err if cfg has empty `groq_api_key` (STT can't run) or
/// `audio::start_capture` fails (WASAPI device problem).
pub fn start_session(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    rt: SharedSlintRuntime,
) -> Result<()> {
    start_session_inner(events, cfg, rt, None)
}

/// Memory Phase 1 (crash recovery) entry point. Identical to
/// [`start_session`] except the new session's `SessionStart` event records
/// `recovered_from` (the unfinished session's `session_id`) so the two
/// journals are linked on disk. The recovered context itself is seeded by
/// the caller into `cfg.meeting_context` BEFORE this call (so STT's whisper
/// prompt + every AI ask pick it up via the live config), which is why this
/// function takes no transcript payload — only the link id.
///
/// # Errors
/// Same as [`start_session`].
pub fn start_session_with_recovery(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    rt: SharedSlintRuntime,
    recovered_from: String,
) -> Result<()> {
    start_session_inner(events, cfg, rt, Some(recovered_from))
}

fn start_session_inner(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    rt: SharedSlintRuntime,
    recovered_from: Option<String>,
) -> Result<()> {
    // ===== 1. Stop any prior session + reset state =====
    {
        let mut s = lock(&rt);
        s.capture = None; // Drop signals capture thread to stop.
        s.transcript.clear();
        // v0.12.0 — the Summary accumulator resets at session START (not
        // stop) so the Summary button keeps working between Стоп and the
        // next Старт.
        s.full_transcript.clear();
        s.full_transcript_truncated = false;
        s.session_cost_microcents = 0;
        // New session generation — invalidates any in-flight auto-tile task
        // from a prior session (it bails post-AI-call on the gen mismatch).
        s.session_gen = s.session_gen.wrapping_add(1);
        if let Some(h) = s.transcript_task.take() {
            h.abort();
        }
        if let Some(h) = s.ai_task.take() {
            h.abort();
        }
        if let Some(h) = s.health_task.take() {
            h.abort();
        }
        // Reset health atomics so first emit shows "idle" not stale-down.
        s.health.last_audio_frame_ms.store(0, Ordering::Relaxed);
        s.health.last_stt_ok_ms.store(0, Ordering::Relaxed);
        s.health.last_ai_ok_ms.store(0, Ordering::Relaxed);
        // V0.8.0 (Поток A) — also clear the AI-error marker + the error-tile
        // debounce, else a session that ended on an AI failure would open the
        // NEXT session already showing "AI недоступен" (stale err >= ok=0).
        s.health.last_ai_err_ms.store(0, Ordering::Relaxed);
        s.last_ai_error_tile_ms = 0;
        s.speech_window.clear();
        s.meeting_ending_emitted = false;
        s.recent_question_prefixes.clear();
        s.qa_cache.clear();
        if let Some(j) = s.journal.take() {
            // Drop the prior session's journal — drops the Arc<mpsc::Tx>
            // which closes the writer task gracefully. (src-tauri side
            // has close_journal_with_summary for the SessionSummary
            // event; we replicate that pattern in stop_session, not here.)
            drop(j);
        }
    }

    // Tell the UI cost is back to zero (chips depending on session_usd
    // get a chance to reset). Pre-port React side did the same.
    events.emit("cost:update", serde_json::json!({ "session_usd": 0.0_f64 }));

    // ===== 2. Read cfg fields needed for capture + STT =====
    let (mic_dev, sys_dev, stt_backend, stt_is_local, groq_key, language, whisper_prompt) = {
        let c = cfg.read();
        (
            c.mic_device.clone(),
            c.system_audio_device.clone(),
            c.stt_backend(),
            c.stt_is_local(),
            c.groq_api_key.clone(),
            c.stt_language.clone(),
            stt::build_whisper_prompt(&c.trigger_keywords, &c.meeting_context),
        )
    };
    // Only the cloud (Groq) backend needs a key — local GigaAM/Whisper don't.
    if !stt_is_local && groq_key.trim().is_empty() {
        anyhow::bail!("Groq API key not set in settings (cfg.groq_api_key empty)");
    }
    // Local GigaAM: validate the model dir loads up front so we fail fast with a
    // clear message instead of silently producing no transcripts once capture
    // starts (mirrors the cloud-key bail above). One-shot blocking load (~0.5s).
    if let overlay_backend::config::SttBackendCfg::Gigaam { model_dir } = &stt_backend {
        if let Err(e) = stt::validate_gigaam_dir(model_dir) {
            anyhow::bail!("GigaAM model dir invalid: {e}");
        }
    }
    // Phase E6 diagnostic — surface device names so we can debug
    // mic-transcript-not-working complaints. Empty = "default device".
    log_info(&format!(
        "audio devices — mic={:?} sys={:?}",
        mic_dev.as_deref().unwrap_or("<default>"),
        sys_dev.as_deref().unwrap_or("<default>"),
    ));
    // Log the RESOLVED backend (not just cfg.stt_model, which is the Groq model
    // and is meaningless for the local engines). model= still shown for the
    // Whisper-family backends where it's the actual model id.
    let backend_desc = match &stt_backend {
        overlay_backend::config::SttBackendCfg::Cloud { model, .. } => {
            format!("cloud-groq (model={model})")
        }
        overlay_backend::config::SttBackendCfg::Whisper {
            base_url, model, ..
        } => format!("local-whisper (url={base_url} model={model})"),
        overlay_backend::config::SttBackendCfg::Gigaam { model_dir } => {
            format!("local-gigaam (dir={model_dir})")
        }
    };
    log_info(&format!(
        "stt config — backend={backend_desc} language={:?} whisper_prompt={}",
        language.as_deref().unwrap_or("<auto>"),
        if cfg.read().trigger_keywords.is_empty() {
            "<no kw prompt>"
        } else {
            "<from trigger_keywords>"
        }
    ));

    // ===== 3. Open fresh journal =====
    let journal = match Journal::open_new_session() {
        Ok(j) => j,
        Err(e) => {
            log_warn(&format!("journal open failed: {e:#}"));
            Journal::default()
        }
    };
    {
        let c = cfg.read();
        journal.write(&JournalEvent::SessionStart {
            unix_ms: now_unix_ms(),
            meeting_context_chars: c.meeting_context.len(),
            ai_model: &c.ai_model,
            prep_model: &c.prep_model,
            stt_language: c.stt_language.as_deref(),
            response_language: &c.response_language,
            recovered_from_session_id: recovered_from.as_deref(),
        });
    }
    lock(&rt).journal = Some(journal.clone());

    // ===== 4. Spawn audio capture =====
    let (audio_rx, capture_handle) = audio::start_capture(mic_dev, sys_dev)
        .context("audio::start_capture failed (check mic / system audio devices in Settings)")?;
    let health = lock(&rt).health.clone();

    // ===== 4b. Optional raw-audio recorder (v0.13.0) =====
    // When recording is enabled, tee the AudioChunk stream to per-channel WAVs
    // via a forwarding task that ALSO feeds the recorder. The recorder.feed is
    // non-blocking (drops on overflow), and forwarding to STT keeps the SAME
    // bounded(128) back-pressure as the direct path — so recording never slows
    // transcription. The recorder is MOVED into the task and dropped (WAVs
    // finalised) when the stream ends on session stop. When disabled, audio_rx
    // flows straight into STT with zero overhead.
    let stt_audio_rx = if cfg.read().record_audio_enabled {
        let keep_sessions = cfg.read().record_retention_sessions as usize;
        let session_id = journal
            .current_path()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| format!("session_{}", now_unix_ms()));
        match overlay_backend::recorder::SessionRecorder::start(&session_id, keep_sessions) {
            Ok(recorder) => {
                log_info(&format!("audio recording → {}", recorder.dir().display()));
                let (stt_tx, stt_rx2) = tokio::sync::mpsc::channel::<AudioChunk>(128);
                let mut src_rx = audio_rx;
                let rt_for_rec = rt.clone();
                // Intentionally NOT stored as an abort-tracked task: it
                // self-terminates when src_rx closes (stop_session drops the
                // CaptureHandle → WASAPI threads exit → senders dropped), and the
                // recorder MUST finalise its WAVs via Drop rather than be aborted
                // mid-write. The retention prune's grace window keeps a rapid
                // restart from racing a still-finalising prior dir.
                tokio::spawn(async move {
                    // `recorder` is owned here → it is finalised when this task
                    // ends, i.e. when capture stops and src_rx closes.
                    let recorder = recorder;
                    while let Some(chunk) = src_rx.recv().await {
                        // v0.13.1 — when the mic chip is muted, do NOT write mic
                        // audio to the recording (system audio still records). The
                        // transcript forwarder drops mic transcript lines on the
                        // same rt.mic_muted flag, so one toggle stops both.
                        let mic_muted =
                            matches!(chunk.source, AudioSource::Mic) && lock(&rt_for_rec).mic_muted;
                        if !mic_muted {
                            recorder.feed(&chunk);
                        }
                        if stt_tx.send(chunk).await.is_err() {
                            break; // STT pipeline gone — stop teeing
                        }
                    }
                    // Finalise the WAVs on the BLOCKING pool: the recorder's Drop
                    // join()s its writer std-thread (real disk I/O), and the
                    // runtime has only 2 worker threads — dropping it inline would
                    // park a scarce async worker for the flush, and a stacked
                    // teardown on rapid restart could park both. spawn_blocking
                    // moves the join off the async workers (review v0.13.0). The
                    // handle is detached (dropped) — the blocking finalise still
                    // runs to completion.
                    std::mem::drop(tokio::task::spawn_blocking(move || drop(recorder)));
                });
                stt_rx2
            }
            Err(e) => {
                // Recording is best-effort: a failure must NOT abort the session.
                log_info(&format!(
                    "audio recording unavailable (continuing without it): {e:#}"
                ));
                audio_rx
            }
        }
    } else {
        audio_rx
    };

    // ===== 5. Spawn STT pipeline =====
    let stt_rx = stt::spawn(
        stt_audio_rx,
        stt_backend,
        language,
        whisper_prompt,
        health.clone(),
    );
    lock(&rt).capture = Some(capture_handle);

    // ===== 6. Spawn health emitter (2s ticker) =====
    let health_for_tick = health.clone();
    let events_for_tick = events.clone();
    let health_task = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let now_ms = now_unix_ms() as u64;
            let snap = health_for_tick.snapshot(now_ms);
            let payload = serde_json::to_value(&snap).unwrap_or(serde_json::Value::Null);
            events_for_tick.emit("health:update", payload);
            // (speech:coach emit lives in the snapshot_speech_coach
            // helper — Slint binary will wire it in E2 follow-up
            // alongside the speech-coach state migration.)
        }
    });
    lock(&rt).health_task = Some(health_task);

    // ===== 7. Spawn transcript forwarder =====
    let events_for_fwd = events.clone();
    let rt_for_fwd = rt.clone();
    let journal_for_fwd = journal.clone();
    let cfg_for_fwd = cfg.clone();
    let forwarder = tokio::spawn(transcript_forwarder(
        stt_rx,
        events_for_fwd,
        cfg_for_fwd,
        rt_for_fwd,
        journal_for_fwd,
    ));
    lock(&rt).transcript_task = Some(forwarder);

    // ===== 8. Signal session start =====
    events.emit(
        "session:started",
        serde_json::json!({ "unix_ms": now_unix_ms() }),
    );

    Ok(())
}

/// Transcript forwarder task body. Reads STT events, pushes to
/// rt.transcript (with 80-line cap), writes journal, emits
/// transcript:line to UI, runs meeting-ending detection, and
/// (Phase E4) invokes the auto-tile detector pipeline.
async fn transcript_forwarder(
    mut stt_rx: tokio::sync::mpsc::Receiver<stt::TranscriptEvent>,
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    rt: SharedSlintRuntime,
    journal: Journal,
) {
    while let Some(ev) = stt_rx.recv().await {
        // Phase E6 diagnostic — log every STT event so we can debug
        // "mic transcript not working" complaints. Truncate text to
        // 80 chars to keep stderr readable.
        log_info(&format!(
            "transcript event: source={:?} text='{}'",
            ev.source,
            ev.text.chars().take(80).collect::<String>(),
        ));
        // Mic-mute drop — same semantic as src-tauri's check.
        if matches!(ev.source, AudioSource::Mic) && lock(&rt).mic_muted {
            log_info("  -> dropped (mic muted)");
            continue;
        }
        let line = TranscriptLine {
            source: ev.source,
            text: ev.text.clone(),
            timestamp_ms: ev.timestamp_ms,
        };
        {
            let mut s = lock(&rt);
            push_transcript_line(&mut s, line.clone());
        }
        journal.write(&JournalEvent::TranscriptLine {
            unix_ms: now_unix_ms(),
            source: match line.source {
                AudioSource::System => "system",
                AudioSource::Mic => "mic",
            },
            text: &line.text,
        });
        let payload = serde_json::to_value(&line).unwrap_or(serde_json::Value::Null);
        events.emit("transcript:line", payload);

        // Meeting-ending detector (system audio only — pre-port
        // semantic). Emit `meeting:ending` exactly once per session.
        if line.source == AudioSource::System && meeting_ending_phrase_match(&line.text) {
            let mut s = lock(&rt);
            if !s.meeting_ending_emitted {
                s.meeting_ending_emitted = true;
                drop(s);
                events.emit("meeting:ending", serde_json::Value::Null);
            }
        }

        // Phase E4 — auto-tile detector. detector_allows mirrors
        // src-tauri's "skip mic when configured" rule so the user can
        // disable self-triggered tiles. The actual ask runs as a
        // tokio task so it doesn't block the forwarder loop.
        let skip_mic = cfg.read().detector_skip_mic;
        if detector_allows(line.source, skip_mic) {
            let events_for_tile = events.clone();
            let cfg_for_tile = cfg.clone();
            let rt_for_tile = rt.clone();
            let journal_for_tile = journal.clone();
            let line_text = line.text.clone();
            // Stamp the task with the current session generation so it can
            // detect a stop/restart that happens during its AI call.
            let gen_for_tile = lock(&rt).session_gen;
            tokio::spawn(async move {
                maybe_spawn_auto_tile(
                    events_for_tile,
                    cfg_for_tile,
                    rt_for_tile,
                    journal_for_tile,
                    line_text,
                    gen_for_tile,
                )
                .await;
            });
        }
    }
    log_info("transcript forwarder exit");
}

/// Auto-detector gate — same matrix as src-tauri's `detector_allows`.
/// Returns true if the forwarder should call `maybe_spawn_auto_tile`
/// for this source. `skip_mic=true` means user-side speech does NOT
/// trigger tiles (useful when the user gives long monologues that
/// shouldn't waste AI calls).
#[must_use]
pub fn detector_allows(source: AudioSource, skip_mic: bool) -> bool {
    match source {
        AudioSource::System => true,
        AudioSource::Mic => !skip_mic,
    }
}

/// Phase E6 v12 — convert a detector Trigger into TileSpec.highlights.
/// First slot is a human-readable trigger label rendered as a colored
/// badge in the tile (e.g. "keyword docker", "question snippet"). Empty Vec for
/// no badge (manual F9 / F6 spawns don't go through this helper).
#[must_use]
pub fn trigger_highlights(trigger: &backend_runtime::Trigger) -> Vec<String> {
    match trigger {
        backend_runtime::Trigger::Keyword(kw, _) => vec![format!("keyword {kw}")],
        backend_runtime::Trigger::Question(q) => {
            let snippet: String = q.trim().chars().take(60).collect();
            vec![format!("question {snippet}")]
        }
    }
}

/// Auto-tile rate limit — drop spawn if more than this many tiles
/// fired in the rolling 60s window. Matches src-tauri's MAX_TILES_
/// PER_MIN value to keep cost behavior identical across binaries.
const MAX_TILES_PER_MIN: usize = 6;
/// Aggressive-mode cap (when `cfg.auto_tile_every_line=true`).
/// Phase E6 v19: reduced 20 → 10 after user reported UI freeze even
/// with cycle 22 spawn rate limit. 20/min still floods the spawn
/// queue + each tile holds AI streaming state + memory. 10/min ≈ one
/// tile every 6s which matches realistic interview pacing.
const MAX_TILES_PER_MIN_AGGRESSIVE: usize = 10;
/// QA cache TTL — 10 min matches src-tauri's qa_cache TTL so a
/// long meeting that re-asks the same question gets a cache hit.
const QA_CACHE_TTL_SECS: u64 = 600;
/// QA cache hard cap before half-eviction (matches src-tauri).
const QA_CACHE_MAX_ENTRIES: usize = 256;
/// V0.8.0 (Поток A) — min interval between auto-tile AI-error notice tiles.
/// During a sustained outage the detector fires per transcript line; we spawn
/// at most one "AI недоступен" tile per this window so the user is informed
/// once, not spammed. 20s balances "noticed promptly" vs "not nagging".
const AI_ERROR_TILE_DEBOUNCE_MS: u64 = 20_000;

/// Phase E4 — Slint-side auto-tile detector + AI ask pipeline.
///
/// Faithful port of src-tauri's `maybe_spawn_tile` (deferred from
/// Phase B2 ports #7/#8 as binary-specific orchestrator). Same
/// guardrails: cfg-disable / no-bearer bail, detector trigger,
/// rate-limit, dedup, QA cache, AI ask, tile spawn.
///
/// Differences from src-tauri version:
/// - Spawns the tile via `events.spawn_tile_full` (trait) instead of
///   direct `tile::spawn_tile_with_stealth`. Adapter routes to the
///   Slint binary's `SpawnTileRequest` channel + poll Timer.
/// - Does NOT yet integrate KB lookup (src-tauri `kb::search` is
///   in overlay-backend and could be wired in a follow-up — for now
///   the prompt skips the KB-context-addon block).
async fn maybe_spawn_auto_tile(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    rt: SharedSlintRuntime,
    journal: Journal,
    text: String,
    session_gen: u64,
) {
    let (
        enabled,
        every_line,
        trigger_keywords,
        base_url,
        bearer,
        model,
        response_language,
        meeting_context,
        cap_usd,
        preferred_monitor,
        stealth,
        is_local,
    ) = {
        let c = cfg.read();
        let ep = c.ai_endpoint(false);
        (
            c.auto_tiles_enabled,
            c.auto_tile_every_line,
            c.trigger_keywords.clone(),
            ep.base_url,
            ep.bearer,
            ep.model,
            c.response_language.clone(),
            c.meeting_context.clone(),
            c.max_session_cost_usd,
            c.tile_monitor_name.clone(),
            c.stealth_enabled,
            ep.is_local,
        )
    };
    // A bearer is required only for the CLOUD bridge; local servers
    // (llama.cpp / Ollama) need no auth, so an empty bearer is valid there —
    // gating on it blocked ALL auto-spawn when the local provider was active.
    if !enabled || (!is_local && bearer.trim().is_empty()) {
        return;
    }

    // ===== Detector trigger =====
    let detected = if every_line {
        if text.trim().chars().count() < 5 {
            None
        } else {
            Some(backend_runtime::Trigger::Question(text.clone()))
        }
    } else {
        backend_runtime::detect_trigger(&text, &trigger_keywords)
    };
    let (triggered, trigger_kind): (bool, Option<String>) = match &detected {
        Some(backend_runtime::Trigger::Question(_)) if every_line => {
            (true, Some("every_line".into()))
        }
        Some(backend_runtime::Trigger::Question(_)) => (true, Some("question".into())),
        Some(backend_runtime::Trigger::Keyword(kw, _)) => (true, Some(format!("keyword:{kw}"))),
        None => (false, None),
    };
    journal.write(&JournalEvent::DetectorDecision {
        unix_ms: now_unix_ms(),
        text: &text,
        triggered,
        trigger_kind: trigger_kind.as_deref(),
    });
    let Some(trigger) = detected else { return };

    // ===== Rate-limit =====
    let cap = if every_line {
        MAX_TILES_PER_MIN_AGGRESSIVE
    } else {
        MAX_TILES_PER_MIN
    };
    {
        let mut s = lock(&rt);
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(60);
        while let Some(front) = s.recent_tile_triggers.front() {
            if *front < cutoff {
                s.recent_tile_triggers.pop_front();
            } else {
                break;
            }
        }
        if s.recent_tile_triggers.len() >= cap {
            log_info(&format!(
                "tile rate-limit hit ({}/{cap} in last 60s) — dropping trigger",
                s.recent_tile_triggers.len()
            ));
            journal.write(&JournalEvent::RateLimited {
                unix_ms: now_unix_ms(),
                what: "auto_tile",
                text: &text,
            });
            drop(s);
            events.emit("tile:rate-limited", serde_json::json!({ "text": text }));
            return;
        }
        s.recent_tile_triggers.push_back(now);
    }

    // ===== Dedup recently-spawned prefixes =====
    {
        let normalized: String = text
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(60)
            .collect();
        let mut s = lock(&rt);
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(60);
        s.recent_question_prefixes.retain(|(_, ts)| *ts > cutoff);
        if s.recent_question_prefixes
            .iter()
            .any(|(prefix, _)| prefix == &normalized)
        {
            log_info(&format!("tile dedup: skipping prefix '{normalized}'"));
            return;
        }
        s.recent_question_prefixes.push((normalized, now));
    }

    // ===== QA cache key + lookup =====
    // Audit (prompt-context): compute the EFFECTIVE context (profile + approved
    // memory) ONCE here, hash IT (not the raw meeting_context) so approving/editing/
    // deleting memory invalidates a stale cached answer, and reuse it for the prompt.
    let effective_context = overlay_backend::memory::context_for_meeting(&meeting_context);
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    effective_context.hash(&mut h);
    let ctx_hash = h.finish();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    trigger_keywords.hash(&mut h2);
    let kw_hash = h2.finish();
    let cache_key: String = format!(
        "m={model};l={response_language};c={ctx_hash:x};k={kw_hash:x};q={}",
        text.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect::<String>(),
    );
    let cache_hit: Option<String> = {
        let mut s = lock(&rt);
        let now = Instant::now();
        let ttl = Duration::from_secs(QA_CACHE_TTL_SECS);
        s.qa_cache
            .retain(|_, (_, ts)| now.duration_since(*ts) < ttl);
        s.qa_cache.get(&cache_key).map(|(a, _)| a.clone())
    };

    if let Some(cached_answer) = cache_hit {
        log_info(&format!(
            "qa_cache HIT (avoided AI call): {}",
            text.chars().take(60).collect::<String>()
        ));
        let trigger_text_for_q = match &trigger {
            backend_runtime::Trigger::Question(q) => q.clone(),
            backend_runtime::Trigger::Keyword(kw, _) => kw.clone(),
        };
        // Phase E6 v12 — populate highlights with the trigger keyword
        // (or "❓" + question prefix) so the tile UI can show a badge
        // explaining why this tile spawned. User: "ключевые слова не
        // помечены я не понимаю на какое окно смотреть".
        let highlights_for_spec = trigger_highlights(&trigger);
        {
            let mut s = lock(&rt);
            s.last_question = Some(trigger_text_for_q.clone());
            s.last_answer = Some(cached_answer.clone());
        }
        let monitor_hint = match preferred_monitor.as_deref() {
            Some(name) if !name.is_empty() => MonitorHint::Named(name.to_string()),
            _ => MonitorHint::Auto,
        };
        let _ = events.spawn_tile_full(
            TileSpec {
                question: trigger_text_for_q,
                answer: cached_answer,
                source: "auto_tile_cached".into(),
                is_translation: false,
                highlights: highlights_for_spec,
            },
            monitor_hint,
            stealth,
            TileKind::Auto,
        );
        return;
    }

    log_info(&format!("auto-tile triggered: {trigger:?}"));

    // ===== Cost cap — BLOCK the auto-tile cloud spend once over budget =====
    // Local inference is free (cost stays 0), so this only ever trips on the
    // cloud bridge. Auto-tiles are the "spend without an explicit keypress"
    // path, so honour the user's cap here by returning after the chip — manual
    // F9/F6 asks still proceed (that path deliberately only warns). Audit #18:
    // the cap previously warned but proceeded, so it never actually capped.
    let current_micro = lock(&rt).session_cost_microcents;
    if cap_usd > 0.0 {
        let current_usd = (current_micro as f64) / 100_000_000.0;
        if current_usd >= cap_usd {
            events.emit(
                "cost:cap-hit",
                serde_json::json!({
                    "reason": format!(
                        "over budget: ${current_usd:.4} spent ≥ ${cap_usd:.2} (Settings → Max cost per session)"
                    ),
                    "source": "auto_tile",
                    "blocking": true,
                }),
            );
            return;
        }
    }

    // ===== Recent transcript context (last 5 labeled lines) =====
    let recent_transcript: Vec<String> = {
        let s = lock(&rt);
        s.transcript
            .iter()
            .rev()
            .take(5)
            .rev()
            .map(|l| {
                let src = match l.source {
                    AudioSource::System => "СОБЕСЕДНИК",
                    AudioSource::Mic => "ПОЛЬЗОВАТЕЛЬ",
                };
                format!("[{src}] {}", l.text)
            })
            .collect()
    };

    // ===== Build prompts + AI call =====
    let trigger_text = match &trigger {
        backend_runtime::Trigger::Question(q) => q.clone(),
        backend_runtime::Trigger::Keyword(kw, _) => kw.clone(),
    };
    let (system_prompt, prompt) = backend_runtime::build_auto_tile_prompts(
        &trigger,
        &recent_transcript,
        // Phase 3b.4 — fold the user's APPROVED memory into the background block.
        // Audit (prompt-context): reuse the SAME effective context hashed for the
        // cache key above (computed once; off the audio thread).
        &effective_context,
        &response_language,
    );
    let sys_full = system_prompt.clone();
    let usr_full = prompt.clone();
    let input_tokens_est = ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4;
    let messages = vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(system_prompt),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(prompt),
        },
    ];
    journal.write(&JournalEvent::AiRequest {
        unix_ms: now_unix_ms(),
        purpose: "auto_tile",
        model: &model,
        system_prompt: &sys_full,
        user_prompt: &usr_full,
        attached_screenshot: false,
        input_tokens_est,
    });

    let t0 = Instant::now();
    let (answer, usage) = match ai::complete_with_usage(&base_url, &bearer, &model, messages, 512)
        .await
    {
        Ok((t, u)) => {
            lock(&rt)
                .health
                .last_ai_ok_ms
                .store(now_unix_ms() as u64, Ordering::Relaxed);
            (t.trim().to_string(), u)
        }
        Err(e) => {
            // V0.8.0 (Поток A) — the auto-tile AI call failed. Previously
            // this returned SILENTLY (log + journal only), so the user saw
            // auto-tiles simply stop with no explanation ("почему тайлы
            // перестали появляться"). Now we (1) mark AI down so the bar
            // flips to "AI недоступен" within one 2s health tick, and
            // (2) spawn ONE debounced, sanitized error tile.
            let chain = format!("{e:#}");
            log_warn(&format!("auto-tile AI failed: {chain}"));
            journal.write(&JournalEvent::Error {
                unix_ms: now_unix_ms(),
                module: "auto_tile_ai",
                message: &chain,
            });
            // v0.8.2 (M1 fix) — mirror the success path's session_gen guard
            // (the `if … != session_gen` check below). If the session was
            // stopped/restarted DURING this (up to ~9-min, with retries)
            // failing call, do NOT poison the NEXT session: skip marking AI
            // down (would flip the new session's bar to "AI недоступен" until
            // its first success) and skip spawning a stray error tile into it.
            // The log + journal above stay unconditional (local diagnostics).
            if lock(&rt).session_gen != session_gen {
                log_info("auto-tile: session changed during failed AI call — not surfacing error");
                return;
            }
            // Mark AI down immediately (snapshot returns "down" while this
            // err is newer than the last ok; auto-clears on next success).
            lock(&rt)
                .health
                .last_ai_err_ms
                .store(now_unix_ms() as u64, Ordering::Relaxed);
            // Debounce: at most one error tile per AI_ERROR_TILE_DEBOUNCE_MS,
            // so a sustained outage (detector fires per line) doesn't spam.
            let now = now_unix_ms() as u64;
            let should_notify = {
                let mut s = lock(&rt);
                if now.saturating_sub(s.last_ai_error_tile_ms) >= AI_ERROR_TILE_DEBOUNCE_MS {
                    s.last_ai_error_tile_ms = now;
                    true
                } else {
                    false
                }
            };
            if should_notify {
                // Generic message — NEVER the raw chain (it carries the local
                // server's LAN IP / base_url; the tile is screen-shared).
                let reason = crate::app_state::classify_ai_error(&chain);
                let _ = events.spawn_tile_full(
                        TileSpec {
                            question: "AI недоступен".into(),
                            answer: format!(
                                "**Не получаю ответ от AI:** {reason}\n\nАвто-подсказки приостановлены, пока AI не ответит. Проверьте локальный AI-сервер или AI-мост (Настройки -> AI). Можно перезапустить приложение кнопкой restart на панели."
                            ),
                            source: "ai_error".into(),
                            is_translation: false,
                            highlights: vec!["AI error".into()],
                        },
                        MonitorHint::Auto,
                        stealth,
                        TileKind::Error,
                    );
            }
            return;
        }
    };
    let latency_ms = t0.elapsed().as_millis() as u64;

    // E9 — if the session was stopped/restarted during this (up to
    // ~9-minute, with retries) AI call, abandon the result: don't cache,
    // bill, or spawn a tile into a session that no longer exists.
    if lock(&rt).session_gen != session_gen {
        log_info("auto-tile: session changed during AI call — discarding result");
        return;
    }

    // ===== Cache the answer =====
    {
        let mut s = lock(&rt);
        if s.qa_cache.len() >= QA_CACHE_MAX_ENTRIES {
            let now = Instant::now();
            let mut by_age: Vec<(String, Duration)> = s
                .qa_cache
                .iter()
                .map(|(k, (_, ts))| (k.clone(), now.duration_since(*ts)))
                .collect();
            by_age.sort_by_key(|(_, age)| std::cmp::Reverse(*age));
            for (k, _) in by_age.into_iter().take(QA_CACHE_MAX_ENTRIES / 2) {
                s.qa_cache.remove(&k);
            }
        }
        s.qa_cache
            .insert(cache_key, (answer.clone(), Instant::now()));
    }

    // ===== Cost accumulate + emit =====
    // Local inference is free — don't bill it.
    let micro = if is_local {
        0
    } else {
        ai::cost_microcents(&model, usage.input, usage.output)
    };
    let total_usd = {
        let mut s = lock(&rt);
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    };
    events.emit(
        "cost:update",
        serde_json::json!({ "session_usd": total_usd }),
    );

    journal.write(&JournalEvent::AiResponse {
        unix_ms: now_unix_ms(),
        purpose: "auto_tile",
        model: &model,
        latency_ms,
        finish_reason: "stop",
        text: &answer,
        output_tokens_est: usage.output,
        cost_microcents: micro,
    });

    if answer.is_empty() {
        return;
    }

    // ===== Spawn auto-tile =====
    let question_label = match &trigger {
        backend_runtime::Trigger::Question(q) => q.clone(),
        // De-emojified to match the rest of the UI (Codex icon pass): the
        // keyword itself is the tile title / stored last_question, no glyph.
        backend_runtime::Trigger::Keyword(kw, _) => kw.clone(),
    };
    {
        let mut s = lock(&rt);
        s.last_question = Some(question_label.clone());
        s.last_answer = Some(answer.clone());
    }
    let monitor_hint = match preferred_monitor.as_deref() {
        Some(name) if !name.is_empty() => MonitorHint::Named(name.to_string()),
        _ => MonitorHint::Auto,
    };
    let label_for_log = question_label.clone();
    let answer_for_journal = answer.clone();
    // Phase E6 v12 — pass trigger info so the tile shows a badge
    // explaining which keyword/question spawned it.
    let highlights_for_spec = trigger_highlights(&trigger);
    match events.spawn_tile_full(
        TileSpec {
            question: question_label.clone(),
            answer,
            source: "auto_tile".into(),
            is_translation: false,
            highlights: highlights_for_spec,
        },
        monitor_hint,
        stealth,
        TileKind::Auto,
    ) {
        Ok(label) => {
            journal.write(&JournalEvent::TileSpawn {
                unix_ms: now_unix_ms(),
                label: &label,
                question: &label_for_log,
                answer: &answer_for_journal,
            });
        }
        Err(e) => log_warn(&format!("auto-tile spawn failed: {e}")),
    }
    let _ = trigger_text;
}

/// Stop the active session: aborts all spawned tasks, drops the
/// capture handle, closes the journal. Returns the transcript
/// snapshot so the caller can pass it to `run_post_meeting_debrief`.
///
/// Phase E5 wiring: caller is expected to invoke `maybe_run_debrief`
/// with the returned snapshot — that helper checks the debrief gate
/// (cfg opt-in + ≥30s session + ≥5 mic lines + non-empty AI bearer)
/// and fires `overlay_backend::runtime::run_post_meeting_debrief`.
pub fn stop_session(rt: SharedSlintRuntime) -> Vec<TranscriptLine> {
    let mut s = lock(&rt);
    s.capture = None;
    // Bump generation so an in-flight auto-tile call from this session
    // discards its result instead of spawning a tile after the stop.
    s.session_gen = s.session_gen.wrapping_add(1);
    if let Some(h) = s.transcript_task.take() {
        h.abort();
    }
    if let Some(h) = s.ai_task.take() {
        h.abort();
    }
    if let Some(h) = s.health_task.take() {
        h.abort();
    }
    s.health.last_audio_frame_ms.store(0, Ordering::Relaxed);
    s.health.last_stt_ok_ms.store(0, Ordering::Relaxed);
    s.health.last_ai_ok_ms.store(0, Ordering::Relaxed);
    // v0.8.2 (N2) — symmetry with start_session (which resets all five): also
    // clear the AI-error marker + error-tile debounce on stop so health reads
    // clean between sessions (defense-in-depth alongside the M1 gen-guard).
    s.health.last_ai_err_ms.store(0, Ordering::Relaxed);
    s.last_ai_error_tile_ms = 0;
    let snapshot: Vec<TranscriptLine> = s.transcript.iter().cloned().collect();
    s.transcript.clear();
    // v0.12.0 — deliberately NOT clearing s.full_transcript here: the
    // Summary button must still work after Стоп (it resets on the next
    // Старт in start_session_inner).
    // Write the SessionSummary roll-up + SessionStop marker before closing, so
    // the journal has the "how did this session go" one-liner on disk (audit:
    // these were defined + counted but never emitted on the shipping stack).
    if let Some(j) = s.journal.take() {
        if let Some(c) = j.snapshot_counters() {
            let now = overlay_backend::journal::now_unix_ms();
            j.write(&overlay_backend::journal::JournalEvent::SessionSummary {
                unix_ms: now,
                duration_ms: now.saturating_sub(c.start_unix_ms),
                transcript_lines: c.transcript_mic + c.transcript_system,
                transcript_mic: c.transcript_mic,
                transcript_system: c.transcript_system,
                detector_triggered: c.detector_triggered,
                detector_skipped: c.detector_skipped,
                ai_requests_total: c.ai_requests_total,
                ai_responses_ok: c.ai_responses_ok,
                ai_errors: c.ai_errors,
                tiles_spawned: c.tiles_spawned,
                rate_limited: c.rate_limited,
                total_cost_microcents: c.total_cost_microcents,
            });
            j.write(&overlay_backend::journal::JournalEvent::SessionStop { unix_ms: now });
        }
        drop(j);
    }
    snapshot
}

/// Minimum session duration (≥30s) before debrief considered
/// worthwhile. Matches src-tauri's gate. Stops single-question
/// sessions from running an expensive debrief AI call.
const DEBRIEF_MIN_SESSION_MS: u64 = 30_000;
/// Minimum number of mic-source transcript lines before debrief
/// runs (user must have actually spoken).
const DEBRIEF_MIN_MIC_LINES: usize = 5;

/// Phase E5 — gate the debrief call. `Ok(())` means the debrief
/// should fire; `Err(reason)` short-circuits with a log line.
/// Mirrors src-tauri's `should_run_debrief` invariants.
pub fn debrief_gate(
    cfg: &SharedConfig,
    transcript: &[TranscriptLine],
    session_duration_ms: u64,
) -> Result<(), &'static str> {
    let c = cfg.read();
    if !c.post_meeting_debrief_enabled {
        return Err("post-meeting debrief disabled in Settings → 🎯 Coaching");
    }
    if c.ai_bearer.trim().is_empty() {
        return Err("AI bearer empty — no bridge configured");
    }
    if session_duration_ms < DEBRIEF_MIN_SESSION_MS {
        return Err("session too short (<30s)");
    }
    let mic_lines = transcript
        .iter()
        .filter(|l| matches!(l.source, AudioSource::Mic))
        .count();
    if mic_lines < DEBRIEF_MIN_MIC_LINES {
        return Err("not enough mic lines for meaningful debrief");
    }
    Ok(())
}

/// Phase E5 — call the ported `run_post_meeting_debrief` if the gate
/// allows. Fire-and-forget — on AI error the ported fn logs + drops
/// silently (matches src-tauri behavior). Caller should pass the
/// transcript snapshot returned by `stop_session` and the session
/// duration in ms.
pub fn maybe_run_debrief(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    transcript: Vec<TranscriptLine>,
    session_duration_ms: u64,
    rt_handle: &tokio::runtime::Handle,
) {
    match debrief_gate(&cfg, &transcript, session_duration_ms) {
        Ok(()) => {
            log_info(&format!(
                "running post-meeting debrief ({} lines, {}s)",
                transcript.len(),
                session_duration_ms / 1000
            ));
            rt_handle.spawn(async move {
                overlay_backend::runtime::run_post_meeting_debrief(events, cfg, transcript).await;
            });
        }
        Err(reason) => {
            log_info(&format!("post-meeting debrief skipped: {reason}"));
        }
    }
}

/// Local meeting-ending phrase detector — same patterns as src-tauri's
/// `meeting_ending_phrase_match`. Duplicated until/unless we move it
/// to overlay-backend in a future cleanup phase. Conservative tuning
/// (multi-word patterns only — no false positives on plain "thanks").
#[must_use]
pub fn meeting_ending_phrase_match(text: &str) -> bool {
    let s = text.to_lowercase();
    let patterns_en = [
        "thanks for your time",
        "thank you for your time",
        "we'll be in touch",
        "we will be in touch",
        "we'll get back to you",
        "we will get back to you",
        "appreciate your time",
        "any final questions",
        "any questions for us",
        "that's all from my side",
        "that wraps it up",
        "let's wrap up",
        "let's call it",
        "have a great rest of your day",
    ];
    let patterns_ru = [
        "спасибо за уделённое время",
        "спасибо за уделенное время",
        "приятно было пообщаться",
        "приятно было поговорить",
        "приятно было познакомиться",
        "будем на связи",
        "свяжемся с вами",
        "ответим в течение",
        "у вас есть вопросы к нам",
        "есть вопросы ко мне",
        "на этом завершим",
        "давайте подытожим",
        "хорошего дня",
        "всего доброго",
    ];
    for p in patterns_en.iter().chain(patterns_ru.iter()) {
        if s.contains(p) {
            return true;
        }
    }
    false
}

// Tiny log shims — slint-experiment doesn't depend on the `log`
// crate. Match the existing eprintln pattern used elsewhere in
// the binary.
fn log_warn(msg: &str) {
    eprintln!("[slint-session] WARN: {msg}");
}
fn log_info(msg: &str) {
    eprintln!("[slint-session] INFO: {msg}");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests need brevity for Result/Option assertions; runtime stays strict"
)]
mod tests {
    use super::*;

    #[test]
    fn meeting_ending_detects_canonical_patterns() {
        assert!(meeting_ending_phrase_match(
            "Well, thanks for your time today."
        ));
        assert!(meeting_ending_phrase_match("We'll be in touch by Friday."));
        assert!(meeting_ending_phrase_match(
            "Any final questions before we wrap?"
        ));
        assert!(meeting_ending_phrase_match("Let's wrap up here."));
        assert!(meeting_ending_phrase_match("Спасибо за уделённое время!"));
        assert!(meeting_ending_phrase_match("Будем на связи."));
        assert!(meeting_ending_phrase_match("Есть вопросы ко мне?"));
    }

    #[test]
    fn meeting_ending_ignores_mid_interview_thanks() {
        // Conservative: "thanks" alone is not enough; need multi-word
        // wrap-up pattern. Same invariant as src-tauri's test.
        assert!(!meeting_ending_phrase_match("Thanks for explaining that."));
        assert!(!meeting_ending_phrase_match("Спасибо за объяснение."));
        assert!(!meeting_ending_phrase_match(""));
    }

    // NOTE: a start_session smoke test would need a hermetic
    // `SharedConfig` (overlay-backend uses parking_lot::RwLock which
    // isn't a direct dep of slint-experiment). The bail-on-empty-
    // groq path is verified live whenever the user opens the
    // overlay without setting the Groq key. For unit-test scope here
    // we exercise only the meeting-ending phrase matcher + stop_session
    // safety; integration testing belongs in Phase E6.

    /// stop_session on a never-started rt is a no-op (no panics, no
    /// resource leak — just returns empty transcript).
    #[test]
    fn stop_session_on_empty_rt_returns_empty_snapshot() {
        use crate::runtime_state::shared_runtime;
        let rt = shared_runtime();
        let snap = stop_session(rt);
        assert!(snap.is_empty());
    }

    /// v0.12.0 — stop_session clears the rolling window (debrief snapshot
    /// semantics unchanged) but KEEPS the full transcript so the Summary
    /// button still works between Стоп and the next Старт.
    #[test]
    fn stop_session_keeps_full_transcript_for_summary() {
        use crate::runtime_state::{lock, push_transcript_line, shared_runtime};
        let rt = shared_runtime();
        {
            let mut s = lock(&rt);
            for i in 0..3u64 {
                push_transcript_line(
                    &mut s,
                    TranscriptLine {
                        source: AudioSource::Mic,
                        text: format!("реплика {i}"),
                        timestamp_ms: i,
                    },
                );
            }
        }
        let snapshot = stop_session(rt.clone());
        assert_eq!(snapshot.len(), 3, "debrief snapshot must be unchanged");
        let s = lock(&rt);
        assert!(s.transcript.is_empty(), "rolling window clears on stop");
        assert_eq!(
            s.full_transcript.len(),
            3,
            "summary source must survive stop"
        );
    }
}
