//! Runtime orchestrator: glues audio → STT → rolling transcript → AI.
//!
//! State is stored as a single Arc<Mutex<RuntimeState>> managed by Tauri.
//! Frontend interacts via Tauri commands (start/stop capture, ask).
//! Events flow back via tauri::Emitter (channel name → see fn names).

use crate::ai;
use crate::audio::{self, AudioSource, CaptureHandle};
use crate::config::SharedConfig;
use crate::journal::{now_unix_ms, Journal, JournalEvent};
use crate::stt;
use crate::tile::SharedTiles;

use anyhow::Result;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

// TranscriptLine moved to overlay_backend::audio during Phase B2
// port #1 — the ported run_post_meeting_debrief needs the type
// accessible from overlay-backend without pulling in Tauri.
// Re-export keeps existing `runtime::TranscriptLine` callers
// compiling without churn.
pub use overlay_backend::audio::TranscriptLine;

#[derive(Default)]
pub struct RuntimeState {
    pub capture: Option<CaptureHandle>,
    pub transcript: VecDeque<TranscriptLine>,
    pub last_screenshot: Option<String>, // data URL, consumed on next ask
    /// Recent tile-trigger timestamps to enforce rate-limit.
    pub recent_tile_triggers: VecDeque<Instant>,
    /// Handle of the currently-running transcript forwarder. Aborted on
    /// next start_session() to prevent duplicated emits.
    pub transcript_task: Option<tokio::task::JoinHandle<()>>,
    /// Handle of the most recent ai::stream_chat task. Aborted on next ask()
    /// so a fresh F9 cancels in-flight response instead of stacking.
    pub ai_task: Option<tokio::task::JoinHandle<()>>,
    /// JSONL journal for the current session. None when stopped.
    pub journal: Option<Journal>,
    /// Accumulated session cost in microcents (1 USD = 100M microcents) —
    /// integer avoids f64 precision loss over long sessions.
    pub session_cost_microcents: u64,
    /// Push-to-talk: timestamps (unix ms) when the user pressed the
    /// 🎤/🔊 button. Kept for UI accounting only — actual audio capture
    /// happens via dedicated WASAPI thread (see push_to_talk).
    pub manual_ask_start_ms: HashMap<AudioSource, u64>,
    /// Active push-to-talk capture (separate from main always-on capture).
    /// stop flips on release, samples_rx delivers the raw PCM blob.
    pub push_to_talk: Option<PushToTalkCapture>,
    /// Health signals — shared atomics for the 3-dot Failure HUD in the
    /// overlay bar. Three subsystems tracked: audio (frame from WASAPI),
    /// stt (successful Groq response), ai (successful Claude response).
    /// Stored as Arc so stt::spawn and ai task closures can bump them
    /// without locking the runtime mutex.
    pub health: Arc<HealthSignals>,
    /// Handle of the health-emit ticker (started by `start_session`, aborted
    /// on `stop_session`). Emits `health:update` event every 2s.
    pub health_task: Option<tokio::task::JoinHandle<()>>,
    /// For F3 "Reask" feature: stores the last question the user got an
    /// answer to (any flow — auto-detector, manual ask, PTT). On F3,
    /// builds a fresh AI call with the same question text but the LATEST
    /// transcript as context, plus a hint that the previous answer was
    /// insufficient. Useful when the answer was right but missed nuance
    /// from words spoken after the trigger.
    pub last_question: Option<String>,
    /// Last AI answer text shown to user — passed back into Reask as the
    /// "previous answer" so the model can correct/expand rather than
    /// repeat itself.
    pub last_answer: Option<String>,
    /// Rolling 60s window of mic transcript stats — feeds the live "voice
    /// coach" pill in the overlay bar (filler words, words-per-minute).
    /// Each tuple is (unix_ms, word_count, filler_count) for one transcript
    /// line. Cleared on session_start; trimmed to last 60s on every push.
    pub speech_window: VecDeque<(u64, u32, u32)>,
    /// v0.0.59: once-per-session flag — flipped to true after the meeting-
    /// ending detector fires "thanks for your time" / "we'll be in touch"
    /// match. Prevents repeated 🏁 chip emit if the phrase is uttered
    /// multiple times. Reset to false on session_start.
    pub meeting_ending_emitted: bool,
    /// v0.0.64: recent question prefixes (normalized to lowercase + first
    /// 60 chars + whitespace-collapsed) with their spawn timestamps.
    /// Used to dedup the "interviewer says 'what about kubernetes' 3
    /// times in 30 sec → 3 identical tiles" spam pattern.
    /// Pruned to last 60s on every check.
    pub recent_question_prefixes: Vec<(String, std::time::Instant)>,
    /// v0.0.75: when true, mic transcripts are dropped before the
    /// detector + AI flow. System audio is unaffected. Useful for
    /// coughing/sneezing without polluting the transcript. Toggle
    /// via 🔇 chip in overlay bar. Not persisted (runtime-only).
    pub mic_muted: bool,
    /// v0.0.79: simple per-session Q→A cache for the auto-tile path.
    /// Key is normalized question (lowercase, whitespace-collapsed,
    /// first 200 chars). Value is (full_answer, insertion_instant).
    /// On detector hit, we look up the normalized key; if found and
    /// age < 10 min, reuse the cached answer + skip the AI call.
    /// Doesn't interact with v0.0.64 dedup (which has 60s window):
    /// dedup short-circuits earlier and prevents the second spawn
    /// entirely. Cache covers the window where dedup expired but the
    /// answer is still likely valid. Cleared on start_session.
    pub qa_cache: HashMap<String, (String, std::time::Instant)>,
}

// HealthSignals + HealthPayload were extracted to overlay_backend::health
// during Phase B1. HealthSignals has live src-tauri callers (runtime
// state field). HealthPayload is constructed inside overlay_backend
// only — the React side reads its serialized form, not the Rust type —
// so we don't re-export it here. Anyone needing it can `use
// overlay_backend::health::HealthPayload` directly.
pub use overlay_backend::health::HealthSignals;

/// Russian filler words tracked by the live voice coach. Lowercase,
/// matched as whole words (boundary = non-alphanumeric). Curated from
/// real Russian interview / talk-pattern lists; kept small + high-signal
/// so we don't flag legitimate technical speech.
///
/// Add carefully — false positives turn the pill into noise the user
/// learns to ignore. "Так" was a candidate but it's also a common
/// declarative ("так, дальше:") that good speakers use; left out.
pub(crate) const FILLERS_RU: &[&str] = &[
    "эм",
    "эмм",
    "эмммм",
    "ну",
    "вот",
    "значит",
    "типа",
    "короче",
    "блин",
    "это самое",
    "как бы",
    "в общем",
    "в принципе",
];

/// Live speech-coaching snapshot pushed on `speech:coach` every 2s while a
/// session has mic transcripts coming in. Computed from a rolling 60-second
/// window of mic transcript lines (system audio NOT counted — coach feedback
/// is about the user, not the interviewer).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpeechCoachPayload {
    /// Total words spoken in the last 60s window (mic only).
    pub words_60s: u32,
    /// Filler words within those 60s.
    pub fillers_60s: u32,
    /// Filler density per 100 words spoken in the window. None if <10 words
    /// total — too little data to be meaningful.
    pub filler_per_100: Option<u32>,
    /// Estimated WPM (words spoken in the last 60s, normalised to per-min).
    /// None if window has <5s of data — same "not enough signal" guard.
    pub wpm: Option<u32>,
    /// "low" (<150 wpm) | "ok" (150-180) | "fast" (>200) — UI colour cue.
    /// "idle" when no recent mic speech.
    pub pace: &'static str,
}

/// Count whole-word filler matches in `text` (case-insensitive). Splits on
/// non-alphanumeric so punctuation doesn't shield a filler. Multi-word
/// fillers like "как бы" are matched against the whole string (after
/// lowercasing) since splitting would prevent matching them.
pub(crate) fn count_fillers(text: &str) -> u32 {
    let lower = text.to_lowercase();
    let mut total: u32 = 0;
    // Single-word fillers: count whole-word occurrences.
    let single_tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    for tok in &single_tokens {
        for f in FILLERS_RU {
            if !f.contains(' ') && tok == f {
                total += 1;
            }
        }
    }
    // Multi-word fillers ("как бы", "в общем", "в принципе", "это самое"):
    // search the raw lowercased string. Bounded to short list (4 entries)
    // so the cost is negligible per line.
    for f in FILLERS_RU.iter().filter(|f| f.contains(' ')) {
        // Count non-overlapping occurrences.
        let mut pos = 0usize;
        while let Some(idx) = lower[pos..].find(f) {
            total += 1;
            pos += idx + f.len();
        }
    }
    total
}

/// Count words in a transcript line. Uses the same tokenisation as the
/// detector so the WPM stat aligns with what the model considers "speech".
pub(crate) fn count_words(text: &str) -> u32 {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .count() as u32
}

/// Push a new sample into the rolling 60s mic-speech window. Trims entries
/// older than 60s from the head. Called only for mic-source transcript
/// lines — system-audio lines are ignored (coach scores the user).
pub(crate) fn push_speech_window(rt: &SharedRuntime, ts_ms: u64, words: u32, fillers: u32) {
    let mut s = rt.lock();
    s.speech_window.push_back((ts_ms, words, fillers));
    let cutoff = ts_ms.saturating_sub(60_000);
    while let Some(&(t, _, _)) = s.speech_window.front() {
        if t < cutoff {
            s.speech_window.pop_front();
        } else {
            break;
        }
    }
}

/// Build a `SpeechCoachPayload` from the current 60s window. Returns idle
/// values when no recent mic speech.
pub(crate) fn snapshot_speech_coach(rt: &SharedRuntime, now_ms: u64) -> SpeechCoachPayload {
    let s = rt.lock();
    if s.speech_window.is_empty() {
        return SpeechCoachPayload {
            words_60s: 0,
            fillers_60s: 0,
            filler_per_100: None,
            wpm: None,
            pace: "idle",
        };
    }
    let (mut words, mut fillers) = (0u32, 0u32);
    let oldest_ts = s.speech_window.front().map(|t| t.0).unwrap_or(now_ms);
    for &(_, w, f) in &s.speech_window {
        words = words.saturating_add(w);
        fillers = fillers.saturating_add(f);
    }
    drop(s);
    let span_ms = now_ms.saturating_sub(oldest_ts).max(1) as u32;
    // Min 5 s of data + 10 words before reporting density / pace; below that
    // a single utterance can dominate and produce alarming numbers.
    let wpm = if span_ms >= 5_000 && words >= 5 {
        Some(((words as u64) * 60_000 / span_ms as u64) as u32)
    } else {
        None
    };
    let filler_per_100 = if words >= 10 {
        Some(((fillers as u64) * 100 / words as u64) as u32)
    } else {
        None
    };
    let pace: &'static str = match wpm {
        None => "idle",
        Some(w) if w < 150 => "low",
        Some(w) if w <= 180 => "ok",
        Some(_) => "fast",
    };
    SpeechCoachPayload {
        words_60s: words,
        fillers_60s: fillers,
        filler_per_100,
        wpm,
        pace,
    }
}

/// Owned by RuntimeState while a push-to-talk hold is active.
///
/// `samples_rx` carries `Result<Vec<i16>, String>` — Err surfaces the real
/// WASAPI/COM failure (device gone, format mismatch) to the UI instead of
/// the prior behaviour of silently sending an empty Vec which then got
/// flagged by the duration gate as a misleading "удерживай дольше" message.
pub struct PushToTalkCapture {
    pub source: AudioSource,
    pub start_ms: u64,
    pub stop: Arc<AtomicBool>,
    pub samples_rx: oneshot::Receiver<Result<Vec<i16>, String>>,
    /// JoinHandle of the dedicated capture thread. On cancel we set `stop`
    /// then wait up to ~600ms (capture loop polls the flag every 500ms) for
    /// the thread to exit, so a quick double-press doesn't accumulate
    /// orphan WASAPI sessions. Optional only because tests don't spawn.
    pub thread: Option<std::thread::JoinHandle<()>>,
}

pub type SharedRuntime = Arc<Mutex<RuntimeState>>;

pub fn shared() -> SharedRuntime {
    Arc::new(Mutex::new(RuntimeState::default()))
}

const TRANSCRIPT_MAX_LINES: usize = 80;

/// Bump health.last_ai_ok_ms to "now". Call after any successful AI op
/// (complete_with_usage return, stream Delta arrival).
#[inline]
fn bump_health_ai(rt: &SharedRuntime) {
    let h = rt.lock().health.clone();
    h.last_ai_ok_ms
        .store(now_unix_ms() as u64, Ordering::Relaxed);
}

/// Remember the last question+answer surfaced to the user. F3 Reask
/// rebuilds an AI call with this question + LATEST transcript + the
/// previous answer (so the model corrects/expands rather than repeats).
#[inline]
fn store_last_qa(rt: &SharedRuntime, q: &str, a: &str) {
    let mut s = rt.lock();
    s.last_question = Some(q.to_string());
    s.last_answer = Some(a.to_string());
}

