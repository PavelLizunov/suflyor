//! Groq Whisper STT client.
//!
//! Consumes AudioChunk from `audio.rs`, runs a tiny per-source VAD/buffer,
//! and POSTs WAV blobs to Groq's transcription endpoint when an utterance
//! finishes (silence > VAD_HANG_MS) or when the buffer would otherwise
//! grow too large.

use crate::audio::{AudioChunk, AudioSource, TARGET_SAMPLE_RATE};
use crate::config::SttBackendCfg;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use transcribe_rs::onnx::gigaam::GigaAMModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::{SpeechModel, TranscribeOptions};

const GROQ_STT_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
/// Models-list endpoint — used by `test_connection` to validate the
/// Groq API key without uploading audio.
const GROQ_MODELS_URL: &str = "https://api.groq.com/openai/v1/models";

/// Select the ONNX Runtime execution provider for the in-process GigaAM model.
///
/// Must be called once at startup, BEFORE any GigaAM model is loaded: the ORT
/// session builder reads this global preference when it creates the session.
/// `use_gpu` → DirectML (GPU, Windows DX12); otherwise CPU-only. ORT always
/// appends a CPU fallback, so if DirectML can't initialise (no compatible GPU,
/// or a missing/shadowed `DirectML.dll`) the model transparently runs on CPU.
/// The first GPU transcription pays a ~1s one-time DirectML shader-compile.
pub fn configure_gigaam_accelerator(use_gpu: bool) {
    let pref = if use_gpu {
        transcribe_rs::OrtAccelerator::DirectMl
    } else {
        transcribe_rs::OrtAccelerator::CpuOnly
    };
    transcribe_rs::set_ort_accelerator(pref);
    log::info!("GigaAM ONNX Runtime accelerator = {pref}");
}

/// Load the GigaAM int8 model under the configured ORT accelerator, transparently
/// falling back to CPU if a GPU (DirectML) session fails to build. DirectML can
/// fail at graph fusion (0x80070715) when the system `DirectML.dll` is absent or
/// shadowed; rather than break STT entirely we switch the process to the
/// always-available CPU provider for this and all future loads, and log it.
fn load_gigaam(dir: &str) -> std::result::Result<GigaAMModel, transcribe_rs::TranscribeError> {
    match GigaAMModel::load(Path::new(dir), &Quantization::Int8) {
        Ok(m) => Ok(m),
        Err(e)
            if transcribe_rs::get_ort_accelerator() != transcribe_rs::OrtAccelerator::CpuOnly =>
        {
            log::warn!("GigaAM GPU (DirectML) load failed ({e}); falling back to CPU");
            transcribe_rs::set_ort_accelerator(transcribe_rs::OrtAccelerator::CpuOnly);
            GigaAMModel::load(Path::new(dir), &Quantization::Int8)
        }
        Err(e) => Err(e),
    }
}

/// Phase E6 v27 — connection test for the Settings "STT" tab. GETs the
/// Groq models list with the bearer; HTTP 2xx means the key is valid +
/// the endpoint is reachable. 10s timeout. Does NOT log the key.
pub async fn test_connection(api_key: String) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow::anyhow!("no Groq API key set"));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("build reqwest client")?;
    let resp = client
        .get(GROQ_MODELS_URL)
        .bearer_auth(&api_key)
        .send()
        .await
        .context("GET groq models")?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("HTTP {} — key valid", status.as_u16()))
    } else {
        Err(anyhow::anyhow!("HTTP {} — check key", status.as_u16()))
    }
}

/// Backend-aware connection test for the Settings "STT" tab. Cloud → ping Groq
/// models; local Whisper → ping the whisper-server; GigaAM → verify the model
/// files actually load. Never logs secrets.
pub async fn test_connection_backend(backend: &SttBackendCfg) -> Result<String> {
    match backend {
        SttBackendCfg::Cloud { api_key, .. } => test_connection(api_key.clone()).await,
        SttBackendCfg::Whisper {
            base_url, bearer, ..
        } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .context("build reqwest client")?;
            let url = format!("{}/models", base_url.trim_end_matches('/'));
            let mut req = client.get(&url);
            if !bearer.trim().is_empty() {
                req = req.bearer_auth(bearer);
            }
            // Generic on transport failure: a reqwest error's chain embeds the
            // request `url` (the LAN base_url), which must NOT reach the screen-
            // capturable Settings/Diagnostics field. Full detail → file log only.
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("STT whisper-server GET failed: {e:#}");
                    anyhow::bail!("whisper-server unreachable");
                }
            };
            // whisper.cpp has no /models route, so a 404/405 here still means the
            // server is UP and reachable (a truly-down server fails at send()).
            // Report that plainly instead of a scary raw 404.
            if resp.status().is_success() {
                Ok(format!(
                    "HTTP {} — whisper-server ready",
                    resp.status().as_u16()
                ))
            } else {
                Ok("whisper-server up & ready to transcribe".to_string())
            }
        }
        SttBackendCfg::Gigaam { model_dir } => {
            let dir = model_dir.clone();
            tokio::task::spawn_blocking(move || {
                load_gigaam(&dir)
                    .map(|_m| "GigaAM model loaded OK".to_string())
                    .map_err(|e| {
                        log::warn!("GigaAM test load failed: {e}");
                        anyhow::anyhow!("GigaAM: model failed to load (see log)")
                    })
            })
            .await
            .map_err(|e| anyhow::anyhow!("GigaAM test join: {e}"))?
        }
    }
}

/// Synchronous one-shot validation that a GigaAM model directory actually
/// loads. Used by the session-start path (which is sync + on the UI thread) to
/// fail fast with a clear error BEFORE the capture/STT pipeline spins up,
/// mirroring the cloud "Groq key not set" bail. Blocks for the load duration
/// (~0.5 s) — acceptable for a once-per-session start gate. The model handle is
/// dropped immediately; the live pipeline loads its own copy via `spawn`.
pub fn validate_gigaam_dir(model_dir: &str) -> Result<()> {
    load_gigaam(model_dir)
        .map(|_m| ())
        .map_err(|e| anyhow::anyhow!("{e}"))
}
/// Fallback Groq model id if config doesn't specify one. Both "whisper-large-v3"
/// (most accurate) and "whisper-large-v3-turbo" (~3× faster) are valid.
const DEFAULT_GROQ_MODEL: &str = "whisper-large-v3";

