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
use overlay_backend::audio::{self, AudioSource, TranscriptLine};
use overlay_backend::config::SharedConfig;
use overlay_backend::events::RuntimeEvents;
use overlay_backend::journal::{now_unix_ms, Journal, JournalEvent};
use overlay_backend::stt;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

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
    // ===== 1. Stop any prior session + reset state =====
    {
        let mut s = lock(&rt);
        s.capture = None; // Drop signals capture thread to stop.
        s.transcript.clear();
        s.session_cost_microcents = 0;
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
    let (mic_dev, sys_dev, groq_key, language, whisper_prompt, stt_model) = {
        let c = cfg.read();
        (
            c.mic_device.clone(),
            c.system_audio_device.clone(),
            c.groq_api_key.clone(),
            c.stt_language.clone(),
            stt::build_whisper_prompt(&c.trigger_keywords, &c.meeting_context),
            c.stt_model.clone(),
        )
    };
    if groq_key.trim().is_empty() {
        anyhow::bail!("Groq API key not set in settings (cfg.groq_api_key empty)");
    }

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
        });
    }
    lock(&rt).journal = Some(journal.clone());

    // ===== 4. Spawn audio capture =====
    let (audio_rx, capture_handle) = audio::start_capture(mic_dev, sys_dev)
        .context("audio::start_capture failed (check mic / system audio devices in Settings)")?;
    let health = lock(&rt).health.clone();

    // ===== 5. Spawn STT pipeline =====
    let stt_rx = stt::spawn(
        audio_rx,
        groq_key,
        language,
        whisper_prompt,
        stt_model,
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
/// transcript:line to UI, runs meeting-ending detection.
///
/// Auto-tile detection is NOT yet wired here — Phase E4 adds it
/// using the same `detect_trigger` + `build_auto_tile_prompts`
/// primitives the src-tauri side uses. Until then this is a
/// faithful transcript pump without auto-spawn.
async fn transcript_forwarder(
    mut stt_rx: tokio::sync::mpsc::Receiver<stt::TranscriptEvent>,
    events: Arc<dyn RuntimeEvents>,
    _cfg: SharedConfig,
    rt: SharedSlintRuntime,
    journal: Journal,
) {
    while let Some(ev) = stt_rx.recv().await {
        // Mic-mute drop — same semantic as src-tauri's check.
        if matches!(ev.source, AudioSource::Mic) && lock(&rt).mic_muted {
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
        // TODO Phase E4: detector_allows + maybe_spawn_tile call.
    }
    log_info("transcript forwarder exit");
}

/// Stop the active session: aborts all spawned tasks, drops the
/// capture handle, closes the journal. Returns the transcript
/// snapshot so the caller can pass it to `run_post_meeting_debrief`
/// (Phase E5 wires this — for now the snapshot is just returned).
pub fn stop_session(rt: SharedSlintRuntime) -> Vec<TranscriptLine> {
    let mut s = lock(&rt);
    s.capture = None;
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
    let snapshot: Vec<TranscriptLine> = s.transcript.iter().cloned().collect();
    s.transcript.clear();
    if let Some(j) = s.journal.take() {
        drop(j); // Phase E5 will replace with close_journal_with_summary.
    }
    snapshot
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
}