/// Start audio capture + STT pipeline. Drops any prior session first.
///
/// MUST be called from a Tokio runtime context (spawns tokio tasks via stt::spawn).
pub async fn start_session(
    app: AppHandle,
    cfg: SharedConfig,
    rt: SharedRuntime,
    tiles: SharedTiles,
) -> Result<()> {
    // Stop any prior capture + cancel its forwarder task so we don't
    // duplicate emits next time around.
    {
        let mut guard = rt.lock();
        guard.capture = None; // Drop → stop signal
        guard.transcript.clear();
        guard.session_cost_microcents = 0;
        if let Some(h) = guard.transcript_task.take() {
            h.abort();
        }
        if let Some(h) = guard.ai_task.take() {
            h.abort();
        }
        // Abort old health ticker EARLY so it doesn't keep emitting against
        // stale snapshots during the rest of start_session setup, AND so a
        // failed start doesn't leak it (S1 from 2nd-pass review).
        if let Some(h) = guard.health_task.take() {
            h.abort();
        }
        // Reset HealthSignals atomics so first emit after fresh start
        // classifies as "idle", not "down" (stale last-session timestamps).
        // S1 from 2nd-pass — was showing "everything's broken" UX on every
        // session start for ~2s until first chunk/transcript landed.
        guard.health.last_audio_frame_ms.store(0, Ordering::Relaxed);
        guard.health.last_stt_ok_ms.store(0, Ordering::Relaxed);
        guard.health.last_ai_ok_ms.store(0, Ordering::Relaxed);
        // Reset speech coach window — last session's WPM / filler tail must
        // not bleed into a fresh meeting (could surprise the user with
        // "fast" pace when they haven't said a word yet).
        guard.speech_window.clear();
        // v0.0.59: reset meeting-ending flag so a fresh session can
        // re-detect goodbye phrases.
        guard.meeting_ending_emitted = false;
        // v0.0.64: drop recent-question dedup cache for fresh session.
        guard.recent_question_prefixes.clear();
        // v0.0.79: drop AI response cache so a new meeting doesn't
        // inherit stale answers from the previous one (different
        // context entirely).
        guard.qa_cache.clear();
        if let Some(j) = guard.journal.take() {
            close_journal_with_summary(j);
        }
    }

    // Tell the overlay that cost is back to zero so chips that depend on
    // session_usd (running-cost display, "💰 over budget" auto-clear) get a
    // chance to reset. Without this emit, every other cost:update event in
    // the codebase only fires after a successful AI call (always with
    // total > 0), so the UI never sees a session_usd: 0 signal and the
    // over-budget chip can linger from a prior session until its 60s timer
    // fires. Found by post-v0.0.12 agent review.
    let _ = app.emit_to(
        "overlay",
        "cost:update",
        serde_json::json!({ "session_usd": 0.0_f64 }),
    );

    // v0.0.19: reset tile sequence counter so each session starts at #1.
    // Without this the counter keeps climbing across sessions in the same
    // process — confusing when the user expects "this is the first tile".
    crate::tile::reset_seq_counter();

    // Open a fresh journal for this session.
    let journal = match Journal::open_new_session() {
        Ok(j) => j,
        Err(e) => {
            log::warn!("journal open failed: {e:#}");
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
    rt.lock().journal = Some(journal.clone());

    let (mic_dev, sys_dev, groq_key, language, whisper_prompt, stt_model) = {
        let c = cfg.read();
        (
            c.mic_device.clone(),
            c.system_audio_device.clone(),
            c.groq_api_key.clone(),
            c.stt_language.clone(),
            // Bias Whisper towards the user's vocab — dramatically improves
            // tech-term recognition in otherwise-Russian speech.
            stt::build_whisper_prompt(&c.trigger_keywords, &c.meeting_context),
            c.stt_model.clone(),
        )
    };

    if groq_key.trim().is_empty() {
        anyhow::bail!("Groq API key not set in settings");
    }

    let (audio_rx, handle) = audio::start_capture(mic_dev, sys_dev)?;
    let health = rt.lock().health.clone();
    let mut stt_rx = stt::spawn(
        audio_rx,
        groq_key,
        language,
        whisper_prompt,
        stt_model,
        health.clone(),
    );

    rt.lock().capture = Some(handle);

    // Health emitter: every 2s pushes a `health:update` event with the
    // current 3-dot state. Cheap (3 atomic loads + serde + IPC).
    // Same ticker also emits the live `speech:coach` snapshot derived from
    // the rolling 60s mic-speech window — shared cadence keeps frontend
    // listener wiring symmetric and avoids a second tokio task.
    let health_for_tick = health.clone();
    let app_for_tick = app.clone();
    let rt_for_tick = rt.clone();
    let health_task = tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(2));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let now_ms = now_unix_ms() as u64;
            let snap = health_for_tick.snapshot(now_ms);
            let _ = app_for_tick.emit_to("overlay", "health:update", &snap);
            let coach = snapshot_speech_coach(&rt_for_tick, now_ms);
            let _ = app_for_tick.emit_to("overlay", "speech:coach", &coach);
        }
    });
    {
        let mut g = rt.lock();
        if let Some(old) = g.health_task.take() {
            old.abort();
        }
        g.health_task = Some(health_task);
    }

    // Forward transcript events into frontend + rolling buffer + auto-tile detector.
    let rt_for_task = rt.clone();
    let cfg_for_task = cfg.clone();
    let tiles_for_task = tiles.clone();
    let journal_for_task = journal.clone();
    let task = tokio::spawn(async move {
        while let Some(ev) = stt_rx.recv().await {
            // v0.0.75: mic mute — drop mic chunks before they hit the
            // buffer/journal/frontend/detector. System audio is
            // unaffected. Cheap check via short lock — toggle is rare
            // (chip click) so contention is non-existent.
            if matches!(ev.source, AudioSource::Mic) && rt_for_task.lock().mic_muted {
                continue;
            }
            let line = TranscriptLine {
                source: ev.source,
                text: ev.text.clone(),
                timestamp_ms: ev.timestamp_ms,
            };
            {
                let mut s = rt_for_task.lock();
                s.transcript.push_back(line.clone());
                while s.transcript.len() > TRANSCRIPT_MAX_LINES {
                    s.transcript.pop_front();
                }
            }
            // Live voice coach: mic-source lines only — system audio is the
            // interviewer/peer, not user speech, so excluding it keeps the
            // WPM / filler-density stats meaningful as a self-coaching signal.
            if matches!(line.source, AudioSource::Mic) {
                let w = count_words(&line.text);
                let f = count_fillers(&line.text);
                if w > 0 {
                    push_speech_window(&rt_for_task, now_unix_ms() as u64, w, f);
                }
            }
            journal_for_task.write(&JournalEvent::TranscriptLine {
                unix_ms: now_unix_ms(),
                source: match line.source {
                    AudioSource::System => "system",
                    AudioSource::Mic => "mic",
                },
                text: &line.text,
            });
            let _ = app.emit_to("overlay", "transcript:line", &line);

            // v0.0.59: meeting-ending detector. Scan transcript text for
            // common goodbye/wrap-up phrases. Emit `meeting:ending` once
            // per session so the overlay can show a 🏁 chip prompting
            // the user to stop the session. Detector is per-text on the
            // captured side only (system audio) — your own "thanks" to
            // the interviewer shouldn't trigger.
            if line.source == AudioSource::System && meeting_ending_phrase_match(&line.text) {
                let mut s = rt_for_task.lock();
                if !s.meeting_ending_emitted {
                    s.meeting_ending_emitted = true;
                    drop(s);
                    let _ = app.emit_to("overlay", "meeting:ending", ());
                }
            }

            // Auto-tile detector: respect detector_skip_mic config (P1-5).
            // Extracted gate function so we can unit-test the matrix without
            // spinning up Tauri AppHandle / WebView. See `detector_allows_*`
            // tests below.
            let line_source = line.source;
            let skip_mic = cfg_for_task.read().detector_skip_mic;
            if detector_allows(line_source, skip_mic) {
                maybe_spawn_tile(
                    app.clone(),
                    cfg_for_task.clone(),
                    rt_for_task.clone(),
                    tiles_for_task.clone(),
                    line.text,
                )
                .await;
            }
        }
        log::info!("transcript forwarder exit");
    });
    rt.lock().transcript_task = Some(task);

    Ok(())
}

/// v0.0.59: case-insensitive substring scan for common meeting-ending
/// phrases. Returns true if the interviewer is wrapping up. Tuned to be
/// conservative — false positives are worse than misses (you don't want
/// the 🏁 chip popping mid-interview because the candidate said "thanks
/// for the heads-up"). Hence:
/// - Phrases anchored to interviewer-typical wording
/// - Requires multi-word patterns, not single "thanks"
/// - Both English + Russian common goodbyes
///
/// All matching is done on lowercase + Unicode-aware (chars().any
/// after to_lowercase, not byte-level — handles Cyrillic correctly).
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

#[cfg(test)]
mod meeting_ending_tests {
    use super::meeting_ending_phrase_match;

    #[test]
    fn detects_en_common_goodbyes() {
        assert!(meeting_ending_phrase_match(
            "Well, thanks for your time today."
        ));
        assert!(meeting_ending_phrase_match("We'll be in touch by Friday."));
        assert!(meeting_ending_phrase_match(
            "Any final questions before we wrap?"
        ));
        assert!(meeting_ending_phrase_match("Let's wrap up here."));
    }

    #[test]
    fn detects_ru_common_goodbyes() {
        assert!(meeting_ending_phrase_match("Спасибо за уделённое время!"));
        assert!(meeting_ending_phrase_match("Приятно было пообщаться."));
        assert!(meeting_ending_phrase_match("Будем на связи."));
        assert!(meeting_ending_phrase_match("Есть вопросы ко мне?"));
    }

    #[test]
    fn ignores_mid_interview_thanks() {
        // Conservative: "thanks" alone is not enough; need multi-word
        // wrap-up pattern.
        assert!(!meeting_ending_phrase_match("Thanks for explaining that."));
        assert!(!meeting_ending_phrase_match("Спасибо за объяснение."));
        assert!(!meeting_ending_phrase_match("That makes sense, thanks."));
    }

    #[test]
    fn case_insensitive() {
        assert!(meeting_ending_phrase_match("THANKS FOR YOUR TIME"));
        assert!(meeting_ending_phrase_match("WE'LL Be In Touch soon"));
    }

    #[test]
    fn empty_or_short_lines_return_false() {
        assert!(!meeting_ending_phrase_match(""));
        assert!(!meeting_ending_phrase_match("ok"));
        assert!(!meeting_ending_phrase_match("yes"));
    }
}

/// Pull the last `max` lines from `transcript` whose source matches.
/// Pure function — no I/O, no state. Returns the raw text (no source
/// label). Currently unused after manual_ask_source switched to cross-
/// source `select_recent_lines_labeled`, kept as a sibling utility for
/// any future feature that wants just one source's text (e.g. "review
/// what you said this session").
///
/// VecDeque + Iterator::filter loses ExactSizeIterator, so we collect
/// into a Vec first then slice off the last `max`. For a rolling buffer
/// capped at TRANSCRIPT_MAX_LINES (=80) this is cheap.
#[allow(dead_code)]
pub fn select_recent_lines_from_source(
    transcript: &VecDeque<TranscriptLine>,
    source: AudioSource,
    max: usize,
) -> Vec<String> {
    let matching: Vec<String> = transcript
        .iter()
        .filter(|l| l.source == source)
        .map(|l| l.text.clone())
        .collect();
    let start = matching.len().saturating_sub(max);
    matching[start..].to_vec()
}

/// Pull the last `max` lines from the transcript with source labels
/// applied — preserves interleaving so the AI sees who said what.
/// Used by manual_ask_source + manual_spawn_tile to give cross-source
/// context (the relevant question often spans both speakers).
pub fn select_recent_lines_labeled(
    transcript: &VecDeque<TranscriptLine>,
    max: usize,
) -> Vec<String> {
    let n = transcript.len();
    let start = n.saturating_sub(max);
    transcript
        .iter()
        .skip(start)
        .map(|l| {
            let src = match l.source {
                AudioSource::System => "[СОБЕСЕДНИК]",
                AudioSource::Mic => "[ПОЛЬЗОВАТЕЛЬ]",
            };
            format!("{src} {}", l.text)
        })
        .collect()
}

/// Find the most-recent line from a specific source — used as the "ask
/// about THIS" trigger when the user presses a source-specific button.
pub fn find_last_line_from_source(
    transcript: &VecDeque<TranscriptLine>,
    source: AudioSource,
) -> Option<String> {
    transcript
        .iter()
        .rev()
        .find(|l| l.source == source)
        .map(|l| l.text.clone())
}

// ── Auto-tile detector ───────────────────────────────────────────────────

/// Max tiles spawned per minute (rate-limit on Haiku spend).
/// Bumped from 6 → 15 — for active interviews 6 was way under capacity
/// (live test showed 0 rate-limit hits even at peak).
const MAX_TILES_PER_MIN: usize = 15;

/// Bumped rate-limit when `auto_tile_every_line` is on. Whisper produces
/// ~30-50 transcript lines per minute of continuous speech; the regular
/// 15/min cap would strangle aggressive mode immediately. 60/min ≈ 1/sec
/// matches the actual transcript throughput.
const MAX_TILES_PER_MIN_AGGRESSIVE: usize = 60;

/// Confidence gate for KB context injection: does every alphanumeric token of
/// the KB entry's key appear as a token in the trigger?
///
/// Both sides go through the SAME tokeniser (`split on !alphanumeric`), so
/// hyphenated keys like `kubectl-debug` or `git-recovery` (~30% of the
/// `commands.md` corpus) are not silently dropped. Single-token keys like
/// `kubernetes` work via the same `all(.contains)` path.
///
/// Live regression: before this helper, `key.as_str().contains` on a HashSet
/// of trigger tokens missed `kubectl-debug` because the stored entry key
/// retained the hyphen while tokens never did.
pub(crate) fn kb_key_matches_trigger(key: &str, trigger: &str) -> bool {
    let trig_lower = trigger.to_lowercase();
    let trig_tokens: std::collections::HashSet<&str> = trig_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let key_lower = key.to_lowercase();
    let entry_tokens: Vec<&str> = key_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    !entry_tokens.is_empty() && entry_tokens.iter().all(|t| trig_tokens.contains(t))
}