/// Below this RMS we consider it silence. Lowered from 200 → 50:
/// A50 Stream Out + YouTube speech can be quiet; 50 still rejects pure silence
/// (which has RMS ≈ 5-20 from interface noise) while catching whispers.
const VAD_RMS_THRESHOLD: f32 = 50.0;
/// How long silence must persist to flush an utterance (ms).
const VAD_HANG_MS: u64 = 800;
/// Force flush if buffer is this long (seconds).
const MAX_UTTERANCE_SEC: u64 = 25;
/// Skip flushing buffers shorter than this (avoid sending noise).
const MIN_UTTERANCE_SEC: f32 = 0.4;
/// Anti-hallucination: require this fraction of 200ms chunks to be above
/// VAD threshold. One isolated spike + 5s of silence = noise burst, not
/// real speech. Whisper hallucinates ("subscribe to my channel", "опыт
/// опыт опыт") on such buffers, so we drop them.
const MIN_VOICE_CHUNK_RATIO: f32 = 0.25;
/// Anti-hallucination: even with voice ratio OK, mean RMS over the WHOLE
/// buffer must be at least this fraction of the VAD threshold. Catches
/// "noise floor + 1 keyboard click" pattern.
const MIN_MEAN_RMS_FRACTION: f32 = 0.6;

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptEvent {
    pub source: AudioSource,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
    #[allow(dead_code)]
    #[serde(default)]
    language: Option<String>,
}

/// Spawn the STT pipeline. Returns receiver of TranscriptEvent.
///
/// `backend` selects the transcription engine: cloud Groq, a local
/// whisper.cpp server, or in-process GigaAM (loaded once here). All the
/// VAD buffering + anti-hallucination logic below is backend-independent.
///
/// `whisper_prompt` (optional) biases recognition toward specific terms —
/// see `build_whisper_prompt`. It is only sent to the Whisper backends
/// (cloud / local whisper-server); GigaAM has no prompt input and ignores it.
pub fn spawn(
    mut audio_rx: mpsc::Receiver<AudioChunk>,
    backend: SttBackendCfg,
    language: Option<String>,
    whisper_prompt: Option<String>,
    health: std::sync::Arc<crate::health::HealthSignals>,
) -> mpsc::Receiver<TranscriptEvent> {
    let (tx, rx) = mpsc::channel::<TranscriptEvent>(64);

    // Back-pressure cap on simultaneous in-flight HTTP STT requests (cloud /
    // local whisper). Carried over from 1st+2nd-pass audits: previously
    // unbounded. GigaAM serialises through its own model mutex, so this only
    // bounds the network backends.
    let stt_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(6));

    tokio::spawn(async move {
        // Pre-load the GigaAM model ONCE (expensive: ~250 MB + ~0.5 s), OFF the
        // async executor via spawn_blocking, when the local-GigaAM backend is
        // selected. None for the HTTP backends. A load failure is logged and
        // leaves the pipeline producing no transcripts (the Settings "Test"
        // button surfaces the real error to the user).
        let gigaam: Option<Arc<Mutex<GigaAMModel>>> =
            if let SttBackendCfg::Gigaam { model_dir } = &backend {
                let dir = model_dir.clone();
                let dir_for_log = model_dir.clone();
                match tokio::task::spawn_blocking(move || load_gigaam(&dir)).await {
                    Ok(Ok(m)) => {
                        log::info!("STT GigaAM model loaded from {dir_for_log}");
                        Some(Arc::new(Mutex::new(m)))
                    }
                    Ok(Err(e)) => {
                        log::error!(
                            "STT GigaAM load FAILED from '{dir_for_log}': {e} — \
                             local STT produces no transcripts until fixed"
                        );
                        None
                    }
                    Err(e) => {
                        log::error!("STT GigaAM load task join failed: {e}");
                        None
                    }
                }
            } else {
                None
            };

        // `reqwest::Client::builder().build()` only fails if TLS init
        // fails — that's a process-wide rustls bring-up failure, not a
        // recoverable runtime condition. Exempt from `expect_used` deny.
        #[allow(
            clippy::expect_used,
            reason = "TLS-init failure is non-recoverable at process startup"
        )]
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client");

        // Resolve the HTTP target (url, optional bearer, model) once for the
        // Whisper-style backends. None for GigaAM (handled in-process).
        let http_target: Option<(String, Option<String>, String)> = match &backend {
            SttBackendCfg::Cloud { api_key, model } => Some((
                GROQ_STT_URL.to_string(),
                Some(api_key.clone()),
                model.clone(),
            )),
            SttBackendCfg::Whisper {
                base_url,
                bearer,
                model,
            } => {
                let url = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));
                let bearer = (!bearer.trim().is_empty()).then(|| bearer.clone());
                Some((url, bearer, model.clone()))
            }
            SttBackendCfg::Gigaam { .. } => None,
        };

        // Per-source rolling buffer + silence tracking
        let mut buffers: HashMap<AudioSource, Utterance> = HashMap::new();

        let mut max_rms_log: HashMap<AudioSource, (f32, u64)> = HashMap::new();
        while let Some(chunk) = audio_rx.recv().await {
            // Health: bump audio-frame timestamp. Chunks arrive every ~200ms
            // so this is plenty granular for the 15s "degraded" threshold.
            health.last_audio_frame_ms.store(
                crate::journal::now_unix_ms() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            let utt = buffers.entry(chunk.source).or_default();
            let rms = rms_i16(&chunk.pcm_i16);
            // Every ~5s log the max RMS we saw — helps diagnose silent/missing capture.
            let entry = max_rms_log.entry(chunk.source).or_insert((0.0, 0));
            if rms > entry.0 {
                entry.0 = rms;
            }
            entry.1 = entry.1.saturating_add(1);
            if entry.1 >= 25 {
                log::info!(
                    "STT [{:?}] last 5s max-RMS={:.1} (VAD threshold {})",
                    chunk.source,
                    entry.0,
                    VAD_RMS_THRESHOLD
                );
                entry.0 = 0.0;
                entry.1 = 0;
            }
            let chunk_duration_ms = (chunk.pcm_i16.len() as u64 * 1000) / TARGET_SAMPLE_RATE as u64;

            utt.samples.extend_from_slice(&chunk.pcm_i16);
            utt.last_ts_ms = chunk.timestamp_ms;
            if utt.start_ts_ms == 0 {
                utt.start_ts_ms = chunk.timestamp_ms;
            }

            if rms < VAD_RMS_THRESHOLD {
                utt.silent_run_ms = utt.silent_run_ms.saturating_add(chunk_duration_ms);
            } else {
                utt.silent_run_ms = 0;
                utt.had_voice = true;
            }

            let dur_sec = utt.samples.len() as f32 / TARGET_SAMPLE_RATE as f32;
            let forced_by_size = dur_sec >= MAX_UTTERANCE_SEC as f32;
            let should_flush =
                (utt.silent_run_ms >= VAD_HANG_MS && utt.had_voice) || forced_by_size;
            if forced_by_size {
                // info, not warn (v0.17.1 audit): the size-cap flush is normal,
                // designed behavior for continuous speech — a warn read like an
                // error in the tester log once the log facade went live.
                log::info!(
                    "STT forced flush for {:?}: {:.1}s buffer reached cap ({}s) — \
                     had_voice={} silent_run={}ms (VAD threshold {})",
                    chunk.source,
                    dur_sec,
                    MAX_UTTERANCE_SEC,
                    utt.had_voice,
                    utt.silent_run_ms,
                    VAD_RMS_THRESHOLD,
                );
            }

            if should_flush {
                let to_send = std::mem::take(utt);
                buffers.remove(&chunk.source);

                // Anti-hallucination gate: buffer must look like real speech.
                // Catches "background noise + keyboard click" patterns that
                // would otherwise trip Whisper into producing fake transcripts.
                let speech_like = buffer_likely_speech(&to_send.samples);
                if !speech_like {
                    log::info!(
                        "noise-gate dropped {:?} buffer ({:.1}s) — pre-STT",
                        chunk.source,
                        dur_sec
                    );
                }
                // Anti-feedback: drop EITHER source while the read-aloud plays.
                // The system loopback hears the TTS directly; the MIC picks up the
                // speakers' acoustic echo (the tester saw read-aloud text appear on
                // the bar — the mic path was previously ungated). Both would be
                // transcribed (shown on the bar / answered by the AI). The user is
                // listening to the read-aloud, not talking, so suppressing both is
                // correct; `is_speaking` clears shortly after playback ends.
                let tts_feedback = crate::tts::is_speaking();
                if tts_feedback {
                    log::info!(
                        "dropped {:?} buffer ({dur_sec:.1}s) — read-aloud is playing",
                        chunk.source
                    );
                }
                if to_send.had_voice && dur_sec >= MIN_UTTERANCE_SEC && speech_like && !tts_feedback
                {
                    let tx = tx.clone();
                    let src = chunk.source;
                    let sample_count = to_send.samples.len();
                    let start_ts = to_send.start_ts_ms;
                    let health_for_task = health.clone();

                    if let Some(model) = &gigaam {
                        // In-process GigaAM — run inference on a blocking thread.
                        let model = model.clone();
                        let samples = to_send.samples;
                        let sem = stt_semaphore.clone();
                        log::info!(
                            "STT submitting {:?}: {} samples ({:.1}s, gigaam)",
                            src,
                            sample_count,
                            dur_sec
                        );
                        tokio::spawn(async move {
                            // Bound queued GigaAM tasks the same way the HTTP path
                            // is bounded: the model mutex serialises execution, but
                            // without a permit the queued tasks (each holding up to
                            // ~800KB of samples) could pile up unbounded if
                            // inference runs slower than the flush rate.
                            let _permit = match sem.acquire_owned().await {
                                Ok(p) => p,
                                Err(_) => return, // semaphore closed, shutting down
                            };
                            let joined = tokio::task::spawn_blocking(move || {
                                gigaam_transcribe(&model, &samples)
                            })
                            .await;
                            let result = joined
                                .unwrap_or_else(|e| Err(anyhow::anyhow!("gigaam task join: {e}")));
                            finish_transcript(
                                result,
                                src,
                                start_ts,
                                sample_count,
                                &tx,
                                &health_for_task,
                            )
                            .await;
                        });
                    } else if let Some((url, bearer, model)) = http_target.clone() {
                        // Cloud Groq or local whisper-server — HTTP multipart.
                        let client = client.clone();
                        let language = language.clone();
                        let whisper_prompt = whisper_prompt.clone();
                        let sem = stt_semaphore.clone();
                        log::info!(
                            "STT submitting {:?}: {} samples ({:.1}s, model={})",
                            src,
                            sample_count,
                            dur_sec,
                            model
                        );
                        tokio::spawn(async move {
                            // Bound concurrent HTTP calls — wait if 6 already in flight.
                            let _permit = match sem.acquire_owned().await {
                                Ok(p) => p,
                                Err(_) => return, // semaphore closed, runtime shutting down
                            };
                            let result = transcribe(
                                &client,
                                &url,
                                bearer.as_deref(),
                                &to_send.samples,
                                language.as_deref(),
                                whisper_prompt.as_deref(),
                                &model,
                            )
                            .await;
                            finish_transcript(
                                result,
                                src,
                                start_ts,
                                sample_count,
                                &tx,
                                &health_for_task,
                            )
                            .await;
                        });
                    }
                }
            }
        }
        log::info!("STT pipeline exit");
    });

    rx
}