async fn maybe_spawn_tile(
    app: AppHandle,
    cfg: SharedConfig,
    rt: SharedRuntime,
    tiles: SharedTiles,
    text: String,
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
    ) = {
        let c = cfg.read();
        (
            c.auto_tiles_enabled,
            c.auto_tile_every_line,
            c.trigger_keywords.clone(),
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.ai_model.clone(), // Haiku — speed matters here
            c.response_language.clone(),
            c.meeting_context.clone(),
        )
    };
    if !enabled || bearer.trim().is_empty() {
        return;
    }

    let journal = rt.lock().journal.clone().unwrap_or_default();
    // AGGRESSIVE MODE (v0.0.18): bypass detect_trigger and treat every
    // line as a Question. We still log a detector_decision event so
    // Replay viewer can show the audit trail, but with trigger_kind
    // "every_line" so it's obvious why we fired.
    let detected = if every_line {
        // Only skip empty / very short lines (Whisper sometimes emits
        // single-char artefacts that aren't worth a tile).
        if text.trim().chars().count() < 5 {
            None
        } else {
            Some(Trigger::Question(text.clone()))
        }
    } else {
        detect_trigger(&text, &trigger_keywords)
    };
    let (triggered, trigger_kind): (bool, Option<String>) = match &detected {
        Some(Trigger::Question(_)) if every_line => (true, Some("every_line".into())),
        Some(Trigger::Question(_)) => (true, Some("question".into())),
        Some(Trigger::Keyword(kw, _)) => (true, Some(format!("keyword:{kw}"))),
        None => (false, None),
    };
    journal.write(&JournalEvent::DetectorDecision {
        unix_ms: now_unix_ms(),
        text: &text,
        triggered,
        trigger_kind: trigger_kind.as_deref(),
    });
    let Some(trigger) = detected else { return };

    // Rate-limit: drop if we already spawned MAX_TILES_PER_MIN in last 60s.
    {
        let mut s = rt.lock();
        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(60);
        while let Some(front) = s.recent_tile_triggers.front() {
            if *front < cutoff {
                s.recent_tile_triggers.pop_front();
            } else {
                break;
            }
        }
        let cap = if every_line {
            MAX_TILES_PER_MIN_AGGRESSIVE
        } else {
            MAX_TILES_PER_MIN
        };
        if s.recent_tile_triggers.len() >= cap {
            log::warn!(
                "tile rate-limit hit ({}/{} in last 60s, aggressive={}) — dropping trigger from text: {}",
                s.recent_tile_triggers.len(),
                cap,
                every_line,
                text.chars().take(60).collect::<String>()
            );
            journal.write(&JournalEvent::RateLimited {
                unix_ms: now_unix_ms(),
                what: "auto_tile",
                text: &text,
            });
            // Notify frontend so user knows AI suggestion was throttled.
            let _ = app.emit_to(
                "overlay",
                "tile:rate-limited",
                serde_json::json!({ "text": text }),
            );
            return;
        }
        s.recent_tile_triggers.push_back(now);
    }

    // v0.0.64: dedup recently-spawned questions. Normalize text to
    // lowercase + first 60 chars + whitespace-collapsed. If the same
    // normalized prefix was used to spawn a tile in last 60s, skip.
    // Stops the "same keyword 3× in 30s" spam pattern.
    {
        let normalized: String = text
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(60)
            .collect();
        let mut s = rt.lock();
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(60);
        s.recent_question_prefixes.retain(|(_, ts)| *ts > cutoff);
        if s.recent_question_prefixes
            .iter()
            .any(|(prefix, _)| prefix == &normalized)
        {
            log::info!("tile dedup: skipping recently-spawned question prefix: {normalized}");
            return;
        }
        s.recent_question_prefixes.push((normalized, now));
    }

    // v0.0.79: AI response cache — check before paying for an AI call.
    // Key normalization: lowercase + collapse whitespace + first 200
    // chars (more characters than dedup uses since cache covers a much
    // wider matching surface). TTL 10 min — long enough to catch the
    // pattern "interviewer asked X at 5:00 and again at 25:00" but
    // short enough that stale context doesn't ruin freshness.
    // v0.0.85 P0 fix: bake ai_model + response_language + meeting_context
    // length into the key. Without these, F2 profile cycle or 🧠 model
    // chip click would return stale cached answers from before the
    // switch.
    // v0.0.92 P1 follow-up: meeting_context.len() was a cheap proxy for
    // "context changed", but two same-length edits (a typo fix, equal-
    // length paste swap) would silently return stale answers. Now hash
    // the full meeting_context via DefaultHasher (one-shot, no extra
    // crate). Also include trigger_keywords hash since v0.0.66 detector
    // tweaks change which triggers fire.
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    meeting_context.hash(&mut h);
    let ctx_hash = h.finish();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    trigger_keywords.hash(&mut h2);
    let kw_hash = h2.finish();
    let cache_seed: String = format!(
        "m={};l={};c={:x};k={:x};q={}",
        model,
        response_language,
        ctx_hash,
        kw_hash,
        text.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect::<String>(),
    );
    let cache_key = cache_seed;
    let cache_hit: Option<String> = {
        let mut s = rt.lock();
        let now = std::time::Instant::now();
        let ttl = std::time::Duration::from_secs(600);
        // Prune stale entries opportunistically — cheap O(n) walk
        // since cache stays tiny (<100 entries even in long sessions).
        s.qa_cache
            .retain(|_, (_, ts)| now.duration_since(*ts) < ttl);
        s.qa_cache.get(&cache_key).map(|(a, _)| a.clone())
    };
    if let Some(cached_answer) = cache_hit {
        log::info!(
            "qa_cache HIT (avoided AI call): {}",
            text.chars().take(60).collect::<String>()
        );
        // Spawn the tile with cached answer + journal the reuse so it
        // shows in Replay. Skip AI request/response events — those are
        // for actual model calls.
        let (preferred_monitor, stealth) = {
            let c = cfg.read();
            (c.tile_monitor_name.clone(), c.stealth_enabled)
        };
        let trigger_text_for_q = match &trigger {
            Trigger::Question(q) => q.clone(),
            Trigger::Keyword(kw, _) => kw.clone(),
        };
        store_last_qa(&rt, &trigger_text_for_q, &cached_answer);
        match crate::tile::spawn_tile_with_stealth(
            &app,
            &tiles,
            trigger_text_for_q.clone(),
            cached_answer.clone(),
            preferred_monitor,
            stealth,
            crate::tile::TileKind::Auto,
        ) {
            Ok(label) => journal.write(&JournalEvent::TileSpawn {
                unix_ms: now_unix_ms(),
                label: &label,
                question: &trigger_text_for_q,
                answer: &cached_answer,
            }),
            Err(e) => log::warn!("cached-tile spawn failed: {e:#}"),
        }
        return;
    }

    log::info!("auto-tile triggered: {:?}", trigger);

    // Cost budget WARN (not block) — see over_cost_budget docstring for
    // why this is no longer a hard rail. We emit the cap-hit event once
    // per crossing so the UI can show a persistent "over budget" chip,
    // but the AI call proceeds normally. User can stop_session if they
    // actually want to stop the bleeding.
    let (cap_usd, current_micro) = {
        let s = rt.lock();
        (cfg.read().max_session_cost_usd, s.session_cost_microcents)
    };
    if let Some(reason) = over_cost_budget(cap_usd, current_micro) {
        let _ = app.emit_to(
            "overlay",
            "cost:cap-hit",
            serde_json::json!({ "reason": reason, "source": "auto_tile", "blocking": false }),
        );
    }

    // Capture last 5 lines for AI context (don't pass single line — context matters).
    let recent_transcript: Vec<String> = {
        let s = rt.lock();
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

    // KB integration: search the embedded knowledge base for a hit on
    // the trigger text. If top result has high-confidence match (key
    // appears as token in trigger), prepend its body as "Релевантная
    // KB-запись" so the AI is grounded in known-good content instead
    // of relying on the model's compressed knowledge alone.
    // Live-test 2026-05-25: detector fires on "Какой-нибудь Kubernetes?"
    // → KB has /kubernetes entry with full definition + ops checklist.
    // AI answer quality jumps when this is in the prompt.
    let trigger_text = match &trigger {
        Trigger::Question(q) => q.clone(),
        Trigger::Keyword(kw, _) => kw.clone(),
    };
    let kb_context_addon: String = {
        let hits = crate::kb::search(&trigger_text, 1);
        match hits.into_iter().next() {
            Some(entry) => {
                if kb_key_matches_trigger(&entry.key, &trigger_text) {
                    log::info!(
                        "KB context injected for trigger '{}' → entry '{}'",
                        trigger_text.chars().take(40).collect::<String>(),
                        entry.key
                    );
                    format!(
                        "\n\n=== Релевантная KB-запись (используй как опорный материал) ===\n\
                         **{}**\n{}",
                        entry.heading, entry.body
                    )
                } else {
                    String::new()
                }
            }
            None => String::new(),
        }
    };
    let augmented_context = if kb_context_addon.is_empty() {
        meeting_context.clone()
    } else {
        format!("{}{}", meeting_context, kb_context_addon)
    };

    let (system_prompt, prompt) = build_auto_tile_prompts(
        &trigger,
        &recent_transcript,
        &augmented_context,
        &response_language,
    );

    // Keep full clones for journal (no truncation — full prompts let us
    // iterate prompt engineering later).
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

    // Non-streaming — we need the full answer before spawning the tile.
    let t0 = std::time::Instant::now();
    let (answer, usage) =
        match ai::complete_with_usage(&base_url, &bearer, &model, messages, 512).await {
            Ok((t, u)) => {
                bump_health_ai(&rt);
                (t.trim().to_string(), u)
            }
            Err(e) => {
                log::warn!("auto-tile AI failed: {e:#}");
                journal.write(&JournalEvent::Error {
                    unix_ms: now_unix_ms(),
                    module: "auto_tile_ai",
                    message: &format!("{e:#}"),
                });
                return;
            }
        };
    let latency_ms = t0.elapsed().as_millis() as u64;

    // v0.0.79: cache the answer so future identical questions in this
    // session skip the AI call. cache_key was computed before the AI
    // request (same normalization as the cache_hit check above).
    // Bounded growth: if we ever hit 256 entries (unrealistic for a
    // single session but defense in depth), drop the oldest half.
    {
        let mut s = rt.lock();
        if s.qa_cache.len() >= 256 {
            // Sort entries by ts and drop the older half. Done in-place
            // by collecting expired keys — O(n log n) but only fires
            // once per session if at all.
            let now = std::time::Instant::now();
            let mut by_age: Vec<(String, std::time::Duration)> = s
                .qa_cache
                .iter()
                .map(|(k, (_, ts))| (k.clone(), now.duration_since(*ts)))
                .collect();
            by_age.sort_by_key(|(_, age)| std::cmp::Reverse(*age));
            for (k, _) in by_age.into_iter().take(128) {
                s.qa_cache.remove(&k);
            }
        }
        s.qa_cache
            .insert(cache_key, (answer.clone(), std::time::Instant::now()));
    }

    // Accumulate cost + notify UI.
    let micro = ai::cost_microcents(&model, usage.input, usage.output);
    let total_usd = {
        let mut s = rt.lock();
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    };
    let _ = app.emit_to(
        "overlay",
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

    let question_label = match &trigger {
        Trigger::Question(q) => q.clone(),
        Trigger::Keyword(kw, _) => format!("📚 {}", kw),
    };

    let (preferred_monitor, stealth) = {
        let c = cfg.read();
        (c.tile_monitor_name.clone(), c.stealth_enabled)
    };

    // v0.0.20: collect keywords to highlight inside the tile content.
    // For Keyword triggers, the matched keyword is obvious. For Question
    // triggers, scan the trigger_keywords config + question tokens
    // intersection — usually 0-3 matches per question.
    let highlights: Vec<String> = match &trigger {
        Trigger::Keyword(kw, _) => vec![kw.clone()],
        Trigger::Question(q) => {
            let q_lower = q.to_lowercase();
            let q_tokens: std::collections::HashSet<&str> = q_lower
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.len() >= 3)
                .collect();
            let mut hits: Vec<String> = trigger_keywords
                .split_whitespace()
                .filter(|kw| {
                    let lower = kw.to_lowercase();
                    q_tokens.contains(lower.as_str())
                })
                .take(8)
                .map(|s| s.to_string())
                .collect();
            // Also include longer keywords (>=4 chars) that appear as
            // substring in the question — catches multi-word entries like
            // "kubernetes operator" that don't tokenise the same way.
            if hits.len() < 8 {
                for kw in trigger_keywords.split_whitespace() {
                    if kw.len() >= 4
                        && q_lower.contains(&kw.to_lowercase())
                        && !hits.iter().any(|h| h.eq_ignore_ascii_case(kw))
                    {
                        hits.push(kw.to_string());
                        if hits.len() >= 8 {
                            break;
                        }
                    }
                }
            }
            hits
        }
    };

    store_last_qa(&rt, &question_label, &answer);
    match crate::tile::spawn_tile_with_highlight(
        &app,
        &tiles,
        question_label.clone(),
        answer.clone(),
        preferred_monitor,
        stealth,
        crate::tile::TileKind::Auto,
        highlights,
    ) {
        Ok(label) => {
            journal.write(&JournalEvent::TileSpawn {
                unix_ms: now_unix_ms(),
                label: &label,
                question: &question_label,
                answer: &answer,
            });
        }
        Err(e) => {
            log::warn!("spawn_tile failed: {e:#}");
            journal.write(&JournalEvent::Error {
                unix_ms: now_unix_ms(),
                module: "tile_spawn",
                message: &format!("{e:#}"),
            });
        }
    }
}

// Trigger moved to overlay_backend::runtime during Phase B2 port #2.
// Used by 7 sites here + 2 sites in lib.rs (DetectorTestResult mapping).
pub use overlay_backend::runtime::Trigger;

/// Cheap noise filter for Whisper artefacts. We accept the line if:
/// - At least 2 word-like tokens (3+ chars each)
/// - At least 60% alpha/digit characters (rest = spaces/punct)
/// - Not a single repeated word ("ага ага ага ага")
///
/// Cyrillic counts via char.is_alphanumeric().
fn looks_like_real_speech(text: &str) -> bool {
    let total: usize = text.chars().count();
    if total == 0 {
        return false;
    }
    let alnum: usize = text.chars().filter(|c| c.is_alphanumeric()).count();
    if (alnum as f32 / total as f32) < 0.60 {
        return false;
    }
    let tokens: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.chars().count() >= 3)
        .collect();
    if tokens.len() < 2 {
        return false;
    }
    // Single-word echo? ("угу угу угу угу")
    let first = tokens[0].to_lowercase();
    if tokens.iter().all(|t| t.to_lowercase() == first) {
        return false;
    }
    true
}

// build_auto_tile_prompts moved to overlay_backend::runtime during
// Phase B2 port #2. Used by 7 sites in runtime.rs across the auto-
// detector + reask + manual-ask paths; reused verbatim post-port.
pub use overlay_backend::runtime::build_auto_tile_prompts;

/// Drop common conversational filler prefixes ("а ", "ну ", "вот ", "так ", "и ")
/// from the start of a sentence so the interrogative-test sees the meaningful
/// first word. "А расскажи как..." → "расскажи как..." (triggers).
/// Strips up to 3 stacked fillers and any leading punctuation.
fn strip_filler_prefix(lower: &str) -> String {
    // Word-only fillers (no trailing space). Boundary is detected by the
    // next char being non-alnum — handles "вот," / "так." / "ну!" etc.
    const FILLERS: &[&str] = &[
        "а",
        "ну",
        "вот",
        "так",
        "и",
        "ладно",
        "хорошо",
        "слушай",
        "ой",
        "эх",
        "ага",
        "угу",
        "да",
        "ок",
        "о'кей",
        "окей",
    ];
    let trim_punct = |s: &str| -> String {
        s.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '?')
            .to_string()
    };
    let mut s = trim_punct(lower);
    for _ in 0..4 {
        let mut matched = false;
        for f in FILLERS {
            if let Some(rest) = s.strip_prefix(f) {
                // Word boundary: filler must be followed by non-alnum
                // (space, comma, punct) or end. Avoids matching "вот"
                // as prefix of "воткни".
                let next_is_alnum = rest.chars().next().is_some_and(|c| c.is_alphanumeric());
                if !next_is_alnum {
                    s = trim_punct(rest);
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            break;
        }
    }
    s
}

pub fn detect_trigger(text: &str, keyword_list: &str) -> Option<Trigger> {
    let trimmed = text.trim();
    if trimmed.len() < 5 {
        return None;
    }
    // Whisper artefact filter — if the transcript is mostly weird characters
    // or has too few real word-like tokens, skip to avoid spam AI calls.
    if !looks_like_real_speech(trimmed) {
        log::debug!(
            "detector noise-filter: '{}'",
            trimmed.chars().take(60).collect::<String>()
        );
        return None;
    }
    let lower = trimmed.to_lowercase();

    // 1. '?' ANYWHERE — Whisper rarely puts it in speech but if it does we
    // definitely want it. BUT only if utterance has enough content:
    // single-word + ? ("Kubernetes?") and 2-word fragments are usually
    // restatements/clarifications, not real questions. Min 4 words skips
    // those without hurting recall on real interview questions
    // (typical ≥6 words: "Расскажи как ты настраивал X?").
    // Live-test 2026-05-25: "Какой-нибудь Kubernetes?" fired tile —
    // user complained. This guard suppresses without dropping the
    // long-form "?" questions detector v3 was already catching.
    if trimmed.contains('?') {
        let word_count = lower.split_whitespace().count();
        if word_count >= 4 {
            return Some(Trigger::Question(trimmed.to_string()));
        } else {
            log::debug!(
                "detector skip short-? utterance ({} words): '{}'",
                word_count,
                trimmed.chars().take(80).collect::<String>()
            );
        }
    }

    // 2. Sentence-leading interrogatives + request verbs.
    //
    // CRITICAL: live test showed " что " / " когда " / " как " in the MIDDLE
    // of a sentence are usually conjunctions ("я знаю, ЧТО Y", "когда он
    // загрузился — отдаёт параметры", "не понятно, КАК это работает") —
    // not questions. Matching them anywhere caused ~50% false-positive rate.
    //
    // New rule: interrogative pronouns must be the FIRST word (with optional
    // filler prefix like "А "). Request verbs ("расскажи", "опиши") can be
    // first or follow a short filler ("А расскажи"). Hypothetical scenarios
    // ("допустим", "представь") same. Question marks anywhere still trigger
    // (handled above in step 1).
    const SENTENCE_LEADING: &[&str] = &[
        // Interrogative pronouns — must be at start (after optional ", А ").
        // NOTE: "когда" intentionally EXCLUDED — even at sentence start it's
        // almost always a temporal subordinate conjunction in spoken Russian
        // ("Когда X, Y" = "When X, Y" — statement). Real "Когда?" questions
        // almost always end in '?' and are caught by step 1.
        // "где" / "кто" / "чей" similarly excluded — high FP-to-TP ratio.
        "что ",
        "как ",
        "почему ",
        "зачем ",
        "какой ",
        "какая ",
        "какое ",
        "какие ",
        "сколько ",
        "чем ",
        // Request verbs — interview pattern
        "расскажи",
        "опиши",
        "поясни",
        "объясни",
        "поделись",
        "приведи пример",
        "приведите пример",
        // Hypothetical scenario openers
        "допустим",
        "представь",
        "представим",
        "если у тебя",
        "если у вас",
        "с чего",
        "с какого",
        // Meta-question openers — interviewer signalling a question is coming
        // ("давай спросим у тебя…", "давай обсудим…", "поговорим про…").
        // Task #103 followup — these were missed before detector v4.
        "давай спросим",
        "давай обсудим",
        "давай поговорим",
        "давай разберём",
        "давай разберем",
        "поговорим про",
        "поговорим о",
        "обсудим",
        // English-mixed (interviews often switch). Include request verbs
        // like "tell me" — many interviewers code-switch mid-sentence.
        "how ",
        "what ",
        "why ",
        "explain ",
        "describe ",
        "tell me ",
    ];
    // Strip optional filler prefix words ("а", "ну", "вот", "так", "и")
    // before checking — they're common conversational starters before a
    // real question.
    let stripped = strip_filler_prefix(&lower);
    for trigger in SENTENCE_LEADING {
        if stripped.starts_with(trigger) {
            return Some(Trigger::Question(trimmed.to_string()));
        }
    }

    // 3. Keyword match (case-insensitive whole-word, alnum boundary).
    // Optimisation (2nd-pass S2): tokenise `lower` ONCE into a HashSet,
    // then O(1) lookup per keyword. Previously we re-split `lower` for
    // every keyword in the user's 250+ token list → O(N·M) per line.
    // With 250 keywords × 8 transcript lines/sec = 2000 splits/sec on
    // the audio hot path. New layout: split once, 250 hashset lookups.
    let tokens: std::collections::HashSet<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    for kw in keyword_list.split_whitespace() {
        // Cheap path first: if the original keyword (already lowercased
        // by the caller's source list, but we don't enforce that) is
        // ASCII-lowercase already, skip the to_lowercase() allocation.
        let kw_lower_owned;
        let kw_lookup: &str = if kw.bytes().all(|b| !b.is_ascii_uppercase()) {
            kw
        } else {
            kw_lower_owned = kw.to_lowercase();
            // Safety: extending the borrow this way requires a leak; instead
            // do the lookup inside the else branch.
            if tokens.contains(kw_lower_owned.as_str()) {
                return Some(Trigger::Keyword(kw.to_string(), trimmed.to_string()));
            }
            continue;
        };
        if tokens.contains(kw_lookup) {
            return Some(Trigger::Keyword(kw.to_string(), trimmed.to_string()));
        }
    }

    log::debug!(
        "detector skipped: '{}'",
        trimmed.chars().take(80).collect::<String>()
    );
    None
}

/// F3 Reask: thin Tauri shim around `overlay_backend::runtime::reask_last`.
///
/// Phase B2 port #2: the body moved to overlay-backend. This shim:
///   1. Snapshots `SharedRuntime` state into `ReaskInputs` under one
///      lock acquisition (matches the original lock-then-await
///      pattern — does NOT hold the lock across the AI call).
///   2. Constructs `TauriEvents` adapter.
///   3. Calls the ported async fn.
///   4. On success-outcome: re-acquires the rt lock to apply the
///      `cost_microcents_delta` saturating-add + `store_last_qa(...)`
///      then emits the `cost:update` event with the new session
///      total (preserves wire-level ordering from the original).
pub async fn reask_last(app: AppHandle, cfg: SharedConfig, rt: SharedRuntime, tiles: SharedTiles) {
    let inputs = {
        let s = rt.lock();
        overlay_backend::runtime::ReaskInputs {
            last_question: s.last_question.clone(),
            last_answer: s.last_answer.clone(),
            recent_transcript_iconized: s
                .transcript
                .iter()
                .rev()
                .take(10)
                .rev()
                .map(|l| {
                    let icon = match l.source {
                        AudioSource::System => "🗣",
                        AudioSource::Mic => "🎤",
                    };
                    format!("{icon} {}", l.text)
                })
                .collect(),
            journal: s.journal.clone(),
            health: s.health.clone(),
        }
    };

    let events: Arc<dyn overlay_backend::events::RuntimeEvents> = Arc::new(crate::TauriEvents {
        app: app.clone(),
        tiles: tiles.clone(),
    });

    let outcome = overlay_backend::runtime::reask_last(events, cfg, inputs).await;

    if let Some(out) = outcome {
        // Apply writebacks under the rt lock + emit cost:update with
        // the new session total. Matches the original wire ordering
        // (cost:update fires after the session-cost mutation).
        let total = {
            let mut s = rt.lock();
            s.session_cost_microcents = s
                .session_cost_microcents
                .saturating_add(out.cost_microcents_delta);
            s.last_question = Some(out.display_question);
            s.last_answer = Some(out.answer_trimmed);
            (s.session_cost_microcents as f64) / 100_000_000.0
        };
        let _ = app.emit_to(
            "overlay",
            "cost:update",
            serde_json::json!({ "session_usd": total }),
        );
    }
}

pub fn stop_session(
    app: AppHandle,
    cfg: SharedConfig,
    rt: SharedRuntime,
    tiles: crate::tile::SharedTiles,
) {
    // Snapshot the transcript BEFORE we drop the journal — debrief needs
    // it. Holding the lock for the entire stop is fine; this fn runs on
    // the Tauri command thread, not the audio path.
    //
    // After snapshotting we CLEAR the transcript so a rapid second Stop
    // (e.g. user double-clicks the Stop button) can't snapshot the same
    // text and queue a duplicate debrief tile + Sonnet call. The transcript
    // is session-scoped anyway — keeping it would also bleed into the next
    // session's recent-lines logic.
    let (transcript_snapshot, session_started_at) = {
        let mut guard = rt.lock();
        guard.capture = None;
        // Abort the health emitter so it stops firing `health:update` events
        // after the session ends. Without this the UI would keep getting
        // "stale" health dots that never go green again.
        if let Some(h) = guard.health_task.take() {
            h.abort();
        }
        // Zero out health atomics so a final emit shows idle state, and
        // future start_session reads them as "never yet" (None → "idle").
        // Without this, after stop_session the dots froze on whatever
        // color they had at the moment of stop — looked like "everything
        // is still working" when actually nothing is running.
        guard
            .health
            .last_audio_frame_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        guard
            .health
            .last_stt_ok_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        guard
            .health
            .last_ai_ok_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        // Snapshot before closing journal (debrief reads from this snapshot,
        // not from the live transcript — which may keep growing if a stray
        // STT result lands after stop).
        let snap: Vec<TranscriptLine> = guard.transcript.iter().cloned().collect();
        guard.transcript.clear();
        let started_at = guard
            .journal
            .as_ref()
            .and_then(|j| j.snapshot_counters())
            .map(|c| c.start_unix_ms)
            .unwrap_or(0);
        if let Some(j) = guard.journal.take() {
            close_journal_with_summary(j);
        }
        (snap, started_at)
    };
    // Emit ONE final health snapshot post-zero so the UI sees the idle
    // state immediately (dots go gray). Without this the React state
    // keeps showing the last green/yellow/red until next start_session.
    let final_health = rt.lock().health.snapshot(now_unix_ms() as u64);
    let _ = app.emit_to("overlay", "health:update", &final_health);
    // Spawn the post-meeting debrief as fire-and-forget. Costs ~1 Sonnet
    // call per session; skipped if the meeting was <30s (nothing to coach
    // about) or fewer than 5 mic transcript lines (test/silent session).
    // Disabled if config flag set OFF or if AI bearer is empty.
    let now = now_unix_ms();
    let duration_ms = now.saturating_sub(session_started_at);
    let mic_lines = transcript_snapshot
        .iter()
        .filter(|l| matches!(l.source, AudioSource::Mic))
        .count();
    // Pre-compute the same mic-text that the debrief runner would build,
    // so should_run_debrief can short-circuit on tiny text BEFORE the Sonnet
    // call goes out (P0-2 fix from review 2026-05-25 — previously the
    // <50-char check happened inside run_post_meeting_debrief AFTER the bill
    // was already in flight for fire-and-forget tokio::spawn).
    let mic_text_chars = transcript_snapshot
        .iter()
        .filter(|l| matches!(l.source, AudioSource::Mic))
        .map(|l| l.text.len())
        .sum::<usize>();
    let enabled = cfg.read().post_meeting_debrief_enabled;
    let has_bearer = !cfg.read().ai_bearer.trim().is_empty();
    match should_run_debrief(enabled, duration_ms, mic_lines, mic_text_chars, has_bearer) {
        Ok(()) => {
            // CRITICAL: must be tauri::async_runtime::spawn, NOT tokio::spawn.
            // stop_session is a sync Tauri command — Tauri 2 runs sync
            // commands on a thread WITHOUT a tokio reactor in TLS, so
            // tokio::spawn here panics with "there is no reactor running".
            // Live crash 2026-05-26 (v0.0.21 panic log src/runtime.rs:1437).
            // tauri::async_runtime::spawn always works — it uses Tauri's
            // own runtime that's installed before commands fire. Same task #93.
            // Phase B2 port #1 — run_post_meeting_debrief now lives in
            // overlay_backend::runtime. We construct a TauriEvents adapter
            // here and delegate. The tauri::async_runtime::spawn wrapping
            // stays (TLS reactor requirement; see comment above).
            use std::sync::Arc;
            let events: Arc<dyn overlay_backend::events::RuntimeEvents> =
                Arc::new(crate::TauriEvents {
                    app: app.clone(),
                    tiles: tiles.clone(),
                });
            tauri::async_runtime::spawn(async move {
                overlay_backend::runtime::run_post_meeting_debrief(
                    events,
                    cfg,
                    transcript_snapshot,
                )
                .await;
            });
        }
        Err(reason) => {
            log::info!("post-meeting debrief skipped: {reason}");
        }
    }
}

/// Gate function: returns Ok if the debrief should run, Err with a
/// human-readable reason otherwise. Pure — depends only on its arguments,
/// no I/O. Extracted so it can be unit-tested without the spawn / AI path.
/// `duration_ms` is `u128` because `journal::now_unix_ms` returns u128 —
/// we don't truncate at the caller.
/// Returns Some(reason) if session cost has crossed the soft-warning budget,
/// None otherwise. Pure — extracted for unit testing. `cap_usd` of 0 (or
/// negative) disables the warning entirely.
///
/// History: this used to HARD-BLOCK new AI calls (v0.0.2-0.0.4). User
/// rightfully pointed out: "странное решение" — blocking mid-interview is
/// worse than the runaway-spend it tries to prevent. The auto-tile
/// rate-limit (15/min) already caps the actual blast radius. So v0.0.5
/// converts this to a passive "over budget" indicator — calls still go
/// through, user just SEES the spend ticking past their threshold and
/// can decide to stop_session manually if needed.
/// Detector gate: decides whether a transcript line of `source` should
/// reach the auto-tile detector. When `skip_mic` is true, mic lines are
/// dropped (candidate's own voice doesn't trigger explanation tiles).
/// System-audio lines (interviewer) always pass through.
///
/// Pure — extracted from the transcript forwarder so the gate matrix
/// can be unit-tested without spinning up AppHandle / WebView / audio.
pub(crate) fn detector_allows(source: AudioSource, skip_mic: bool) -> bool {
    match source {
        AudioSource::Mic => !skip_mic,
        AudioSource::System => true,
    }
}

pub(crate) fn over_cost_budget(cap_usd: f64, current_microcents: u64) -> Option<String> {
    if cap_usd <= 0.0 {
        return None; // disabled
    }
    let current_usd = (current_microcents as f64) / 100_000_000.0;
    if current_usd >= cap_usd {
        Some(format!(
            "over budget: ${:.4} spent ≥ ${:.2} (Settings → Max cost per session)",
            current_usd, cap_usd
        ))
    } else {
        None
    }
}

pub(crate) fn should_run_debrief(
    enabled: bool,
    duration_ms: u128,
    mic_lines: usize,
    mic_text_chars: usize,
    has_bearer: bool,
) -> Result<(), &'static str> {
    if !enabled {
        return Err("disabled by config");
    }
    if duration_ms < 30_000 {
        return Err("session too short (<30s)");
    }
    if mic_lines < 5 {
        return Err("fewer than 5 mic lines");
    }
    // 50 chars ≈ one short sentence — anything less and Sonnet can't yield
    // 3 meaningful coaching observations. Previously checked AFTER the AI
    // call already cost money; now gated upfront (P0-2, review 2026-05-25).
    if mic_text_chars < 50 {
        return Err("mic transcript too short (<50 chars)");
    }
    if !has_bearer {
        return Err("no AI bearer configured");
    }
    Ok(())
}