#[derive(Default)]
struct Utterance {
    samples: Vec<i16>,
    start_ts_ms: u64,
    last_ts_ms: u64,
    silent_run_ms: u64,
    had_voice: bool,
}

/// Pre-Whisper noise gate. Returns true if buffer is mostly noise/silence
/// — caller should skip transcription. Prevents Whisper hallucinations
/// like "subscribe to my channel" / "продолжение следует" / repetition
/// loops that fire when the model sees mostly-silent input.
///
/// Two tests must both pass to consider it speech:
///   1. Mean RMS over the WHOLE buffer ≥ VAD_RMS_THRESHOLD * MIN_MEAN_RMS_FRACTION
///   2. Fraction of 200ms chunks above VAD threshold ≥ MIN_VOICE_CHUNK_RATIO
pub fn buffer_likely_speech(samples: &[i16]) -> bool {
    if samples.is_empty() {
        return false;
    }
    // Test 1: overall energy
    let mean = rms_i16(samples);
    if mean < VAD_RMS_THRESHOLD * MIN_MEAN_RMS_FRACTION {
        log::debug!(
            "noise-gate: skip — mean RMS {:.1} < {:.1}",
            mean,
            VAD_RMS_THRESHOLD * MIN_MEAN_RMS_FRACTION
        );
        return false;
    }
    // Test 2: voice-chunk ratio
    let chunk = (TARGET_SAMPLE_RATE as usize) / 5; // ~200 ms
    let mut total = 0usize;
    let mut voiced = 0usize;
    for c in samples.chunks(chunk) {
        total += 1;
        if rms_i16(c) > VAD_RMS_THRESHOLD {
            voiced += 1;
        }
    }
    if total == 0 {
        return false;
    }
    let ratio = voiced as f32 / total as f32;
    if ratio < MIN_VOICE_CHUNK_RATIO {
        log::debug!(
            "noise-gate: skip — voice ratio {:.0}% < {:.0}%",
            ratio * 100.0,
            MIN_VOICE_CHUNK_RATIO * 100.0
        );
        return false;
    }
    true
}