// run_post_meeting_debrief MOVED to `overlay_backend::runtime` as part
// of Phase B2 port #1. The stop_session caller above constructs a
// `TauriEvents` adapter (defined in src/lib.rs) + delegates. Body
// reproduction (~85 LOC) lives at `overlay-backend/src/runtime.rs`.

/// Flushes a SessionSummary (from accumulated counters) and a SessionStop
/// event, then closes the journal. Called from both start_session (when
/// rolling over an existing session) and stop_session (explicit stop).
fn close_journal_with_summary(j: Journal) {
    if let Some(counters) = j.snapshot_counters() {
        let now = now_unix_ms();
        j.write(&JournalEvent::SessionSummary {
            unix_ms: now,
            duration_ms: now.saturating_sub(counters.start_unix_ms),
            transcript_lines: counters.transcript_mic + counters.transcript_system,
            transcript_mic: counters.transcript_mic,
            transcript_system: counters.transcript_system,
            detector_triggered: counters.detector_triggered,
            detector_skipped: counters.detector_skipped,
            ai_requests_total: counters.ai_requests_total,
            ai_responses_ok: counters.ai_responses_ok,
            ai_errors: counters.ai_errors,
            tiles_spawned: counters.tiles_spawned,
            rate_limited: counters.rate_limited,
            total_cost_microcents: counters.total_cost_microcents,
        });
    }
    j.write(&JournalEvent::SessionStop {
        unix_ms: now_unix_ms(),
    });
    j.close();
}

/// Manual ask from a specific source (mic or system) — grabs last 5 lines
/// from that source's transcript, asks AI, spawns tile. Bypasses detector.
pub async fn manual_ask_source(
    app: AppHandle,
    cfg: SharedConfig,
    rt: SharedRuntime,
    tiles: SharedTiles,
    source: AudioSource,
) {
    // Pull cross-source context: the trigger is the LAST line from the
    // requested source, but we feed the AI the last 8 lines from BOTH
    // speakers so it sees the back-and-forth. Without this, asking about
    // "почему?" from the interviewer loses the topic context entirely.
    let (recent, trigger_text): (Vec<String>, String) = {
        let s = rt.lock();
        let lines = select_recent_lines_labeled(&s.transcript, 8);
        let trigger = find_last_line_from_source(&s.transcript, source).unwrap_or_default();
        (lines, trigger)
    };
    if trigger_text.is_empty() {
        let _ = app.emit_to(
            "overlay",
            "tile:error",
            serde_json::json!({
                "message": format!("Транскрипт от {} пустой — нечего спросить",
                    if matches!(source, AudioSource::Mic) { "микрофона" } else { "system audio" })
            }),
        );
        return;
    }

    // Cost budget WARN (not block) — manual ask is user-initiated.
    let (cap_usd, current_micro) = (
        cfg.read().max_session_cost_usd,
        rt.lock().session_cost_microcents,
    );
    if let Some(reason) = over_cost_budget(cap_usd, current_micro) {
        let _ = app.emit_to(
            "overlay",
            "cost:cap-hit",
            serde_json::json!({ "reason": reason, "source": "manual_ask", "blocking": false }),
        );
    }

    let (base_url, bearer, model, response_language, meeting_context, preferred_monitor, stealth) = {
        let c = cfg.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.ai_model.clone(),
            c.response_language.clone(),
            c.meeting_context.clone(),
            c.tile_monitor_name.clone(),
            c.stealth_enabled,
        )
    };

    let trigger_for_prompt = Trigger::Question(trigger_text.clone());
    let (system_prompt, user_prompt) = build_auto_tile_prompts(
        &trigger_for_prompt,
        &recent,
        &meeting_context,
        &response_language,
    );

    let sys_full = system_prompt.clone();
    let usr_full = user_prompt.clone();
    let input_tokens_est = ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4;
    let messages = vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(system_prompt),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(user_prompt),
        },
    ];

    let journal = rt.lock().journal.clone().unwrap_or_default();
    let purpose = match source {
        AudioSource::System => "manual_ask_system",
        AudioSource::Mic => "manual_ask_mic",
    };
    journal.write(&JournalEvent::AiRequest {
        unix_ms: now_unix_ms(),
        purpose,
        model: &model,
        system_prompt: &sys_full,
        user_prompt: &usr_full,
        attached_screenshot: false,
        input_tokens_est,
    });
    let t0 = std::time::Instant::now();

    let (answer, usage) =
        match ai::complete_with_usage(&base_url, &bearer, &model, messages, 512).await {
            Ok(t) => {
                bump_health_ai(&rt);
                t
            }
            Err(e) => {
                log::warn!("manual_ask_source AI failed: {e:#}");
                journal.write(&JournalEvent::Error {
                    unix_ms: now_unix_ms(),
                    module: purpose,
                    message: &format!("{e:#}"),
                });
                return;
            }
        };
    let micro = ai::cost_microcents(&model, usage.input, usage.output);
    let total = {
        let mut s = rt.lock();
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    };
    let _ = app.emit_to(
        "overlay",
        "cost:update",
        serde_json::json!({ "session_usd": total }),
    );
    journal.write(&JournalEvent::AiResponse {
        unix_ms: now_unix_ms(),
        purpose,
        model: &model,
        latency_ms: t0.elapsed().as_millis() as u64,
        finish_reason: "stop",
        text: &answer,
        output_tokens_est: usage.output,
        cost_microcents: micro,
    });

    let icon = match source {
        AudioSource::System => "🔊",
        AudioSource::Mic => "🎤",
    };
    let question = format!("{icon} {trigger_text}");
    let kind = match source {
        AudioSource::System => crate::tile::TileKind::System,
        AudioSource::Mic => crate::tile::TileKind::Mic,
    };
    store_last_qa(&rt, &question, answer.trim());
    match crate::tile::spawn_tile_with_stealth(
        &app,
        &tiles,
        question.clone(),
        answer.trim().to_string(),
        preferred_monitor,
        stealth,
        kind,
    ) {
        Ok(label) => journal.write(&JournalEvent::TileSpawn {
            unix_ms: now_unix_ms(),
            label: &label,
            question: &question,
            answer: &answer,
        }),
        Err(e) => log::warn!("manual ask spawn_tile failed: {e:#}"),
    }
}

/// Push-to-talk START: open a DEDICATED WASAPI capture on the requested
/// source so the audio is recorded as one continuous blob (no VAD chunks,
/// no main-capture interference). The stop signal + samples-receiver are
/// stored in RuntimeState; manual_ask_window_end consumes them on release.
///
/// Returns the start timestamp (unix ms) for UI tracking.
pub fn manual_ask_window_start(rt: SharedRuntime, cfg: SharedConfig, source: AudioSource) -> u64 {
    let now = crate::journal::now_unix_ms() as u64;
    // If a previous PTT is still active, cancel it + JOIN its thread (with
    // bounded wait) so spamming the button doesn't leak WASAPI sessions.
    let cancel_old = rt.lock().push_to_talk.take();
    if let Some(old) = cancel_old {
        old.stop.store(true, Ordering::Release);
        if let Some(handle) = old.thread {
            // Capture loop polls stop every 500ms — wait briefly then move on.
            // We don't enforce a hard deadline; the WASAPI handle drops when
            // record_source_until_stop returns regardless.
            let _ = std::thread::Builder::new()
                .name("ptt-cancel-join".into())
                .spawn(move || {
                    let _ = handle.join();
                });
        }
    }

    let (mic_dev, sys_dev) = {
        let c = cfg.read();
        (c.mic_device.clone(), c.system_audio_device.clone())
    };

    let (samples_tx, samples_rx) = oneshot::channel::<Result<Vec<i16>, String>>();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let thread_result = std::thread::Builder::new()
        .name(format!("ptt-{:?}", source))
        .spawn(move || {
            let res =
                crate::audio::record_source_until_stop(source, mic_dev, sys_dev, stop_for_thread);
            let payload = res.map_err(|e| format!("{e:#}"));
            // Log on error so the surface is visible even if the receiver
            // dropped (race with rapid cancel).
            if let Err(ref msg) = payload {
                log::warn!("PTT capture failed for {:?}: {msg}", source);
            }
            let _ = samples_tx.send(payload);
        });

    let thread = match thread_result {
        Ok(h) => Some(h),
        Err(e) => {
            // Couldn't even spawn — surface as an immediate Err on the channel
            // (we have nothing to send to since the closure didn't run).
            // Caller's manual_ask_window_end will see no push_to_talk and warn.
            log::error!("PTT thread spawn failed: {e}");
            return now;
        }
    };

    // Single critical section: both insert + Some assignment under one lock
    // (previously two separate rt.lock() calls created a window where
    // manual_ask_window_end could see the timestamp but no PTT struct).
    {
        let mut s = rt.lock();
        s.manual_ask_start_ms.insert(source, now);
        s.push_to_talk = Some(PushToTalkCapture {
            source,
            start_ms: now,
            stop,
            samples_rx,
            thread,
        });
    }
    log::info!("PTT hold start: {:?} at {}", source, now);
    now
}