/// Post-Whisper hallucination filter. Returns true if text looks like a
/// Whisper hallucination (silence/noise → fabricated phrase). Drop those.
///
/// Known patterns observed in the wild:
/// - "Thank you for watching", "Subscribe to my channel" (YouTube-trained)
/// - "Продолжение следует", "Спасибо за просмотр" (Russian YT artifacts)
/// - Repetition loops: "опыт опыт опыт опыт"
/// - Single very-short non-word: ".", "—", "..."
/// - Only punctuation/whitespace
pub fn is_likely_hallucination(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    // Pure punctuation / no alphanumeric chars at all.
    if !trimmed.chars().any(|c| c.is_alphanumeric()) {
        return true;
    }
    let lower = trimmed.to_lowercase();

    // Known hallucination phrases (substring match — they sometimes have
    // small variations like trailing periods).
    const KNOWN_HALLUCINATIONS: &[&str] = &[
        // English
        "subscribe to my channel",
        "subscribe to our channel",
        "thanks for watching",
        "thank you for watching",
        "please like and subscribe",
        "don't forget to subscribe",
        // Russian / YouTube
        "продолжение следует",
        "спасибо за просмотр",
        "подпишись на канал",
        "подпишитесь на канал",
        "не забудьте подписаться",
        "ставьте лайки",
        // Common gibberish leak from training data
        "субтитры подогнал",
        "редактор субтитров",
        // Live-test 2026-05-25: Russian YouTube subtitlers — Whisper
        // hallucinates these as the audio's "credits line" during silence.
        "субтитры создавал",
        "субтитры от",
        "корректор",
        "dimatorzok",
        "dima torzok",
        "субтитры подготовил",
        "перевод субтитров",
        "автор субтитров",
    ];
    for h in KNOWN_HALLUCINATIONS {
        if lower.contains(h) {
            log::info!("hallucination filter: dropped — matched '{}'", h);
            return true;
        }
    }

    // Repetition loop: same word repeated ≥3 times in a row.
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() >= 3 {
        // Same-word loop ("опыт опыт опыт ...")
        let all_same = words.iter().all(|w| *w == words[0]);
        if all_same {
            log::info!(
                "hallucination filter: dropped — repetition loop of '{}'",
                words[0]
            );
            return true;
        }
        // Same 2-word phrase repeated ("опыт работы опыт работы опыт работы")
        if words.len() >= 6 && words.len().is_multiple_of(2) {
            let pair_match = (0..words.len() / 2)
                .all(|i| words[2 * i] == words[0] && words[2 * i + 1] == words[1]);
            if pair_match {
                log::info!(
                    "hallucination filter: dropped — 2-word loop of '{} {}'",
                    words[0],
                    words[1]
                );
                return true;
            }
        }
    }

    false
}

fn rms_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    (sum / samples.len() as f64).sqrt() as f32
}

/// Process-global single-entry cache for the ad-hoc GigaAM path
/// (`transcribe_once`). Loading GigaAM is ~250 MB + ONNX init (~0.5 s), so
/// re-loading on every push-to-talk / dictation call stalls the user each
/// time. We keep ONE `(model_dir, model)` pair alive: repeated calls with the
/// same dir reuse it; a different dir reloads (replacing the entry). The live
/// session pipeline (`spawn`) keeps its OWN preloaded model, so this is purely
/// for the ad-hoc one-shot flows. GigaAM already serialises through its own
/// `&mut self`, so holding this lock across the transcribe call is fine.
static GIGAAM_CACHE: std::sync::OnceLock<std::sync::Mutex<Option<(String, GigaAMModel)>>> =
    std::sync::OnceLock::new();

/// Drop the cached ad-hoc GigaAM model so the next transcription reloads it.
/// The ORT session bakes its execution provider in at load time, so after the
/// GPU/CPU preference changes (`configure_gigaam_accelerator`) a cached model
/// would keep the old backend. The live session pipeline reloads its own copy
/// on the next session start, so this only needs to clear the ad-hoc cache.
pub fn reset_gigaam_cache() {
    if let Some(lock) = GIGAAM_CACHE.get() {
        *lock.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

/// Public one-shot transcription helper for ad-hoc flows (e.g. prep recording).
/// Provider-aware: cloud Groq, local whisper-server, or in-process GigaAM
/// (cached in `GIGAAM_CACHE` so repeated ad-hoc calls with the same model dir
/// don't pay the ~250 MB + ONNX-init reload cost; the live pipeline preloads
/// its own copy).
pub async fn transcribe_once(
    backend: &SttBackendCfg,
    pcm: &[i16],
    language: Option<&str>,
    whisper_prompt: Option<&str>,
) -> Result<String> {
    match backend {
        SttBackendCfg::Cloud { api_key, model } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .context("build client")?;
            transcribe(
                &client,
                GROQ_STT_URL,
                Some(api_key),
                pcm,
                language,
                whisper_prompt,
                model,
            )
            .await
        }
        SttBackendCfg::Whisper {
            base_url,
            bearer,
            model,
        } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .context("build client")?;
            let url = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));
            let bearer = (!bearer.trim().is_empty()).then_some(bearer.as_str());
            transcribe(&client, &url, bearer, pcm, language, whisper_prompt, model).await
        }
        SttBackendCfg::Gigaam { model_dir } => {
            let dir = model_dir.clone();
            let pcm = pcm.to_vec();
            tokio::task::spawn_blocking(move || {
                let f32s: Vec<f32> = pcm.iter().map(|&s| f32::from(s) / 32768.0).collect();
                // Reuse the cached model when the dir matches; otherwise load
                // (and replace any stale entry). The guard is intentionally held
                // across transcribe(): GigaAM needs `&mut self`, so this is a
                // deliberate single-flight — two concurrent ad-hoc callers (e.g.
                // a tile mic follow-up while a PTT blob is still transcribing)
                // serialise on this process-global cache mutex for the full
                // inference. The live session pipeline uses a SEPARATE model
                // handle, so contention here is rare by design.
                let mut guard = GIGAAM_CACHE
                    .get_or_init(|| Mutex::new(None))
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                if !matches!(&*guard, Some((cached_dir, _)) if *cached_dir == dir) {
                    let m = load_gigaam(&dir).map_err(|e| {
                        // Don't surface the model_dir path (it embeds the user's
                        // Windows username) into a screen-capturable tile; log it.
                        log::warn!("GigaAM load failed: {e}");
                        anyhow::anyhow!("local STT model failed to load (see log)")
                    })?;
                    *guard = Some((dir.clone(), m));
                }
                let model = match guard.as_mut() {
                    Some((_, m)) => m,
                    // Unreachable: we just ensured an entry for `dir` above.
                    None => return Err(anyhow::anyhow!("GigaAM cache empty after load")),
                };
                let r = model
                    .transcribe(&f32s, &TranscribeOptions::default())
                    .map_err(|e| anyhow::anyhow!("GigaAM transcribe: {e}"))?;
                Ok::<String, anyhow::Error>(r.text)
            })
            .await
            .map_err(|e| anyhow::anyhow!("GigaAM join: {e}"))?
        }
    }
}

async fn transcribe(
    client: &reqwest::Client,
    url: &str,
    bearer: Option<&str>,
    pcm: &[i16],
    language: Option<&str>,
    prompt: Option<&str>,
    stt_model: &str,
) -> Result<String> {
    // Encode WAV once; reuse on retries.
    let wav = encode_wav_pcm_i16_mono_16k(pcm)?;

    // Exponential backoff: 0s, 1s, 2s (3 attempts total).
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0u32..3 {
        if attempt > 0 {
            let delay = std::time::Duration::from_millis(1000 * (1u64 << (attempt - 1)));
            tokio::time::sleep(delay).await;
            log::info!("STT retry attempt {} (after {:?})", attempt + 1, delay);
        }

        match transcribe_once_attempt(client, url, bearer, &wav, language, prompt, stt_model).await
        {
            Ok(text) => return Ok(text),
            Err(e) => {
                let msg = format!("{e:#}");
                if is_permanent_error(&msg) {
                    return Err(e);
                }
                log::warn!("STT attempt {} failed: {msg}", attempt + 1);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("STT failed after 3 attempts")))
}

/// Classify error message into "won't get better with retries". Used by
/// the STT retry loop to short-circuit auth/quota errors instead of
/// hammering Groq with 3 attempts.
///
/// Retryable (NOT permanent): network drops, 5xx, 408 timeout, 429 rate-
/// limit (the next attempt is delayed enough to succeed).
fn is_permanent_error(msg: &str) -> bool {
    // 401 invalid key, 403 IP-blocked, 404 model not found, 413 payload too large
    msg.contains("HTTP 401")
        || msg.contains("HTTP 403")
        || msg.contains("HTTP 404")
        || msg.contains("HTTP 413")
}

async fn transcribe_once_attempt(
    client: &reqwest::Client,
    url: &str,
    bearer: Option<&str>,
    wav: &[u8],
    language: Option<&str>,
    prompt: Option<&str>,
    stt_model: &str,
) -> Result<String> {
    let part = reqwest::multipart::Part::bytes(wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    // Use configured model; fall back to default if empty.
    let model = if stt_model.is_empty() {
        DEFAULT_GROQ_MODEL
    } else {
        stt_model
    };
    let mut form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json")
        .text("temperature", "0")
        .part("file", part);
    if let Some(lang) = language {
        if !lang.is_empty() {
            form = form.text("language", lang.to_string());
        }
    }
    // Whisper `prompt` parameter (OpenAI-compatible) biases the decoder
    // toward this vocabulary. Critical for technical terms in Russian
    // speech: without it "kubernetes" gets phonetised to "кобернетес".
    if let Some(p) = prompt {
        if !p.is_empty() {
            form = form.text("prompt", p.to_string());
        }
    }

    let mut req = client.post(url);
    if let Some(b) = bearer {
        req = req.bearer_auth(b);
    }
    // Generic on transport failure: a reqwest error's chain embeds the request
    // `url` (the local Whisper base_url / LAN IP), which must NOT surface in the
    // screen-capturable PTT tile or Diagnostics field. Log full detail to the
    // file log; return a secret-free message. Network errors stay retryable
    // (no "HTTP 4xx" in the text, so is_permanent_error keeps them in the loop).
    let resp = match req.multipart(form).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("STT POST failed: {e:#}");
            anyhow::bail!("STT network error");
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        // Keep the status (drives is_permanent_error + tells the user 401 vs 5xx)
        // but DROP the response body from the returned error: a local server's
        // body can carry paths/internals that would paint into the screen-
        // capturable PTT tile. Body → file log only.
        let body = resp.text().await.unwrap_or_default();
        log::warn!(
            "STT HTTP {status} body: {}",
            body.chars().take(500).collect::<String>()
        );
        anyhow::bail!("STT HTTP {status}");
    }

    let parsed: GroqResponse = resp.json().await.context("parse stt json")?;
    Ok(parsed.text)
}

/// Shared post-transcription handling for every backend: drop empty results,
/// apply the hallucination filter, bump the health timestamp on success, and
/// emit the `TranscriptEvent`.
async fn finish_transcript(
    result: Result<String>,
    src: AudioSource,
    start_ts_ms: u64,
    sample_count: usize,
    tx: &mpsc::Sender<TranscriptEvent>,
    health: &crate::health::HealthSignals,
) {
    match result {
        Ok(text) if !text.trim().is_empty() => {
            if is_likely_hallucination(&text) {
                // Log a COUNT, never the recognized text — overlay-host.log is
                // shareable (the "Collect logs" button) and must not carry the
                // user's meeting transcript.
                log::info!(
                    "STT [{:?}] hallucination filtered ({} chars)",
                    src,
                    text.chars().count()
                );
            } else {
                health.last_stt_ok_ms.store(
                    crate::journal::now_unix_ms() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                // COUNT only — never the recognized text (shareable log; no
                // meeting transcript in it).
                log::info!("STT got text [{:?}]: {} chars", src, text.chars().count());
                let _ = tx
                    .send(TranscriptEvent {
                        source: src,
                        text,
                        timestamp_ms: start_ts_ms,
                    })
                    .await;
            }
        }
        Ok(_) => log::warn!(
            "STT returned EMPTY for {:?} ({} samples) — heard silence/noise",
            src,
            sample_count
        ),
        Err(e) => log::warn!("STT failed for {:?}: {e:#}", src),
    }
}

/// Run one in-process GigaAM transcription (called on a blocking thread).
/// Converts i16 PCM (16 kHz mono) to f32 in [-1, 1] then runs the shared model
/// under its mutex. GigaAM is Russian-specialised and takes no prompt.
fn gigaam_transcribe(model: &Arc<Mutex<GigaAMModel>>, pcm: &[i16]) -> Result<String> {
    let f32s: Vec<f32> = pcm.iter().map(|&s| f32::from(s) / 32768.0).collect();
    // Recover from a poisoned mutex instead of erroring out: a single ORT
    // panic on one utterance must not brick STT for the rest of the session
    // (the model's internal state is re-entrant per transcribe call).
    let mut m = model.lock().unwrap_or_else(|p| p.into_inner());
    let res = m
        .transcribe(&f32s, &TranscribeOptions::default())
        .map_err(|e| anyhow::anyhow!("GigaAM transcribe: {e}"))?;
    Ok(res.text)
}

/// Canonical Latin spellings of high-frequency loanwords that Whisper
/// notoriously cyrillicises wrong on Russian-language audio. Listed
/// FIRST in the Whisper prompt so the decoder biases strongest toward
/// these spellings — empirically Whisper weights the first ~50 tokens
/// of the prompt heaviest.
///
/// Curated from live-test mistranscriptions (#101 — alias scan): we
/// observed "Кубернетес" / "докер" / "имидж" / "Демеск" / "Прокмаст"
/// where Latin spelling would have been recognised by detector.
/// ~80 most-confusable Latin-spelled tech terms. Budget: ≤500 chars so
/// the user's trigger_keywords + meeting_context still fit in Whisper's
/// 800-char prompt window. Curated 2026-05-25 from real Russian-language
/// engineering transcripts (live tests + interview audio corpora) where
/// Whisper consistently mangled the spelling.
const CANONICAL_TECH_VOCAB: &str = "kubernetes k8s k3s docker containerd \
podman gitlab github jenkins ansible terraform helm kustomize argocd \
prometheus grafana alertmanager loki tempo jaeger elasticsearch kibana \
opensearch fluentbit fluentd vector etcd consul vault nomad nginx haproxy \
envoy traefik istio linkerd cilium calico flannel ingress service \
postgres pgbouncer mysql mariadb mongo redis memcached cassandra clickhouse \
cockroachdb kafka rabbitmq nats activemq pulsar \
aws gcp azure ec2 s3 rds eks gke aks lambda dynamodb cloudfront vpc iam \
proxmox vmware xcp openstack kvm qemu libvirt \
runner registry alpine debian ubuntu rhel systemd cgroup namespace \
dmesg journalctl iptables nftables bpf ebpf \
pipeline compose container image network cache volume statefulset \
deployment daemonset job cronjob configmap secret pvc";

/// Compose a Whisper `prompt` from the canonical tech vocab + user's
/// trigger keywords + a trimmed meeting context. Whisper uses this as
/// recent-context priming — terms appearing here are spelled correctly
/// in output far more often than without.
///
/// Layout, ordered by decoder weight (most influential first):
///   1. Canonical Latin tech vocab (constant, ~200 chars)
///   2. User's per-profile trigger keywords (config)
///   3. Trimmed meeting_context (whatever fits)
///
/// Groq Whisper inherits original limit ~224 tokens (≈800 chars). Cap.
///
/// Returns None if user has nothing custom — just CANONICAL_TECH_VOCAB
/// alone is too generic to bias correctly.
pub fn build_whisper_prompt(keywords: &str, meeting_context: &str) -> Option<String> {
    // Groq enforces 896-char hard limit on the `prompt` field (Whisper
    // inherits ~224 tokens). Live regression 2026-05-25: a prompt builder
    // bug let 946 chars leak through (kw_section_len + ctx_section_min
    // budget reservation underestimated the final string length when the
    // user's `trigger_keywords` was 500+ chars). Drop the soft cap to
    // 700 to give a 196-char safety margin AND add a hard truncate
    // sanity-check at the bottom that asserts post-condition.
    const MAX_CHARS: usize = 700;

    let kw: Vec<&str> = keywords
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .collect();
    let ctx = meeting_context.trim();
    if kw.is_empty() && ctx.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(MAX_CHARS);
    // Lead with the canonical tech vocab so Whisper biases strongest here.
    // "Англоязычные термины пишутся латиницей" hints the decoder to keep
    // Latin spellings for the listed words even when audio is ambiguous.
    out.push_str(
        "Технический разговор о DevOps и SRE. Англоязычные термины \
                  пишутся латиницей: ",
    );

    // BUDGET ALLOCATION: vocab is generic, user keywords are specific.
    // When the expanded vocab would consume the whole 800-char budget,
    // we'd squeeze out the per-user keywords entirely (regression caught
    // by `whisper_prompt_includes_keywords_for_bias` test). Reserve at
    // minimum 180 chars for keywords + 100 for context if those inputs
    // are present, then trim vocab to whatever's left.
    let kw_joined = if !kw.is_empty() {
        Some(kw.join(", "))
    } else {
        None
    };
    let kw_section_len = kw_joined
        .as_ref()
        .map(|s| ". Дополнительно: ".chars().count() + s.chars().count())
        .unwrap_or(0);
    let ctx_section_min = if !ctx.is_empty() {
        ". Контекст: ".chars().count() + 80
    } else {
        0
    };
    let header_used = out.chars().count();
    let vocab_budget = MAX_CHARS
        .saturating_sub(header_used)
        .saturating_sub(kw_section_len)
        .saturating_sub(ctx_section_min);
    if vocab_budget >= CANONICAL_TECH_VOCAB.chars().count() {
        out.push_str(CANONICAL_TECH_VOCAB);
    } else {
        // Truncate vocab on whitespace boundary so we don't end mid-token
        // (Whisper would otherwise treat "kuberne" as a noise token).
        let trimmed: String = CANONICAL_TECH_VOCAB.chars().take(vocab_budget).collect();
        let cut = trimmed.rfind(' ').unwrap_or(trimmed.len());
        out.push_str(&trimmed[..cut]);
    }

    if let Some(joined) = kw_joined {
        out.push_str(". Дополнительно: ");
        out.push_str(&joined);
    }
    if !ctx.is_empty() {
        let remaining = MAX_CHARS.saturating_sub(out.chars().count() + 20);
        if remaining > 50 {
            let snippet: String = ctx.chars().take(remaining).collect();
            out.push_str(". Контекст: ");
            out.push_str(&snippet);
        }
    }

    // Hard char-cap (defensive — should be redundant after the budget
    // logic above, but if any of those calcs underestimate we still
    // never ship a prompt that Groq will 400 on).
    if out.chars().count() > MAX_CHARS {
        out = out.chars().take(MAX_CHARS).collect::<String>();
    }
    // Belt-and-suspenders against the Groq 896-char hard limit. If we
    // ever produce >800 chars (i.e. the cap above didn't engage for some
    // reason), force-truncate to 800 instead of letting the API 400.
    const GROQ_HARD_LIMIT: usize = 800;
    if out.chars().count() > GROQ_HARD_LIMIT {
        log::warn!(
            "build_whisper_prompt over-cap (was {} chars, truncating to {}). \
             Check kw_section/ctx_section budget logic.",
            out.chars().count(),
            GROQ_HARD_LIMIT
        );
        out = out.chars().take(GROQ_HARD_LIMIT).collect::<String>();
    }
    Some(out)
}

/// Minimal RIFF WAVE encoder (PCM int16 mono, 16 kHz).
fn encode_wav_pcm_i16_mono_16k(pcm: &[i16]) -> Result<Vec<u8>> {
    let data_size = (pcm.len() * 2) as u32;
    let riff_size = 36 + data_size;
    let mut out = Vec::with_capacity(44 + data_size as usize);

    out.write_all(b"RIFF")?;
    out.write_all(&riff_size.to_le_bytes())?;
    out.write_all(b"WAVE")?;

    // fmt chunk
    out.write_all(b"fmt ")?;
    out.write_all(&16u32.to_le_bytes())?; // chunk size
    out.write_all(&1u16.to_le_bytes())?; // PCM
    out.write_all(&1u16.to_le_bytes())?; // mono
    out.write_all(&TARGET_SAMPLE_RATE.to_le_bytes())?; // sample rate
    out.write_all(&(TARGET_SAMPLE_RATE * 2).to_le_bytes())?; // byte rate
    out.write_all(&2u16.to_le_bytes())?; // block align
    out.write_all(&16u16.to_le_bytes())?; // bits per sample

    // data chunk
    out.write_all(b"data")?;
    out.write_all(&data_size.to_le_bytes())?;
    for &s in pcm {
        out.write_all(&s.to_le_bytes())?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn permanent_error_classification_survives_redaction() {
        // After secret-redaction, non-2xx errors keep the status (so auth/quota
        // stay permanent → no pointless retries) while the body is dropped.
        assert!(is_permanent_error("STT HTTP 401 Unauthorized"));
        assert!(is_permanent_error("STT HTTP 403 Forbidden"));
        assert!(is_permanent_error("STT HTTP 404 Not Found"));
        assert!(is_permanent_error("STT HTTP 413 Payload Too Large"));
        // Transport failures are now generic + retryable (carry no HTTP status).
        assert!(!is_permanent_error("STT network error"));
        assert!(!is_permanent_error("whisper-server unreachable"));
        // 5xx / 429 remain retryable.
        assert!(!is_permanent_error("STT HTTP 500 Internal Server Error"));
        assert!(!is_permanent_error("STT HTTP 429 Too Many Requests"));
    }

    #[test]
    fn wav_header_is_44_bytes() {
        let pcm = vec![0i16; 1600]; // 100 ms
        let wav = encode_wav_pcm_i16_mono_16k(&pcm).unwrap();
        assert_eq!(wav.len(), 44 + pcm.len() * 2);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
    }

    /// INTEGRATION: encode PCM → decode with a real WAV library (hound) →
    /// confirm samples + format match. This is what actually proves Whisper
    /// can read what we send (vs just checking magic bytes).
    #[test]
    fn wav_roundtrip_through_hound_preserves_samples_and_format() {
        // Deterministic non-trivial signal: triangle-ish wave + DC offset.
        let pcm_in: Vec<i16> = (0..1600)
            .map(|i| {
                let phase = (i * 7) % 1000 - 500;
                (phase * 30) as i16
            })
            .collect();

        let wav_bytes = encode_wav_pcm_i16_mono_16k(&pcm_in).unwrap();

        let cursor = std::io::Cursor::new(wav_bytes);
        let reader = hound::WavReader::new(cursor).expect("hound must accept our WAV");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1, "must be mono");
        assert_eq!(
            spec.sample_rate, 16_000,
            "must be 16 kHz (Whisper requirement)"
        );
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);

        let pcm_out: Vec<i16> = reader.into_samples::<i16>().map(|r| r.unwrap()).collect();
        assert_eq!(pcm_out.len(), pcm_in.len(), "sample count must match");
        assert_eq!(pcm_out, pcm_in, "samples must be bit-exact after roundtrip");
    }

    #[test]
    fn rms_of_zeros_is_zero() {
        assert_eq!(rms_i16(&[0, 0, 0, 0]), 0.0);
    }

    #[test]
    fn rms_of_const_is_const() {
        let v: Vec<i16> = vec![100; 1000];
        let r = rms_i16(&v);
        assert!((r - 100.0).abs() < 0.01);
    }

    #[test]
    fn rms_handles_empty_input_without_div_by_zero() {
        // RMS of [] must not NaN/inf — should be 0.
        assert_eq!(rms_i16(&[]), 0.0);
    }

    #[test]
    fn rms_ignores_sign_via_squaring() {
        // |-100| == |+100| under RMS (squared then sqrt).
        let pos: Vec<i16> = vec![100; 500];
        let neg: Vec<i16> = vec![-100; 500];
        assert!((rms_i16(&pos) - rms_i16(&neg)).abs() < 0.01);
    }

    #[test]
    fn rms_max_amplitude_does_not_overflow_or_nan() {
        // i16::MAX squared as f64 is well within range, but i16::MIN squared
        // is special (|-32768| > i16::MAX). Make sure f64 path handles it.
        let v: Vec<i16> = vec![i16::MIN; 100];
        let r = rms_i16(&v);
        assert!(r.is_finite(), "RMS of i16::MIN must not be NaN/inf");
        assert!((r - 32768.0).abs() < 1.0, "expected ≈32768, got {r}");
    }

    // ── WAV encoder edge cases ──

    #[test]
    fn wav_with_empty_pcm_produces_valid_header_only() {
        let wav = encode_wav_pcm_i16_mono_16k(&[]).unwrap();
        assert_eq!(wav.len(), 44, "header is exactly 44 bytes for 0-sample PCM");
        // data chunk size (bytes 40..44) must be 0.
        assert_eq!(&wav[40..44], &[0, 0, 0, 0], "data chunk size = 0");
        // RIFF size (bytes 4..8) = 36 + 0
        assert_eq!(&wav[4..8], &36u32.to_le_bytes());
    }

    #[test]
    fn wav_riff_size_matches_actual_content_length() {
        // 1000 samples = 2000 bytes data → RIFF size = 36 + 2000 = 2036
        let pcm = vec![0i16; 1000];
        let wav = encode_wav_pcm_i16_mono_16k(&pcm).unwrap();
        let riff_size = u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]);
        assert_eq!(riff_size, 2036);
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]);
        assert_eq!(data_size, 2000);
    }

    // ── Retry classifier ──

    #[test]
    fn permanent_errors_short_circuit_retry() {
        // 4xx-permanent: 401, 403, 404, 413
        assert!(is_permanent_error("Groq HTTP 401: invalid api key"));
        assert!(is_permanent_error("HTTP 403 Forbidden — IP blocked"));
        assert!(is_permanent_error("HTTP 404: model not found"));
        assert!(is_permanent_error("HTTP 413: payload too large"));
    }

    // ── build_whisper_prompt ──

    #[test]
    fn whisper_prompt_returns_none_for_empty_inputs() {
        assert!(build_whisper_prompt("", "").is_none());
        assert!(build_whisper_prompt("   ", "   ").is_none());
    }

    /// REGRESSION (live 2026-05-25): Groq STT returned 400 with
    /// "prompt length must be 896 characters or fewer, but provided
    /// prompt contains 946 characters". The build_whisper_prompt soft
    /// cap of 800 char DID engage but the per-section budget logic
    /// underestimated by ~150 chars when user trigger_keywords was 500+.
    /// Hard cap of 800 must hold for ANY input.
    #[test]
    fn whisper_prompt_never_exceeds_groq_hard_limit() {
        // Synthesize a realistic large input: 500-char user keywords
        // (Russian DevOps stack) + 300-char meeting_context.
        let big_kw = "kubernetes etcd istio prometheus grafana loki tempo jaeger elasticsearch \
                      kibana opensearch postgres mysql redis kafka rabbitmq mongo clickhouse \
                      docker containerd nginx haproxy envoy traefik linux bash systemd cgroup \
                      namespace iptables conntrack tcpdump strace ltrace bpftrace ebpf perf \
                      htop iostat vmstat netstat ss dig curl wget ssh ansible terraform helm";
        let big_ctx = "Это собеседование на Senior SRE. Опыт: 7 лет Kubernetes, networking, \
                       etcd, прометей. Компания — финтех, ищет инженера в команду из 5 человек, \
                       зрелая инфраструктура AWS + on-prem микс. Контекст про их боли, надо \
                       обсудить chaos engineering, runbooks, error budgets, blue-green deploys.";
        let prompt = build_whisper_prompt(big_kw, big_ctx).expect("should produce a prompt");
        let len = prompt.chars().count();
        assert!(
            len <= 800,
            "prompt is {len} chars — must be ≤800 (Groq enforces 896 hard limit \
             and we want safety margin). build_whisper_prompt regression."
        );
    }

    #[test]
    fn whisper_prompt_includes_keywords_for_bias() {
        let p = build_whisper_prompt("custom-tool another", "").unwrap();
        assert!(p.contains("custom-tool"));
        assert!(p.contains("another"));
        // Comma-separated for natural language flow
        assert!(p.contains("custom-tool, another"));
    }

    #[test]
    fn whisper_prompt_leads_with_canonical_tech_vocab() {
        // Canonical English tech vocab must be present FIRST (highest decoder weight).
        // Only assert words near the START of CANONICAL_TECH_VOCAB — words at
        // the tail (e.g. "dmesg", "proxmox") may legitimately get trimmed when
        // the per-user budget is reserved (e.g. 500-char trigger_keywords).
        // The bias for highest-priority terms is still preserved.
        let p = build_whisper_prompt("etcd", "").unwrap();
        assert!(
            p.contains("kubernetes"),
            "canonical vocab must include kubernetes"
        );
        assert!(p.contains("docker"));
        assert!(p.contains("gitlab"));
        assert!(p.contains("ansible"));
        assert!(p.contains("prometheus"));
        // Canonical vocab appears before user keywords
        let canon_pos = p.find("kubernetes").unwrap();
        let user_pos = p.find("etcd").unwrap();
        assert!(
            canon_pos < user_pos,
            "canonical vocab must precede user keywords"
        );
    }

    #[test]
    fn whisper_prompt_includes_context_snippet_after_keywords() {
        let p = build_whisper_prompt("kubernetes", "Senior SRE interview").unwrap();
        assert!(p.contains("kubernetes"));
        assert!(p.contains("Senior SRE interview"));
        // Keywords come BEFORE context (denser info per token)
        let kw_pos = p.find("kubernetes").unwrap();
        let ctx_pos = p.find("Senior").unwrap();
        assert!(kw_pos < ctx_pos);
    }

    #[test]
    fn whisper_prompt_caps_at_max_chars_for_token_budget() {
        // 200 keywords × 8 chars each = 1600 chars; must cap to ~800.
        let keywords: String = (0..200)
            .map(|i| format!("term{i:03}"))
            .collect::<Vec<_>>()
            .join(" ");
        let p = build_whisper_prompt(&keywords, "").unwrap();
        assert!(
            p.chars().count() <= 800,
            "prompt {} chars exceeds 800-cap",
            p.chars().count()
        );
    }

    #[test]
    fn whisper_prompt_keywords_only_no_context_section() {
        let p = build_whisper_prompt("kubernetes", "").unwrap();
        assert!(
            !p.contains("Контекст:"),
            "no context section when context empty"
        );
    }

    #[test]
    fn whisper_prompt_context_only_no_terms_section() {
        let p = build_whisper_prompt("", "Interview about cloud infra").unwrap();
        assert!(
            !p.contains("Термины:"),
            "no terms section when keywords empty"
        );
        assert!(p.contains("Interview"));
    }

    #[test]
    fn whisper_prompt_skips_context_when_budget_exhausted() {
        // Keywords fill the entire 800-char budget — context must NOT be appended.
        let huge_kw: String = (0..500)
            .map(|i| format!("verylongtermname{i:04}"))
            .collect::<Vec<_>>()
            .join(" ");
        let p = build_whisper_prompt(&huge_kw, "context that should be dropped").unwrap();
        // The kw budget is full; "Контекст:" should be absent.
        assert!(
            !p.contains("Контекст:"),
            "context must be dropped when keyword budget overflows"
        );
    }

    // ── Anti-hallucination tests ──

    #[test]
    fn noise_gate_rejects_pure_silence() {
        let silent = vec![0i16; 16000 * 2]; // 2s of silence
        assert!(!buffer_likely_speech(&silent));
    }

    #[test]
    fn noise_gate_rejects_low_level_noise() {
        // Random low-amplitude noise — below VAD threshold mean.
        let noise: Vec<i16> = (0..32000).map(|i| ((i * 7) % 30 - 15) as i16).collect();
        assert!(!buffer_likely_speech(&noise), "low noise should be skipped");
    }

    #[test]
    fn noise_gate_rejects_silence_plus_one_spike() {
        // 2s silence + 100ms loud spike. Voice ratio < 25% → drop.
        let mut buf = vec![0i16; 32000];
        for sample in buf.iter_mut().take(1600) {
            // 100ms at start
            *sample = 5000;
        }
        assert!(!buffer_likely_speech(&buf), "isolated spike isn't speech");
    }

    #[test]
    fn noise_gate_accepts_sustained_speech() {
        // 2s of sustained signal above threshold.
        let speech: Vec<i16> = (0..32000)
            .map(|i| ((i as f32 * 0.1).sin() * 5000.0) as i16)
            .collect();
        assert!(
            buffer_likely_speech(&speech),
            "sustained signal should pass"
        );
    }

    #[test]
    fn hallucination_filter_drops_empty_or_punct() {
        assert!(is_likely_hallucination(""));
        assert!(is_likely_hallucination("   "));
        assert!(is_likely_hallucination("..."));
        assert!(is_likely_hallucination(" — "));
    }

    #[test]
    fn hallucination_filter_drops_known_phrases() {
        assert!(is_likely_hallucination("Subscribe to my channel!"));
        assert!(is_likely_hallucination("Thanks for watching."));
        assert!(is_likely_hallucination("Спасибо за просмотр"));
        assert!(is_likely_hallucination("Продолжение следует..."));
        assert!(is_likely_hallucination("Не забудьте подписаться на канал"));
    }

    #[test]
    fn hallucination_filter_drops_repetition_loop() {
        assert!(is_likely_hallucination("опыт опыт опыт опыт"));
        assert!(is_likely_hallucination("foo foo foo"));
        // 2-word loop
        assert!(is_likely_hallucination(
            "опыт работы опыт работы опыт работы"
        ));
    }

    #[test]
    fn hallucination_filter_accepts_real_speech() {
        assert!(!is_likely_hallucination("А что такое etcd?"));
        assert!(!is_likely_hallucination(
            "Расскажи как ты диагностировал бы такое"
        ));
        assert!(
            !is_likely_hallucination("Спасибо за ответ"),
            "not a YT phrase"
        );
        // Edge: repeated word but not a loop.
        assert!(!is_likely_hallucination(
            "опыт работы и опыт жизни — оба важны"
        ));
    }

    #[test]
    fn transient_errors_keep_retrying() {
        // 5xx + network: must NOT be classified permanent
        assert!(!is_permanent_error("HTTP 500: internal server error"));
        assert!(!is_permanent_error("HTTP 502: bad gateway"));
        assert!(!is_permanent_error("HTTP 503: service unavailable"));
        assert!(!is_permanent_error("HTTP 504: gateway timeout"));
        // 408 + 429 are intentionally retryable
        assert!(!is_permanent_error("HTTP 408: request timeout"));
        assert!(!is_permanent_error("HTTP 429: rate limited"));
        // Network failures
        assert!(!is_permanent_error("connection reset by peer"));
        assert!(!is_permanent_error("dns lookup failed"));
        assert!(!is_permanent_error("tcp timed out"));
    }
}