/// Push-to-talk END: signal stop to the dedicated capture thread, await
/// the full PCM blob, send as ONE WAV to Whisper (no VAD splitting →
/// no chunk-boundary artifacts → cleaner text), ask AI, spawn tile.
pub async fn manual_ask_window_end(
    app: AppHandle,
    cfg: SharedConfig,
    rt: SharedRuntime,
    tiles: SharedTiles,
    source: AudioSource,
) {
    // Take ownership of the PTT capture struct (releases lock immediately).
    let ptt = {
        let mut s = rt.lock();
        s.manual_ask_start_ms.remove(&source);
        s.push_to_talk.take()
    };
    let Some(ptt) = ptt else {
        log::warn!("PTT end for {:?} without matching start — ignored", source);
        return;
    };
    if ptt.source != source {
        log::warn!(
            "PTT end source mismatch: held={:?}, end={:?}",
            ptt.source,
            source
        );
        // Still consume the receiver so the thread doesn't leak.
    }

    let now = crate::journal::now_unix_ms() as u64;
    let duration_ms = now.saturating_sub(ptt.start_ms);
    log::info!(
        "PTT hold end: {:?} after {}ms — awaiting samples",
        ptt.source,
        duration_ms
    );

    // Signal stop and await samples. Channel carries Result so we can
    // surface the real WASAPI/COM failure to the UI instead of the prior
    // misleading "too short" message.
    ptt.stop.store(true, Ordering::Release);
    let samples = match ptt.samples_rx.await {
        Ok(Ok(s)) => s,
        Ok(Err(capture_err)) => {
            let _ = app.emit_to(
                "overlay",
                "tile:error",
                serde_json::json!({
                    "message": format!("Push-to-talk capture: {}", capture_err)
                }),
            );
            return;
        }
        Err(_) => {
            log::warn!("PTT samples_rx dropped — capture thread crashed");
            let _ = app.emit_to(
                "overlay",
                "tile:error",
                serde_json::json!({
                    "message": "Push-to-talk: capture thread crashed (см. лог)"
                }),
            );
            return;
        }
    };

    // Best-effort cleanup of the OS thread — it should already be exiting.
    if let Some(handle) = ptt.thread {
        let _ = std::thread::Builder::new()
            .name("ptt-end-join".into())
            .spawn(move || {
                let _ = handle.join();
            });
    }

    if samples.len() < (crate::audio::TARGET_SAMPLE_RATE as usize / 4) {
        // <250ms — too short to be meaningful speech.
        let _ = app.emit_to(
            "overlay",
            "tile:error",
            serde_json::json!({
                "message": format!("Push-to-talk: записано всего {}ms — удерживай дольше", duration_ms)
            }),
        );
        return;
    }
    // Pre-Whisper noise gate — same filter as always-on capture.
    if !crate::stt::buffer_likely_speech(&samples) {
        let _ = app.emit_to(
            "overlay",
            "tile:error",
            serde_json::json!({
                "message": "Push-to-talk: фон без речи — нечего распознавать"
            }),
        );
        return;
    }

    // Transcribe via DEDICATED Whisper call — one WAV, full context, no VAD chunks.
    let (groq_key, language, whisper_prompt, stt_model) = {
        let c = cfg.read();
        (
            c.groq_api_key.clone(),
            c.stt_language.clone(),
            crate::stt::build_whisper_prompt(&c.trigger_keywords, &c.meeting_context),
            c.stt_model.clone(),
        )
    };

    let journal = rt.lock().journal.clone().unwrap_or_default();
    let purpose = match source {
        AudioSource::System => "push_to_talk_system",
        AudioSource::Mic => "push_to_talk_mic",
    };

    let t_stt0 = std::time::Instant::now();
    let transcribed = match crate::stt::transcribe_once(
        &samples,
        &groq_key,
        language.as_deref(),
        whisper_prompt.as_deref(),
        &stt_model,
    )
    .await
    {
        Ok(t) => t.trim().to_string(),
        Err(e) => {
            log::warn!("PTT transcription failed: {e:#}");
            journal.write(&JournalEvent::Error {
                unix_ms: now_unix_ms(),
                module: "ptt_stt",
                message: &format!("{e:#}"),
            });
            let _ = app.emit_to(
                "overlay",
                "tile:error",
                serde_json::json!({ "message": format!("STT error: {}", e) }),
            );
            return;
        }
    };
    log::info!(
        "PTT transcribed in {}ms: '{}'",
        t_stt0.elapsed().as_millis(),
        transcribed.chars().take(80).collect::<String>()
    );

    if transcribed.is_empty() {
        let _ = app.emit_to(
            "overlay",
            "tile:error",
            serde_json::json!({ "message": "Push-to-talk: Whisper не услышал речи (тишина?)" }),
        );
        return;
    }
    // Post-Whisper hallucination filter — drop "subscribe to my channel",
    // repetition loops, punctuation-only output.
    if crate::stt::is_likely_hallucination(&transcribed) {
        log::info!(
            "PTT dropped hallucination: '{}'",
            transcribed.chars().take(80).collect::<String>()
        );
        let _ = app.emit_to(
            "overlay",
            "tile:error",
            serde_json::json!({
                "message": format!("Push-to-talk: распознанное похоже на галлюцинацию Whisper («{}»)",
                    transcribed.chars().take(60).collect::<String>())
            }),
        );
        return;
    }

    // Emit the PTT transcript as a synthetic transcript line so it shows
    // up in the journal AND in the UI tail.
    journal.write(&JournalEvent::TranscriptLine {
        unix_ms: now_unix_ms(),
        source: match source {
            AudioSource::System => "system",
            AudioSource::Mic => "mic",
        },
        text: &transcribed,
    });
    let _ = app.emit_to(
        "overlay",
        "transcript:line",
        &TranscriptLine {
            source,
            text: transcribed.clone(),
            timestamp_ms: now,
        },
    );

    // Build AI prompt — only the freshly-transcribed text, plus a short
    // labeled context from the still-rolling main transcript for situational awareness.
    let (base_url, bearer, model, response_language, meeting_context, preferred_monitor, stealth) = {
        let c = cfg.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.ai_model.clone(),
            c.response_language.clone(),
            c.meeting_context.clone(),
            c.tile_monitor_name.clone(),
            c.stealth_enabled,
        )
    };
    let recent_context = select_recent_lines_labeled(&rt.lock().transcript, 5);
    let mut labeled = recent_context.clone();
    let ptt_label = match source {
        AudioSource::System => format!("[СОБЕСЕДНИК ⏺] {}", transcribed),
        AudioSource::Mic => format!("[ПОЛЬЗОВАТЕЛЬ ⏺] {}", transcribed),
    };
    labeled.push(ptt_label);

    let trigger_for_prompt = Trigger::Question(transcribed.clone());
    let (system_prompt, user_prompt) = build_auto_tile_prompts(
        &trigger_for_prompt,
        &labeled,
        &meeting_context,
        &response_language,
    );

    let sys_full = system_prompt.clone();
    let usr_full = user_prompt.clone();
    let input_tokens_est = ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4;
    let messages = vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(system_prompt),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(user_prompt),
        },
    ];

    journal.write(&JournalEvent::AiRequest {
        unix_ms: now_unix_ms(),
        purpose,
        model: &model,
        system_prompt: &sys_full,
        user_prompt: &usr_full,
        attached_screenshot: false,
        input_tokens_est,
    });
    let t0 = std::time::Instant::now();

    let (answer, usage) =
        match ai::complete_with_usage(&base_url, &bearer, &model, messages, 512).await {
            Ok(t) => {
                bump_health_ai(&rt);
                t
            }
            Err(e) => {
                log::warn!("PTT AI failed: {e:#}");
                journal.write(&JournalEvent::Error {
                    unix_ms: now_unix_ms(),
                    module: purpose,
                    message: &format!("{e:#}"),
                });
                return;
            }
        };
    let micro = ai::cost_microcents(&model, usage.input, usage.output);
    let total = {
        let mut s = rt.lock();
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    };
    let _ = app.emit_to(
        "overlay",
        "cost:update",
        serde_json::json!({ "session_usd": total }),
    );
    journal.write(&JournalEvent::AiResponse {
        unix_ms: now_unix_ms(),
        purpose,
        model: &model,
        latency_ms: t0.elapsed().as_millis() as u64,
        finish_reason: "stop",
        text: &answer,
        output_tokens_est: usage.output,
        cost_microcents: micro,
    });

    let icon = match source {
        AudioSource::System => "🔊⏺",
        AudioSource::Mic => "🎤⏺",
    };
    let snippet: String = transcribed.chars().take(80).collect();
    let question = format!("{icon} {snippet}");
    let kind = match source {
        AudioSource::System => crate::tile::TileKind::System,
        AudioSource::Mic => crate::tile::TileKind::Mic,
    };
    store_last_qa(&rt, &question, answer.trim());
    match crate::tile::spawn_tile_with_stealth(
        &app,
        &tiles,
        question.clone(),
        answer.trim().to_string(),
        preferred_monitor,
        stealth,
        kind,
    ) {
        Ok(label) => journal.write(&JournalEvent::TileSpawn {
            unix_ms: now_unix_ms(),
            label: &label,
            question: &question,
            answer: &answer,
        }),
        Err(e) => log::warn!("PTT spawn_tile failed: {e:#}"),
    }
}

/// F6 Manual spawn tile: thin Tauri shim around
/// `overlay_backend::runtime::manual_spawn_tile`.
///
/// Phase B2 port #3: body moved to overlay-backend. This shim follows
/// the same snapshot-and-writeback pattern established by port #2
/// (reask_last):
///   1. Snapshot SharedRuntime + cost-cap check under one rt lock.
///   2. Drop lock.
///   3. Construct TauriEvents + call ported async fn.
///   4. On success-outcome: re-acquire rt lock to apply session-cost
///      add + store_last_qa, then emit cost:update with new total.
pub async fn manual_spawn_tile(
    app: AppHandle,
    cfg: SharedConfig,
    rt: SharedRuntime,
    tiles: SharedTiles,
) {
    let inputs = {
        let s = rt.lock();
        let recent = select_recent_lines_labeled(&s.transcript, 8);
        let last_line = s.transcript.back().cloned();
        let cap_usd = cfg.read().max_session_cost_usd;
        let cost_cap_reason = over_cost_budget(cap_usd, s.session_cost_microcents);
        overlay_backend::runtime::ManualSpawnInputs {
            recent_transcript_labeled: recent,
            last_line,
            cost_cap_reason,
            journal: s.journal.clone(),
            health: s.health.clone(),
        }
    };

    let events: Arc<dyn overlay_backend::events::RuntimeEvents> = Arc::new(crate::TauriEvents {
        app: app.clone(),
        tiles: tiles.clone(),
    });

    let outcome = overlay_backend::runtime::manual_spawn_tile(events, cfg, inputs).await;

    if let Some(out) = outcome {
        let total = {
            let mut s = rt.lock();
            s.session_cost_microcents = s
                .session_cost_microcents
                .saturating_add(out.cost_microcents_delta);
            s.last_question = Some(out.display_question);
            s.last_answer = Some(out.answer_trimmed);
            (s.session_cost_microcents as f64) / 100_000_000.0
        };
        let _ = app.emit_to(
            "overlay",
            "cost:update",
            serde_json::json!({ "session_usd": total }),
        );
    }
}

/// F9 Live Ask: thin Tauri shim around
/// `overlay_backend::runtime::ask_stream_loop`.
///
/// Phase B2 port #4: streaming body moved to overlay-backend. This
/// shim:
///   1. Snapshots cfg + transcript + screenshot under rt locks.
///   2. Emits `cost:cap-hit` (non-blocking warn) if over budget.
///   3. Writes JournalEvent::AiRequest (sync, pre-stream).
///   4. Starts ai::stream_chat → gets ai_rx Receiver.
///   5. Cancels any in-flight ai_task (rapid-F9 protection).
///   6. Builds the cost-mutation closure (captures rt).
///   7. tokio::spawns `ask_stream_loop` + stores JoinHandle in rt.
///
/// MUST be called from a Tokio runtime context.
pub async fn ask(app: AppHandle, cfg: SharedConfig, rt: SharedRuntime) {
    let (base_url, bearer, model, meeting_context, response_language, cap_usd) = {
        let c = cfg.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.ai_model.clone(),
            c.meeting_context.clone(),
            c.response_language.clone(),
            c.max_session_cost_usd,
        )
    };

    // Cost budget WARN (not block) — F9 is user-initiated, blocking it
    // mid-interview defeats the entire point. Emit the warn chip so the
    // user sees they crossed budget but proceed with the ask.
    let current_micro = rt.lock().session_cost_microcents;
    if let Some(reason) = over_cost_budget(cap_usd, current_micro) {
        let _ = app.emit_to(
            "overlay",
            "cost:cap-hit",
            serde_json::json!({ "reason": reason, "source": "live_ask", "blocking": false }),
        );
    }

    let (transcript_lines, screenshot) = {
        let mut s = rt.lock();
        let lines: Vec<String> = s
            .transcript
            .iter()
            .map(|l| format!("[{:?}] {}", l.source, l.text))
            .collect();
        let shot = s.last_screenshot.take(); // consume
        (lines, shot)
    };

    let messages = ai::build_request(
        &meeting_context,
        &response_language,
        &transcript_lines,
        screenshot.as_deref(),
        None,
    );

    let (journal_for_request, journal_for_loop, health_for_stream) = {
        let s = rt.lock();
        // Pre-port code: `rt.lock().journal.clone().unwrap_or_default()`.
        // We pass the Option through so the port can `if let Some(j)`
        // (Journal::default() write is a no-op anyway — wire-equivalent
        // but the explicit Option is cleaner per port #2/#3 pattern).
        let j = s.journal.clone();
        (j.clone(), j, s.health.clone())
    };
    let sys_full = messages
        .first()
        .and_then(|m| match &m.content {
            ai::MessageContent::Text(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let usr_full = match messages.get(1).map(|m| &m.content) {
        Some(ai::MessageContent::Text(t)) => t.clone(),
        Some(ai::MessageContent::Parts(parts)) => parts
            .iter()
            .find_map(|p| match p {
                ai::ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        None => String::new(),
    };
    let input_tokens_est = ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4;
    if let Some(j) = journal_for_request.as_ref() {
        j.write(&JournalEvent::AiRequest {
            unix_ms: now_unix_ms(),
            purpose: "live_ask",
            model: &model,
            system_prompt: &sys_full,
            user_prompt: &usr_full,
            attached_screenshot: screenshot.is_some(),
            input_tokens_est,
        });
    }

    let ai_rx = ai::stream_chat(base_url, bearer, model.clone(), messages, 4096);

    // Cancel any in-flight ask before spawning a new one — otherwise rapid
    // F9 presses stack responses on top of each other.
    {
        let mut s = rt.lock();
        if let Some(h) = s.ai_task.take() {
            h.abort();
        }
    }

    let t0 = std::time::Instant::now();
    let rt_for_cost = rt.clone();
    // FnOnce closure: lock rt, accumulate session_cost, return new
    // total in USD. The port calls this once at end-of-stream then
    // emits cost:update with the returned value — preserves the
    // pre-port mutate-then-emit ordering.
    let cost_apply: overlay_backend::runtime::CostApplyFn = Box::new(move |micro| {
        let mut s = rt_for_cost.lock();
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    });

    // Fetch the real SharedTiles from Tauri state so the adapter's
    // spawn_tile_full path stays sane if any future port adds a tile
    // spawn here. ask_stream_loop itself doesn't spawn tiles today,
    // but using the real registry future-proofs the wiring.
    use tauri::Manager as _;
    let tiles = app.state::<SharedTiles>().inner().clone();
    let events: Arc<dyn overlay_backend::events::RuntimeEvents> = Arc::new(crate::TauriEvents {
        app: app.clone(),
        tiles,
    });

    let task = tokio::spawn(overlay_backend::runtime::ask_stream_loop(
        events,
        ai_rx,
        model,
        sys_full,
        usr_full,
        journal_for_loop,
        health_for_stream,
        t0,
        cost_apply,
    ));
    rt.lock().ai_task = Some(task);
}

/// Store a screenshot for use on the next ask.
pub fn stash_screenshot(rt: SharedRuntime, data_url: String) {
    rt.lock().last_screenshot = Some(data_url);
}

/// Read snapshot of current transcript (for UI fetch / debugging).
pub fn snapshot_transcript(rt: &SharedRuntime) -> Vec<TranscriptLine> {
    rt.lock().transcript.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HealthSignals tests (2nd-pass review identified gap) ────────
    //
    // Phase B1: classify_thresholds test moved with HealthSignals to
    // overlay-backend/src/health.rs (the `classify` fn is private to
    // that module). snapshot tests stay here since they exercise the
    // public API + integration with crate::journal::now_unix_ms().

    /// snapshot() with all atomics zeroed = "idle" for every subsystem.
    /// This is the post-`stop_session` / post-zero state.
    #[test]
    fn health_snapshot_all_idle_when_zeroed() {
        let h = HealthSignals::default();
        let snap = h.snapshot(crate::journal::now_unix_ms() as u64);
        assert_eq!(snap.audio, "idle");
        assert_eq!(snap.stt, "idle");
        assert_eq!(snap.ai, "idle");
        assert!(snap.audio_age_ms.is_none());
        assert!(snap.stt_age_ms.is_none());
        assert!(snap.ai_age_ms.is_none());
    }

    /// After bumping all three to "now", snapshot at the same instant
    /// shows age ≈ 0 → "ok" for each.
    #[test]
    fn health_snapshot_all_ok_after_recent_bumps() {
        let h = HealthSignals::default();
        let now = crate::journal::now_unix_ms() as u64;
        h.last_audio_frame_ms.store(now, Ordering::Relaxed);
        h.last_stt_ok_ms.store(now, Ordering::Relaxed);
        h.last_ai_ok_ms.store(now, Ordering::Relaxed);
        let snap = h.snapshot(now);
        assert_eq!(snap.audio, "ok");
        assert_eq!(snap.stt, "ok");
        assert_eq!(snap.ai, "ok");
        assert_eq!(snap.audio_age_ms, Some(0));
        assert_eq!(snap.stt_age_ms, Some(0));
        assert_eq!(snap.ai_age_ms, Some(0));
    }

    /// Per-subsystem thresholds differ (audio strict 15s/60s; ai loose
    /// 180s/600s). Verify a "20s old" signal classifies as `degraded`
    /// for audio but still `ok` for ai.
    #[test]
    fn health_snapshot_per_subsystem_thresholds_differ() {
        let h = HealthSignals::default();
        let now = 1_000_000u64;
        // 20s ago = now - 20_000
        let twenty_s_ago = now - 20_000;
        h.last_audio_frame_ms.store(twenty_s_ago, Ordering::Relaxed);
        h.last_stt_ok_ms.store(twenty_s_ago, Ordering::Relaxed);
        h.last_ai_ok_ms.store(twenty_s_ago, Ordering::Relaxed);
        let snap = h.snapshot(now);
        // audio 15s/60s → 20s = degraded
        assert_eq!(snap.audio, "degraded");
        // stt 60s/180s → 20s = still ok
        assert_eq!(snap.stt, "ok");
        // ai 180s/600s → 20s = still ok
        assert_eq!(snap.ai, "ok");
    }

    /// `store_last_qa` writes both fields atomically (under the same
    /// mutex). Reading immediately after must see both new values, not
    /// a torn write (Some(new q) + None a) or vice versa.
    #[test]
    fn store_last_qa_writes_both_atomically() {
        let rt = shared();
        store_last_qa(&rt, "Q1", "A1");
        let s = rt.lock();
        assert_eq!(s.last_question.as_deref(), Some("Q1"));
        assert_eq!(s.last_answer.as_deref(), Some("A1"));
        drop(s);
        // Overwrite — both update.
        store_last_qa(&rt, "Q2", "A2");
        let s2 = rt.lock();
        assert_eq!(s2.last_question.as_deref(), Some("Q2"));
        assert_eq!(s2.last_answer.as_deref(), Some("A2"));
    }

    /// bump_health_ai writes a fresh timestamp; classify next snapshot ok.
    #[test]
    fn bump_health_ai_updates_atomic() {
        let rt = shared();
        let before = rt.lock().health.last_ai_ok_ms.load(Ordering::Relaxed);
        assert_eq!(before, 0);
        bump_health_ai(&rt);
        let after = rt.lock().health.last_ai_ok_ms.load(Ordering::Relaxed);
        assert!(after > 0, "bump_health_ai should write current unix ms");
    }

    // ── Prompt-builder tests moved to overlay_backend::runtime ─────
    // Phase B2 port #2 relocated build_auto_tile_prompts + Trigger
    // to overlay-backend; the 7 robustness tests live there now
    // (search `prompt_always_contains_injection_guard` in
    // overlay-backend/src/runtime.rs). detect_trigger tests stay
    // below because detect_trigger itself stays in src-tauri.

    #[test]
    fn detect_question_mark() {
        assert!(matches!(
            detect_trigger("Как у тебя с Kubernetes?", "etcd"),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn detect_keyword() {
        match detect_trigger("Мы используем etcd для consensus", "kubernetes etcd istio")
        {
            Some(Trigger::Keyword(kw, _)) => assert_eq!(kw, "etcd"),
            _ => panic!("expected keyword trigger"),
        }
    }

    #[test]
    fn ignore_short_text() {
        assert!(detect_trigger("ok", "kubernetes").is_none());
    }

    #[test]
    fn ignore_normal_statement() {
        assert!(detect_trigger("Сегодня хорошая погода", "kubernetes").is_none());
    }

    // ── KB key tokenisation match (regression for hyphenated keys) ──

    #[test]
    fn kb_match_single_token_key() {
        assert!(kb_key_matches_trigger(
            "kubernetes",
            "Какой-нибудь Kubernetes?"
        ));
        assert!(kb_key_matches_trigger(
            "etcd",
            "Расскажи как работает etcd внутри"
        ));
    }

    #[test]
    fn kb_match_hyphenated_key_requires_all_tokens() {
        // The bug: previously "kubectl-debug" was checked via HashSet contains
        // on the literal "kubectl-debug" string — but the trigger tokeniser
        // stripped hyphens, so the key never matched. Fix verifies both halves.
        assert!(kb_key_matches_trigger(
            "kubectl-debug",
            "как сделать kubectl debug на упавшем поде",
        ));
        // Both tokens of the key must appear; one missing → no match.
        assert!(!kb_key_matches_trigger(
            "kubectl-debug",
            "kubectl plan apply"
        ));
        assert!(!kb_key_matches_trigger("kubectl-debug", "debug a pod"));
    }

    #[test]
    fn kb_match_is_case_insensitive() {
        assert!(kb_key_matches_trigger("Kubernetes", "kubernetes pods"));
        assert!(kb_key_matches_trigger("kubernetes", "KUBERNETES POD"));
        assert!(kb_key_matches_trigger(
            "Git-Recovery",
            "git recovery please"
        ));
    }

    #[test]
    fn kb_match_empty_inputs_dont_panic_or_match() {
        assert!(!kb_key_matches_trigger("", "anything"));
        assert!(!kb_key_matches_trigger("kubernetes", ""));
        assert!(!kb_key_matches_trigger("", ""));
        // Punctuation-only key tokens collapse to zero entry tokens → no match.
        assert!(!kb_key_matches_trigger("---", "kubernetes"));
    }

    #[test]
    fn kb_match_partial_substring_doesnt_count() {
        // "kuber" appearing inside "kubernetes" should NOT trigger a key=kuber match.
        // The tokeniser splits on word boundaries, not substrings.
        assert!(!kb_key_matches_trigger("kuber", "kubernetes pods"));
    }

    // ── Voice coach: filler + word counting ──

    #[test]
    fn count_fillers_single_word_matches_whole_word() {
        assert_eq!(count_fillers("ну вот значит мы делаем kubernetes"), 3);
        assert_eq!(count_fillers("просто работаем без фillerов"), 0);
        // Substring shouldn't match — "значительно" contains "значит" but is
        // a legitimate word, not a filler.
        assert_eq!(count_fillers("это значительно лучше"), 0);
    }

    #[test]
    fn count_fillers_case_insensitive() {
        assert_eq!(count_fillers("НУ типа ВОТ"), 3);
    }

    #[test]
    fn count_fillers_multi_word() {
        assert_eq!(count_fillers("мы как бы делаем это в общем нормально"), 2);
        assert_eq!(count_fillers("это самое надо как бы понять"), 2);
    }

    #[test]
    fn count_fillers_multiple_in_one_line() {
        assert_eq!(count_fillers("ну ну ну блин"), 4);
    }

    #[test]
    fn count_words_basic() {
        assert_eq!(count_words("привет как дела"), 3);
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("один,два!три"), 3);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("   "), 0);
    }

    #[test]
    fn speech_coach_idle_when_window_empty() {
        let rt = crate::runtime::shared();
        let snap = snapshot_speech_coach(&rt, 1_700_000_000_000);
        assert_eq!(snap.pace, "idle");
        assert_eq!(snap.words_60s, 0);
        assert_eq!(snap.fillers_60s, 0);
        assert!(snap.wpm.is_none());
        assert!(snap.filler_per_100.is_none());
    }

    #[test]
    fn speech_coach_aggregates_window_and_trims_old_entries() {
        let rt = crate::runtime::shared();
        let now: u64 = 1_700_000_000_000;
        // Old entry (>60s back) — should get trimmed on next push.
        push_speech_window(&rt, now - 70_000, 50, 5);
        // Recent: 90 words 0-60s window
        push_speech_window(&rt, now - 30_000, 60, 4);
        push_speech_window(&rt, now - 1_000, 30, 2);
        let snap = snapshot_speech_coach(&rt, now);
        // Old should be trimmed because last push trimmed below 60s cutoff.
        assert_eq!(snap.words_60s, 90, "old entry should have been trimmed");
        assert_eq!(snap.fillers_60s, 6);
        assert!(snap.wpm.is_some());
        assert_eq!(snap.filler_per_100, Some(6)); // 6 / 90 * 100 ≈ 6
    }

    #[test]
    fn speech_coach_below_min_words_returns_none_for_density_and_wpm() {
        let rt = crate::runtime::shared();
        let now: u64 = 1_700_000_000_000;
        push_speech_window(&rt, now - 2_000, 3, 1); // <5s span, <5 words
        let snap = snapshot_speech_coach(&rt, now);
        assert!(snap.wpm.is_none(), "insufficient data shouldn't report WPM");
        assert!(
            snap.filler_per_100.is_none(),
            "insufficient data shouldn't report density"
        );
        assert_eq!(snap.pace, "idle");
    }

    #[test]
    fn speech_coach_pace_buckets() {
        let rt = crate::runtime::shared();
        let now: u64 = 1_700_000_000_000;
        // 60 words across 60s = 60 WPM → low
        push_speech_window(&rt, now - 60_000, 60, 0);
        let snap = snapshot_speech_coach(&rt, now);
        assert_eq!(
            snap.pace, "low",
            "60 WPM should be 'low'; got {:?}",
            snap.wpm
        );
    }

    #[test]
    fn speech_coach_fast_pace() {
        let rt = crate::runtime::shared();
        let now: u64 = 1_700_000_000_000;
        // 250 words across 60s = 250 WPM → fast
        push_speech_window(&rt, now - 60_000, 250, 0);
        let snap = snapshot_speech_coach(&rt, now);
        assert_eq!(snap.pace, "fast");
    }

    // ── Post-meeting debrief gate ──

    #[test]
    fn debrief_runs_on_normal_session() {
        assert_eq!(should_run_debrief(true, 60_000, 10, 200, true), Ok(()));
        assert_eq!(should_run_debrief(true, 1_000_000, 80, 4_000, true), Ok(()));
    }

    #[test]
    fn debrief_skips_when_disabled() {
        assert_eq!(
            should_run_debrief(false, 60_000, 10, 200, true),
            Err("disabled by config")
        );
    }

    #[test]
    fn debrief_skips_short_session() {
        assert_eq!(
            should_run_debrief(true, 29_999, 10, 200, true),
            Err("session too short (<30s)")
        );
        // Exactly 30s boundary is included → not skipped on duration.
        assert!(should_run_debrief(true, 30_000, 10, 200, true).is_ok());
    }

    #[test]
    fn debrief_skips_thin_mic_history() {
        assert_eq!(
            should_run_debrief(true, 60_000, 4, 200, true),
            Err("fewer than 5 mic lines")
        );
        // 5 is the inclusive floor.
        assert!(should_run_debrief(true, 60_000, 5, 200, true).is_ok());
    }

    #[test]
    fn debrief_skips_tiny_mic_text() {
        // 5 mic lines but each only "ok" / "ага" — < 50 chars total — Sonnet
        // can't produce 3 useful observations. Gate at the should_run layer
        // so we don't even spawn the AI call (P0-2 fix from review).
        assert_eq!(
            should_run_debrief(true, 60_000, 5, 49, true),
            Err("mic transcript too short (<50 chars)")
        );
        // 50 is the inclusive floor.
        assert!(should_run_debrief(true, 60_000, 5, 50, true).is_ok());
    }

    #[test]
    fn debrief_skips_when_no_bearer() {
        assert_eq!(
            should_run_debrief(true, 60_000, 10, 200, false),
            Err("no AI bearer configured")
        );
    }

    // ── Detector skip-mic gate ──

    #[test]
    fn detector_default_allows_both_sources() {
        // When skip_mic=false (legacy v0.0.1 behaviour), both sources
        // feed the detector.
        assert!(detector_allows(AudioSource::Mic, false));
        assert!(detector_allows(AudioSource::System, false));
    }

    #[test]
    fn detector_skip_mic_blocks_only_mic() {
        // When skip_mic=true (v0.0.2+ default), mic is filtered out.
        // System is unaffected — interviewer questions still spawn tiles.
        assert!(!detector_allows(AudioSource::Mic, true));
        assert!(detector_allows(AudioSource::System, true));
    }

    /// Regression for live bug #96: candidate said "Я работал с Kubernetes"
    /// and a redundant explanation tile spawned. detector_skip_mic should
    /// prevent that exact scenario.
    #[test]
    fn detector_regression_candidate_voice_no_spawn() {
        let source = AudioSource::Mic; // candidate speaking
        let cfg_skip_mic = true; // default v0.0.2+
        assert!(
            !detector_allows(source, cfg_skip_mic),
            "candidate's own mic line must NOT trigger detector when skip_mic=true"
        );
    }

    // ── Cost budget (soft warning, never blocks) ──

    #[test]
    fn cost_budget_disabled_when_zero_or_negative() {
        assert!(over_cost_budget(0.0, u64::MAX).is_none());
        assert!(over_cost_budget(-1.0, 1_000_000_000).is_none());
    }

    #[test]
    fn cost_budget_silent_under_threshold() {
        // 50¢ < $1.00 — no warn.
        assert!(over_cost_budget(1.00, 50_000_000).is_none());
        // Exactly $0 spent — silent.
        assert!(over_cost_budget(1.00, 0).is_none());
    }

    #[test]
    fn cost_budget_warns_at_or_above_threshold() {
        // 1¢ over → warn message.
        let r = over_cost_budget(1.00, 101_000_000);
        assert!(r.is_some());
        let msg = r.unwrap();
        assert!(msg.contains("over budget"), "got: {msg}");
        assert!(
            msg.contains("$1.01") || msg.contains("$1.0100"),
            "shows current spend; got: {msg}"
        );
    }

    #[test]
    fn cost_budget_warn_at_exact_boundary() {
        // At $1.00 == $1.00 → warn (≥ comparison).
        assert!(over_cost_budget(1.00, 100_000_000).is_some());
    }

    #[test]
    fn debrief_skip_priority_order() {
        // If multiple conditions fail, "disabled" wins (first check) — that
        // way the log message tells the user the simplest fix first.
        assert_eq!(
            should_run_debrief(false, 1_000, 0, 0, false),
            Err("disabled by config")
        );
        // With enabled=true the next failure (duration) takes priority over
        // the mic-lines check.
        assert_eq!(
            should_run_debrief(true, 1_000, 0, 0, false),
            Err("session too short (<30s)")
        );
    }

    // ── v2 tests for expanded detector ──

    #[test]
    fn detect_rasskazi_anywhere() {
        // Live interview pattern — request verb embedded in sentence.
        assert!(matches!(
            detect_trigger("А вот расскажи как ты диагностировал бы такое", "etcd"),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn detect_dopustim_scenario() {
        // Hypothetical scenario opener — DevOps interview favourite.
        assert!(matches!(
            detect_trigger("Допустим у тебя падает сервис в продакшене", ""),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn skip_kak_in_middle_of_statement() {
        // v3 detector: "как" in middle is usually a conjunction in a statement.
        // Live test showed "Мне интересно как ты..." was actually a statement
        // by the candidate, not a question. Now correctly skipped.
        assert!(
            detect_trigger("Мне интересно как ты будешь это решать", "").is_none(),
            "v3: 'как' in middle should not trigger (it's a conjunction here)"
        );
    }

    #[test]
    fn detect_s_chego_nachnesh() {
        // Has '?', triggers via step 1.
        assert!(matches!(
            detect_trigger("С чего начнёшь дебагать?", ""),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn detect_english_at_start() {
        // English interrogative at start (after fillers).
        assert!(matches!(
            detect_trigger("How would you debug this", ""),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn skip_english_how_in_middle() {
        // "how" mid-sentence as relative pronoun, not question.
        assert!(
            detect_trigger("I know how this works", "").is_none(),
            "v3: 'how' mid-sentence should not trigger"
        );
    }

    // ── v3 regression tests for false positives found in live test #92 ──

    #[test]
    fn skip_kogda_as_temporal_conjunction() {
        // Live FP: "А когда он загрузился, ему мастер-сервер отдает все параметры."
        // 'когда' here = temporal "when", not interrogative.
        assert!(
            detect_trigger(
                "А когда он загрузился, ему мастер-сервер отдает все параметры",
                ""
            )
            .is_none(),
            "v3: 'когда' as temporal conjunction should not trigger"
        );
    }

    #[test]
    fn skip_chto_as_subordinate_conjunction() {
        // Live FP: "Заходишь в команды, ну, в смысле, что у него творится"
        // 'что' here = subordinate conjunction, not interrogative.
        assert!(
            detect_trigger("Заходишь в команды, ну, в смысле, что у него творится", "").is_none(),
            "v3: 'что' as subordinate conjunction should not trigger"
        );
    }

    #[test]
    fn skip_chto_as_relative_in_explanation() {
        // Live FP: "Используется, что собирается ISO-шник"
        assert!(
            detect_trigger(
                "Там такое же самое используется, что собирается ISO-шник",
                ""
            )
            .is_none(),
            "v3: 'что собирается' = relative clause, not a question"
        );
    }

    #[test]
    fn detect_real_question_with_fillers_at_start() {
        // Real interviewer pattern with filler prefix.
        assert!(matches!(
            detect_trigger("Ну а что такое LVM?", ""),
            Some(Trigger::Question(_))
        ));
        // Even without ?
        assert!(matches!(
            detect_trigger("А как работает kubernetes", ""),
            Some(Trigger::Question(_))
        ));
    }

    /// Detector v5: short `?` utterances suppressed.
    /// Live-test 2026-05-25 caught false-positive "Какой-нибудь Kubernetes?"
    /// (2 words + ?, fragment not real question) firing AI + tile.
    /// New min-word gate = 4 words for `?` triggers.
    #[test]
    fn detect_short_question_mark_suppressed() {
        // 2 words + ? → suppressed.
        assert!(
            detect_trigger("Какой-нибудь Kubernetes?", "").is_none(),
            "2-word + ? fragment should not trigger"
        );
        // 3 words + ? → still suppressed (borderline; we err strict).
        assert!(
            detect_trigger("А этот sshd?", "").is_none(),
            "3-word + ? should be suppressed by new gate"
        );
        // 4 words + ? → fires.
        assert!(matches!(
            detect_trigger("Что такое k8s namespace?", ""),
            Some(Trigger::Question(_))
        ));
        // Long realistic interview question still fires.
        assert!(matches!(
            detect_trigger("Расскажи как ты настраивал репликацию postgres?", ""),
            Some(Trigger::Question(_))
        ));
    }

    /// Detector v4 — meta-question patterns like "давай спросим" / "давай обсудим".
    /// These signal the interviewer is about to ask but don't end with '?'.
    /// Without explicit triggers, detector v3 missed them entirely.
    #[test]
    fn detect_davai_sprosim_meta_question() {
        assert!(matches!(
            detect_trigger("Давай спросим как ты диагностировал кластер", ""),
            Some(Trigger::Question(_))
        ));
        assert!(matches!(
            detect_trigger("давай обсудим вопрос про репликацию", ""),
            Some(Trigger::Question(_))
        ));
        assert!(matches!(
            detect_trigger("Поговорим про твой опыт с istio", ""),
            Some(Trigger::Question(_))
        ));
        // With filler prefix.
        assert!(matches!(
            detect_trigger("Ну давай разберём такую штуку", ""),
            Some(Trigger::Question(_))
        ));
    }

    /// REGRESSION: meta-question triggers must NOT fire on candidate's own
    /// reply ("давай я расскажу" — a statement-of-intent, not a question).
    /// Currently "давай я" doesn't match any pattern, but a future loosening
    /// of "давай" alone would regress.
    #[test]
    fn detect_davai_alone_not_trigger() {
        // Just "давай" without one of the v4 meta-verbs should NOT fire.
        assert!(
            detect_trigger("давай я попробую объяснить так", "").is_none(),
            "bare 'давай' should not trigger — only 'давай {{спросим|обсудим|разберём}}' patterns"
        );
    }

    #[test]
    fn detect_request_verb_with_fillers() {
        // "А вот расскажи..." — strip "а вот" filler → "расскажи..." → trigger.
        assert!(matches!(
            detect_trigger("А вот расскажи как ты диагностировал бы такое", ""),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn strip_filler_handles_stacked_fillers() {
        assert_eq!(strip_filler_prefix("ну а что такое pod?"), "что такое pod?");
        assert_eq!(strip_filler_prefix("так вот, расскажи..."), "расскажи...");
        assert_eq!(strip_filler_prefix("кубернетес"), "кубернетес"); // no fillers
        assert_eq!(strip_filler_prefix("..., а ну как?"), "как?");
    }

    #[test]
    fn detect_short_clarification() {
        // 5-char minimum — should catch short clarifications.
        assert!(matches!(
            detect_trigger("что это?", ""),
            Some(Trigger::Question(_))
        ));
    }

    #[test]
    fn keyword_devops_basics_match() {
        let keywords = "kubernetes nginx prometheus";
        // Mention of nginx as keyword.
        match detect_trigger("мы используем nginx в качестве reverse proxy", keywords)
        {
            Some(Trigger::Keyword(kw, _)) => assert_eq!(kw, "nginx"),
            other => panic!("expected nginx keyword, got {other:?}"),
        }
    }

    #[test]
    fn still_ignore_pure_statement_without_triggers() {
        // No question marker, no keyword → still ignored even with v2 detector.
        assert!(detect_trigger("Я согласен с этим подходом полностью", "kubernetes").is_none());
    }

    // ── Noise filter tests ──

    #[test]
    fn noise_filter_passes_real_question() {
        assert!(looks_like_real_speech("Расскажи как работает kubernetes"));
        assert!(looks_like_real_speech("Что такое LVM?"));
        assert!(looks_like_real_speech("Tell me about etcd"));
    }

    #[test]
    fn noise_filter_rejects_too_few_words() {
        assert!(!looks_like_real_speech("ok"));
        assert!(!looks_like_real_speech("да!"));
        assert!(!looks_like_real_speech("в как")); // 2 short tokens, both <3 chars
    }

    #[test]
    fn noise_filter_rejects_punctuation_spam() {
        assert!(!looks_like_real_speech(".......!!!,,,;;;"));
    }

    #[test]
    fn noise_filter_rejects_repeated_single_word() {
        assert!(!looks_like_real_speech("угу угу угу угу угу"));
        assert!(!looks_like_real_speech("ага ага ага"));
    }

    #[test]
    fn noise_filter_accepts_normal_speech_with_punct() {
        // Plenty of alnum chars even with commas/dots
        assert!(looks_like_real_speech(
            "Ну вот, допустим, у нас есть кластер."
        ));
    }

    #[test]
    fn detector_skips_noise_via_filter() {
        // Garbage that would otherwise match keyword "etcd" if we didn't filter
        assert!(detect_trigger("....,,,;", "etcd").is_none());
        assert!(detect_trigger("угу угу угу", "etcd").is_none());
    }

    // ── select_recent_lines_from_source — extracted from manual_ask_source ──

    fn mk_line(source: AudioSource, text: &str) -> TranscriptLine {
        TranscriptLine {
            source,
            text: text.to_string(),
            timestamp_ms: 0,
        }
    }

    #[test]
    fn select_lines_filters_by_source_preserves_order() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::Mic, "a"));
        t.push_back(mk_line(AudioSource::System, "b"));
        t.push_back(mk_line(AudioSource::Mic, "c"));
        t.push_back(mk_line(AudioSource::System, "d"));

        let mic = select_recent_lines_from_source(&t, AudioSource::Mic, 10);
        assert_eq!(mic, vec!["a".to_string(), "c".into()]);

        let sys = select_recent_lines_from_source(&t, AudioSource::System, 10);
        assert_eq!(sys, vec!["b".to_string(), "d".into()]);
    }

    #[test]
    fn select_lines_respects_max_takes_tail() {
        let mut t = VecDeque::new();
        for i in 0..10 {
            t.push_back(mk_line(AudioSource::System, &format!("{i}")));
        }
        let recent = select_recent_lines_from_source(&t, AudioSource::System, 3);
        // Last 3 lines (newest).
        assert_eq!(recent, vec!["7".to_string(), "8".into(), "9".into()]);
    }

    #[test]
    fn select_lines_empty_transcript_returns_empty() {
        let t = VecDeque::new();
        let r = select_recent_lines_from_source(&t, AudioSource::Mic, 5);
        assert!(r.is_empty());
    }

    #[test]
    fn select_lines_zero_max_returns_empty() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::Mic, "a"));
        let r = select_recent_lines_from_source(&t, AudioSource::Mic, 0);
        assert!(r.is_empty());
    }

    #[test]
    fn select_lines_no_matching_source_returns_empty() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::System, "x"));
        t.push_back(mk_line(AudioSource::System, "y"));
        let r = select_recent_lines_from_source(&t, AudioSource::Mic, 5);
        assert!(r.is_empty(), "no Mic lines present, should return empty");
    }

    #[test]
    fn select_lines_max_larger_than_count_returns_all() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::Mic, "only"));
        let r = select_recent_lines_from_source(&t, AudioSource::Mic, 100);
        assert_eq!(r, vec!["only".to_string()]);
    }

    // ── select_recent_lines_labeled — cross-source context for manual ask ──

    #[test]
    fn labeled_lines_preserve_interleaving_with_source_tags() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::System, "Что такое etcd?"));
        t.push_back(mk_line(AudioSource::Mic, "Это распределённый KV-store"));
        t.push_back(mk_line(AudioSource::System, "А что про raft?"));
        let r = select_recent_lines_labeled(&t, 5);
        assert_eq!(r.len(), 3);
        assert!(r[0].starts_with("[СОБЕСЕДНИК]"));
        assert!(r[1].starts_with("[ПОЛЬЗОВАТЕЛЬ]"));
        assert!(r[2].starts_with("[СОБЕСЕДНИК]"));
        // Order is preserved (oldest → newest)
        assert!(r[0].contains("etcd"));
        assert!(r[2].contains("raft"));
    }

    #[test]
    fn labeled_lines_takes_tail_when_over_max() {
        let mut t = VecDeque::new();
        for i in 0..10 {
            t.push_back(mk_line(
                if i % 2 == 0 {
                    AudioSource::Mic
                } else {
                    AudioSource::System
                },
                &format!("line{i}"),
            ));
        }
        let r = select_recent_lines_labeled(&t, 3);
        assert_eq!(r.len(), 3);
        assert!(r[2].contains("line9"), "last must be newest");
        assert!(r[0].contains("line7"));
    }

    #[test]
    fn labeled_lines_empty_returns_empty() {
        let t = VecDeque::new();
        assert!(select_recent_lines_labeled(&t, 5).is_empty());
    }

    // ── find_last_line_from_source — trigger lookup for source button ──

    #[test]
    fn find_last_returns_newest_from_source_even_with_others_in_between() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::System, "sys-old"));
        t.push_back(mk_line(AudioSource::Mic, "mic-only"));
        t.push_back(mk_line(AudioSource::System, "sys-new"));
        t.push_back(mk_line(AudioSource::Mic, "mic-new"));
        assert_eq!(
            find_last_line_from_source(&t, AudioSource::System).as_deref(),
            Some("sys-new")
        );
        assert_eq!(
            find_last_line_from_source(&t, AudioSource::Mic).as_deref(),
            Some("mic-new")
        );
    }

    #[test]
    fn find_last_returns_none_when_no_match() {
        let mut t = VecDeque::new();
        t.push_back(mk_line(AudioSource::Mic, "a"));
        t.push_back(mk_line(AudioSource::Mic, "b"));
        assert!(find_last_line_from_source(&t, AudioSource::System).is_none());
    }

    #[test]
    fn find_last_empty_transcript_returns_none() {
        let t = VecDeque::new();
        assert!(find_last_line_from_source(&t, AudioSource::Mic).is_none());
    }

    // ── Regression: replay real DevOps interview transcript through detector ──
    // Captured from actual YouTube session 2026-05-24. If we ever change
    // the detector and these stop triggering, we're regressing.
    #[test]
    fn replay_real_interview_chunks_still_trigger() {
        let kws = "kubernetes etcd istio terraform prometheus postgres redis kafka \
                   docker nginx linux load balancer cpu memory monitoring lvm";

        // These all triggered in the live session — must keep triggering.
        let triggering = [
            "как можно ее решить",
            "Что такое LVM? КВМ или LVM? LVM",
            "расскажите там по уровню абстракции",
            "какие у него есть логических томов",
            "А вот расскажи как ты диагностировал бы такое",
            "Допустим у тебя падает сервис в продакшене",
            "С чего начнёшь дебагать?",
            "у нас используется prometheus в стеке",
            "почему load average растет",
            "Tell me how would you debug this",
        ];
        for text in triggering.iter() {
            assert!(
                detect_trigger(text, kws).is_some(),
                "regression — should trigger: '{text}'"
            );
        }

        // These should NOT trigger — quiet acknowledgements, fillers, etc.
        let non_triggering = ["да-да понял", "угу", "ну вот так", "конечно", "так точно"];
        for text in non_triggering.iter() {
            assert!(
                detect_trigger(text, kws).is_none(),
                "regression — should NOT trigger: '{text}'"
            );
        }
    }
}
