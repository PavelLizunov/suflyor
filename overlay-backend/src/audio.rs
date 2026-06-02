//! Audio capture: WASAPI loopback (system output) + microphone.
//!
//! Two independent threads, both delivering AudioChunk into a single
//! tokio::mpsc channel. Both streams are resampled to 16 kHz mono i16
//! before emission (Groq Whisper input format).
//!
//! Loopback trick: open a *render* endpoint, but initialise its
//! IAudioClient with Direction::Capture — WASAPI returns the audio
//! that's being played back. This is the same approach pluely uses.
//!
//! Tier 3 note: this module is one of the few exempt from
//! `clippy::unwrap_used` (12 sites). The unwraps are inside loops
//! guarded by `byte_q.len() >= 4` etc. — bounds-checked precondition
//! makes them safe. Rewriting to `.next_chunk::<4>().unwrap_or(...)`
//! would obscure the invariant. See module-level `#[allow]` below.
#![allow(
    clippy::unwrap_used,
    reason = "bounds-checked precondition makes these pop_front().unwrap() calls safe"
)]

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

// wasapi 0.22+ moved free fns onto DeviceEnumerator. Centralise the
// shim so we don't sprinkle `DeviceEnumerator::new()?` everywhere.
fn get_default_device(dir: &Direction) -> Result<wasapi::Device> {
    let e = DeviceEnumerator::new().context("DeviceEnumerator::new")?;
    e.get_default_device(dir).context("get_default_device")
}

fn device_collection(dir: &Direction) -> Result<wasapi::DeviceCollection> {
    let e = DeviceEnumerator::new().context("DeviceEnumerator::new")?;
    e.get_device_collection(dir)
        .context("get_device_collection")
}

/// Target format for downstream STT.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    /// What the other party says — captured via WASAPI loopback.
    System,
    /// What you say — captured from microphone endpoint.
    Mic,
}

/// One line in the rolling session transcript. Each STT-completed
/// utterance becomes a `TranscriptLine` and joins the runtime
/// `VecDeque<TranscriptLine>` (max ~5 min of conversation).
///
/// Moved from `src-tauri/src/runtime.rs` 2026-05-27 as part of
/// Phase B2 port #1 (run_post_meeting_debrief) — the ported fn
/// takes `Vec<TranscriptLine>` and needs the type accessible from
/// overlay-backend without pulling in Tauri.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptLine {
    pub source: AudioSource,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub source: AudioSource,
    /// 16 kHz mono i16 PCM samples.
    pub pcm_i16: Vec<i16>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DeviceList {
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
}

/// Enumerate render + capture endpoints for settings dropdowns.
pub fn list_devices() -> Result<DeviceList> {
    wasapi::initialize_mta().ok().map(|_| ()).unwrap_or(());

    Ok(DeviceList {
        outputs: enumerate(&Direction::Render).unwrap_or_default(),
        inputs: enumerate(&Direction::Capture).unwrap_or_default(),
    })
}

fn enumerate(dir: &Direction) -> Result<Vec<String>> {
    let coll = device_collection(dir)?;
    let n = coll.get_nbr_devices()?;
    let mut v = Vec::with_capacity(n as usize);
    for i in 0..n {
        if let Ok(d) = coll.get_device_at_index(i) {
            if let Ok(name) = d.get_friendlyname() {
                v.push(name);
            }
        }
    }
    // Warn on duplicate friendly names: find_device_by_name matches on friendly
    // name only, so two identically-named endpoints (e.g. two identical USB
    // headsets) resolve to whichever WASAPI enumerates first — not stable across
    // replug/reboot, and presents as silent "wrong device" / "silent capture".
    // Log it so the ambiguous case is at least diagnosable.
    let mut seen = std::collections::HashSet::new();
    for name in &v {
        if !seen.insert(name.as_str()) {
            log::warn!(
                "audio: duplicate {dir:?} endpoint name {name:?} — selection by name is ambiguous"
            );
        }
    }
    Ok(v)
}

fn find_device_by_name(dir: &Direction, name: &str) -> Option<wasapi::Device> {
    let coll = device_collection(dir).ok()?;
    let n = coll.get_nbr_devices().ok()?;
    for i in 0..n {
        if let Ok(d) = coll.get_device_at_index(i) {
            if let Ok(fname) = d.get_friendlyname() {
                if fname == name {
                    return Some(d);
                }
            }
        }
    }
    None
}

/// Handle returned to caller — drop it to stop all capture threads.
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Start capture of system audio (loopback) + microphone.
/// Either source may be skipped by passing an unknown name (logged + ignored).
pub fn start_capture(
    mic_device: Option<String>,
    sys_device: Option<String>,
) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)> {
    let (tx, rx) = mpsc::channel::<AudioChunk>(128);
    let stop = Arc::new(AtomicBool::new(false));

    // System audio (loopback on render endpoint)
    {
        let tx = tx.clone();
        let stop = stop.clone();
        thread::Builder::new()
            .name("audio-system".into())
            .spawn(move || {
                if let Err(e) =
                    capture_thread(AudioSource::System, Direction::Render, sys_device, tx, stop)
                {
                    log::error!("system audio capture failed: {e:#}");
                }
            })?;
    }

    // Microphone (direct capture endpoint)
    {
        let tx = tx;
        let stop = stop.clone();
        thread::Builder::new()
            .name("audio-mic".into())
            .spawn(move || {
                if let Err(e) =
                    capture_thread(AudioSource::Mic, Direction::Capture, mic_device, tx, stop)
                {
                    log::error!("microphone capture failed: {e:#}");
                }
            })?;
    }

    Ok((rx, CaptureHandle { stop }))
}

fn capture_thread(
    source: AudioSource,
    endpoint_dir: Direction,
    device_name: Option<String>,
    tx: mpsc::Sender<AudioChunk>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    // Each WASAPI thread needs its own COM apartment.
    // Returns Err if already initialized — safe to ignore.
    let _ = wasapi::initialize_mta();

    // Try requested direction first; fall back to the other direction.
    // This handles devices like Astro A50 "Stream Out" — exposed as a
    // capture endpoint that already contains the desired mixed audio,
    // so no WASAPI loopback magic is needed.
    let (device, used_dir) = match device_name.as_deref() {
        Some(name) if !name.is_empty() => {
            if let Some(d) = find_device_by_name(&endpoint_dir, name) {
                (d, endpoint_dir)
            } else {
                let other_dir = match endpoint_dir {
                    Direction::Render => Direction::Capture,
                    Direction::Capture => Direction::Render,
                };
                if let Some(d) = find_device_by_name(&other_dir, name) {
                    log::info!(
                        "[{source:?}] device '{}' found as {:?} endpoint (not {:?})",
                        name,
                        other_dir,
                        endpoint_dir
                    );
                    (d, other_dir)
                } else {
                    return Err(anyhow!(
                        "device '{}' not found in either Render or Capture endpoints",
                        name
                    ));
                }
            }
        }
        _ => (
            get_default_device(&endpoint_dir).context("get default device")?,
            endpoint_dir,
        ),
    };

    let dev_name = device.get_friendlyname().unwrap_or_else(|_| "?".into());
    log::info!("[{source:?}] opening device '{dev_name}' (resolved as {used_dir:?})");

    let mut client = device.get_iaudioclient().context("get IAudioClient")?;
    let mix_format = client.get_mixformat().context("get mixformat")?;
    let actual_rate = mix_format.get_samplespersec();
    let actual_channels = mix_format.get_nchannels();

    log::info!(
        "[{source:?}] device native format: {} Hz, {} ch",
        actual_rate,
        actual_channels
    );

    // We ask wasapi to autoconvert to f32 mono at native sample rate.
    let desired = WaveFormat::new(32, 32, &SampleType::Float, actual_rate as usize, 1, None);
    let (_def_period, min_period) = client.get_device_period().context("get device period")?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_period,
    };

    // The loopback trick: render endpoint, but Capture direction in initialize.
    let init_dir = Direction::Capture;
    client
        .initialize_client(&desired, &init_dir, &mode)
        .context("initialize_client")?;

    let event = client.set_get_eventhandle().context("event handle")?;
    let cap_client = client
        .get_audiocaptureclient()
        .context("get audiocaptureclient")?;

    client.start_stream().context("start_stream")?;

    let mut byte_q: VecDeque<u8> = VecDeque::with_capacity(64 * 1024);
    let mut f32_buf: Vec<f32> = Vec::with_capacity(actual_rate as usize); // ~1 sec
    let chunk_samples_target = actual_rate as usize / 5; // ~200 ms native chunks

    // Downsampling state — simple averaging decimator from actual_rate → 16k.
    let ratio = actual_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let start_ts = std::time::Instant::now();

    while !stop.load(Ordering::Acquire) {
        if event.wait_for_event(2000).is_err() {
            // No audio for 2s — Zoom paused, headphones idle, etc. Keep waiting.
            continue;
        }

        byte_q.clear();
        if let Err(e) = cap_client.read_from_device_to_deque(&mut byte_q) {
            log::warn!("[{source:?}] read failed: {e}");
            continue;
        }
        if byte_q.is_empty() {
            continue;
        }

        // Decode f32 LE
        while byte_q.len() >= 4 {
            let b = [
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
            ];
            f32_buf.push(f32::from_le_bytes(b));
        }

        // Emit when we've buffered ~200 ms of native audio.
        if f32_buf.len() >= chunk_samples_target {
            let pcm_i16 = resample_and_quantise(&f32_buf, ratio);
            f32_buf.clear();
            if !pcm_i16.is_empty() {
                let chunk = AudioChunk {
                    source,
                    pcm_i16,
                    timestamp_ms: start_ts.elapsed().as_millis() as u64,
                };
                if tx.blocking_send(chunk).is_err() {
                    log::info!("[{source:?}] receiver dropped, exiting");
                    break;
                }
            }
        }
    }

    let _ = client.stop_stream();
    log::info!("[{source:?}] capture thread exit");
    Ok(())
}

/// Open a fresh WASAPI handle on the requested source and accumulate
/// raw audio samples until `stop` flips to true. Returns one big Vec<i16>
/// at 16 kHz mono — ready to send as ONE WAV to Whisper.
///
/// This is the push-to-talk capture path. Unlike start_capture (which
/// VAD-chunks for the always-on transcript), this records the whole
/// held duration as a single blob so Whisper gets full context and
/// doesn't hallucinate chunk-boundary artifacts.
///
/// Blocking — call from a std::thread::spawn, signal via Arc<AtomicBool>.
/// Both `mic_device` and `sys_device` are needed because we pick one
/// based on `source` and ignore the other.
pub fn record_source_until_stop(
    source: AudioSource,
    mic_device: Option<String>,
    sys_device: Option<String>,
    stop: Arc<AtomicBool>,
) -> Result<Vec<i16>> {
    let _ = wasapi::initialize_mta();

    // Pick the right device + WASAPI direction for the source.
    let (device_name, endpoint_dir) = match source {
        AudioSource::Mic => (mic_device, Direction::Capture),
        AudioSource::System => (sys_device, Direction::Render),
    };

    // Mirror start_capture's device-resolution logic: try requested
    // direction first, fall back to the other (handles A50 Stream Out
    // which lives on Capture but provides system audio).
    let (device, used_dir) = match device_name.as_deref() {
        Some(name) if !name.is_empty() => {
            if let Some(d) = find_device_by_name(&endpoint_dir, name) {
                (d, endpoint_dir)
            } else {
                let other_dir = match endpoint_dir {
                    Direction::Render => Direction::Capture,
                    Direction::Capture => Direction::Render,
                };
                if let Some(d) = find_device_by_name(&other_dir, name) {
                    (d, other_dir)
                } else {
                    return Err(anyhow!("device '{}' not found", name));
                }
            }
        }
        _ => (
            get_default_device(&endpoint_dir).context("get default device")?,
            endpoint_dir,
        ),
    };

    log::info!(
        "[PTT {:?}] opening '{}' as {:?}",
        source,
        device.get_friendlyname().unwrap_or_default(),
        used_dir
    );

    let mut client = device.get_iaudioclient()?;
    let mix_format = client.get_mixformat()?;
    let actual_rate = mix_format.get_samplespersec();
    let desired = WaveFormat::new(32, 32, &SampleType::Float, actual_rate as usize, 1, None);
    let (_def, min_period) = client.get_device_period()?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_period,
    };

    // The loopback trick: even for a render endpoint we init as Capture
    // so WASAPI gives us what's being PLAYED. Same as start_capture.
    client.initialize_client(&desired, &Direction::Capture, &mode)?;
    let event = client.set_get_eventhandle()?;
    let cap_client = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let mut byte_q: VecDeque<u8> = VecDeque::with_capacity(64 * 1024);
    // Reserve for ~30 sec worst case; grows automatically if longer.
    let mut all_f32: Vec<f32> = Vec::with_capacity((actual_rate as usize) * 30);

    while !stop.load(Ordering::Acquire) {
        // Short timeout so we notice the stop flag quickly (≤500 ms).
        if event.wait_for_event(500).is_err() {
            continue;
        }
        byte_q.clear();
        if cap_client.read_from_device_to_deque(&mut byte_q).is_err() {
            continue;
        }
        while byte_q.len() >= 4 {
            let b = [
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
            ];
            all_f32.push(f32::from_le_bytes(b));
        }
    }

    let _ = client.stop_stream();
    let ratio = actual_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let pcm = resample_and_quantise(&all_f32, ratio);
    log::info!(
        "[PTT {:?}] stopped — captured {} samples ({:.1}s mono 16kHz)",
        source,
        pcm.len(),
        pcm.len() as f32 / TARGET_SAMPLE_RATE as f32
    );
    Ok(pcm)
}

/// Record the microphone for a fixed duration, return 16 kHz mono i16 PCM.
/// Blocking — suitable for spawn_blocking from an async command.
/// Record the SYSTEM (loopback) audio for a fixed duration, return
/// 16 kHz mono i16 PCM. Thin wrapper over `record_source_until_stop`:
/// spawns a sleep+set-flag thread to terminate after `duration_ms`.
/// Used by the overlay-host's sys chip for a 3s loopback-health probe
/// (mirrors `record_mic_blocking` for the mic chip).
pub fn record_sys_blocking(duration_ms: u64, sys_device: Option<String>) -> Result<Vec<i16>> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_setter = stop.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(duration_ms));
        stop_setter.store(true, Ordering::Release);
    });
    record_source_until_stop(AudioSource::System, None, sys_device, stop)
}

/// RMS energy of 16-bit PCM samples in dBFS (0 = full-scale, −∞ = silence).
/// Shared by the mic chip + the diagnostics mic/system-audio level checks. A
/// silent room is < −55 dBFS; real speech is > −40 dBFS (threshold −45 dBFS).
#[must_use]
pub fn rms_dbfs(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return f64::NEG_INFINITY;
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|s| {
            let v = f64::from(*s) / 32768.0;
            v * v
        })
        .sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    if rms <= 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * rms.log10()
    }
}

/// A short, gentle, royalty-free ambient clip (faded + level-reduced) used as
/// the diagnostics system-audio self-test cue. Raw interleaved **stereo** i16
/// LE at 44.1 kHz, embedded into the binary. Stereo (not the old mono sine) so
/// it plays in BOTH ears, and ambient (not a piercing tone) so it's pleasant.
static DIAG_CHIME_S16: &[u8] = include_bytes!("../assets/diag-chime.s16");
const DIAG_CHIME_RATE: u32 = 44_100;

/// Read `frames` interleaved-stereo frames from the embedded clip (wrapping at
/// the end), convert i16→f32, and return them as little-endian bytes ready for
/// the WASAPI render buffer. `cursor` indexes individual i16 samples.
fn clip_frames_to_bytes(clip: &[i16], cursor: &mut usize, frames: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(frames * 8); // 2 ch × 4 bytes (f32)
    for _ in 0..(frames * 2) {
        let s = if clip.is_empty() {
            0.0
        } else {
            f32::from(clip[*cursor % clip.len()]) / 32768.0
        };
        *cursor += 1;
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Play the embedded ambient cue (stereo) through the default RENDER device
/// until `stop` is set. 32-bit float, WASAPI shared event mode; autoconvert
/// adapts the clip's 44.1 kHz stereo to the device format. Mirrors the capture
/// loop in `record_source_until_stop`. Used by [`play_tone_and_capture`].
fn play_test_clip(stop: Arc<AtomicBool>) -> Result<()> {
    let _ = wasapi::initialize_mta();
    let device = get_default_device(&Direction::Render).context("default render device")?;
    let mut client = device.get_iaudioclient()?;
    let (_def, min_period) = client.get_device_period()?;
    // Present our format as 44.1 kHz STEREO f32; autoconvert resamples/upmixes
    // to whatever the output device actually runs.
    let desired = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        DIAG_CHIME_RATE as usize,
        2,
        None,
    );
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_period,
    };
    client.initialize_client(&desired, &Direction::Render, &mode)?;
    let event = client.set_get_eventhandle()?;
    let render_client = client.get_audiorenderclient()?;
    let buffer_frames = client.get_buffer_size()?;
    let clip: Vec<i16> = DIAG_CHIME_S16
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    let mut cursor = 0_usize;
    // Pre-fill the entire buffer before starting, then top it up on each event.
    let pre = clip_frames_to_bytes(&clip, &mut cursor, buffer_frames as usize);
    render_client.write_to_device(buffer_frames as usize, &pre, None)?;
    client.start_stream()?;
    while !stop.load(Ordering::Acquire) {
        if event.wait_for_event(200).is_err() {
            continue;
        }
        // `break` (not `?`) on a transient padding error so `stop_stream()`
        // below always runs — graceful shutdown over abrupt early-return.
        let padding = match client.get_current_padding() {
            Ok(p) => p,
            Err(_) => break,
        };
        let avail = buffer_frames.saturating_sub(padding);
        if avail == 0 {
            continue;
        }
        let chunk = clip_frames_to_bytes(&clip, &mut cursor, avail as usize);
        let _ = render_client.write_to_device(avail as usize, &chunk, None);
    }
    let _ = client.stop_stream();
    Ok(())
}

/// Diagnostics system-audio self-test: play a short test tone through the
/// default output WHILE recording the system loopback, and return the captured
/// PCM. If the loopback "hears" the tone the app just played, the whole
/// output→loopback path works — the user doesn't have to play any audio. The
/// caller measures the captured level (see `rms_dbfs`) to decide pass/fail.
pub fn play_tone_and_capture(sys_device: Option<String>) -> Result<Vec<i16>> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_play = stop.clone();
    let play = std::thread::spawn(move || {
        if let Err(e) = play_test_clip(stop_play) {
            log::warn!("diagnostics test cue failed: {e:#}");
        }
    });
    // Let the render stream come up before sampling the loopback.
    std::thread::sleep(std::time::Duration::from_millis(250));
    let captured = record_sys_blocking(1200, sys_device);
    stop.store(true, Ordering::Release);
    let _ = play.join();
    captured
}

pub fn record_mic_blocking(duration_ms: u64, mic_device: Option<String>) -> Result<Vec<i16>> {
    use std::time::Instant;

    let _ = wasapi::initialize_mta();

    let device = match mic_device.as_deref() {
        Some(name) if !name.is_empty() => find_device_by_name(&Direction::Capture, name)
            .ok_or_else(|| anyhow!("mic '{}' not found", name))?,
        _ => get_default_device(&Direction::Capture).context("default capture device")?,
    };

    let mut client = device.get_iaudioclient()?;
    let mix_format = client.get_mixformat()?;
    let actual_rate = mix_format.get_samplespersec();

    let desired = WaveFormat::new(32, 32, &SampleType::Float, actual_rate as usize, 1, None);
    let (_def, min_period) = client.get_device_period()?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_period,
    };

    client.initialize_client(&desired, &Direction::Capture, &mode)?;
    let event = client.set_get_eventhandle()?;
    let cap_client = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let ratio = actual_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let deadline = Instant::now() + std::time::Duration::from_millis(duration_ms);
    let mut byte_q: VecDeque<u8> = VecDeque::with_capacity(64 * 1024);
    let mut f32_all: Vec<f32> =
        Vec::with_capacity((actual_rate as usize) * (duration_ms as usize) / 1000);

    while Instant::now() < deadline {
        if event.wait_for_event(500).is_err() {
            continue;
        }
        byte_q.clear();
        if cap_client.read_from_device_to_deque(&mut byte_q).is_err() {
            continue;
        }
        while byte_q.len() >= 4 {
            let b = [
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
                byte_q.pop_front().unwrap(),
            ];
            f32_all.push(f32::from_le_bytes(b));
        }
    }
    let _ = client.stop_stream();

    Ok(resample_and_quantise(&f32_all, ratio))
}

/// Average-decimate `input` (f32, native rate, mono) → i16 at 16 kHz.
/// Simple but good enough for STT.
fn resample_and_quantise(input: &[f32], ratio: f64) -> Vec<i16> {
    if input.is_empty() || ratio <= 0.0 {
        return Vec::new();
    }
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let start = (i as f64 * ratio).floor() as usize;
        let end = (((i + 1) as f64 * ratio).floor() as usize).min(input.len());
        if start >= end {
            continue;
        }
        let mean: f32 = input[start..end].iter().copied().sum::<f32>() / (end - start) as f32;
        let clamped = mean.clamp(-1.0, 1.0);
        out.push((clamped * i16::MAX as f32) as i16);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_dbfs_silence_and_full_scale() {
        // Empty / all-zero → -inf (silence).
        assert_eq!(rms_dbfs(&[]), f64::NEG_INFINITY);
        assert_eq!(rms_dbfs(&[0, 0, 0, 0]), f64::NEG_INFINITY);
        // Full-scale square wave → ~0 dBFS.
        let full = [i16::MAX, i16::MIN, i16::MAX, i16::MIN];
        let d = rms_dbfs(&full);
        assert!(d > -0.5 && d <= 0.0, "full-scale ~0 dBFS, got {d}");
        // A tiny signal sits well below the -45 dBFS speech threshold.
        assert!(
            rms_dbfs(&[10, -10, 10, -10]) < -45.0,
            "tiny signal must read as quiet"
        );
    }

    #[test]
    fn clip_frames_to_bytes_length_and_wrap() {
        let clip = [100i16, -100, 200, -200]; // 2 stereo frames
        let mut cursor = 0_usize;
        // 3 frames → 3 × 2ch × 4 bytes = 24; reads wrap past the clip end.
        let b = clip_frames_to_bytes(&clip, &mut cursor, 3);
        assert_eq!(b.len(), 24);
        assert_eq!(cursor, 6, "3 frames × 2 channels = 6 samples consumed");
        // Empty clip → silence, no panic / no div-by-zero.
        let mut c2 = 0_usize;
        let z = clip_frames_to_bytes(&[], &mut c2, 2);
        assert_eq!(z.len(), 16);
        assert!(z.iter().all(|&x| x == 0));
    }

    #[test]
    fn decimator_48k_to_16k_is_3_to_1() {
        let input: Vec<f32> = (0..48).map(|i| (i as f32) / 100.0).collect();
        let out = resample_and_quantise(&input, 3.0);
        // 48 / 3 = 16 samples out
        assert_eq!(out.len(), 16);
    }

    #[test]
    fn decimator_handles_empty() {
        let out = resample_and_quantise(&[], 3.0);
        assert!(out.is_empty());
    }

    /// INTEGRATION: a 1 kHz sine wave at 48 kHz, decimated 3:1, must still
    /// have its peak in autocorrelation at ~16 samples (= 1 kHz at 16 kHz).
    /// This proves frequency content is preserved — not just length.
    #[test]
    fn decimator_preserves_1khz_sine_frequency() {
        let sample_rate = 48_000.0;
        let target_rate = 16_000.0;
        let freq = 1000.0;
        let n_samples = 9600; // 200 ms
        let input: Vec<f32> = (0..n_samples)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
            .collect();

        let out = resample_and_quantise(&input, (sample_rate / target_rate) as f64);
        assert_eq!(out.len(), 3200, "200 ms at 16 kHz = 3200 samples");

        // Autocorrelation: find lag at peak (excluding lag=0). For 1 kHz at
        // 16 kHz, period = 16 samples → peak should be at lag 16 (±1).
        let signal: Vec<f32> = out.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        let mut best_lag = 0usize;
        let mut best_corr = f32::MIN;
        for lag in 8..=32 {
            let mut corr = 0.0;
            for i in 0..(signal.len() - lag) {
                corr += signal[i] * signal[i + lag];
            }
            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }
        assert!(
            (16i32 - best_lag as i32).abs() <= 1,
            "expected period ≈16 samples (1 kHz at 16 kHz), got lag={best_lag}"
        );
    }

    #[test]
    fn decimator_ratio_one_is_identity_quantisation() {
        // ratio=1.0 means no resampling — output length == input length,
        // and each sample is just quantised to i16.
        let input = vec![0.0_f32, 0.5, -0.5, 1.0, -1.0];
        let out = resample_and_quantise(&input, 1.0);
        assert_eq!(out.len(), input.len());
        // Check sign + approximate magnitude on bounds.
        assert_eq!(out[0], 0);
        assert!(
            (out[3] - i16::MAX).abs() <= 1,
            "max should round to i16::MAX"
        );
    }

    #[test]
    fn decimator_oversaturation_clamped() {
        // f32 input > 1.0 should be clamped, not wrap around to negative.
        let input = vec![2.0_f32, -2.0, 5.5];
        let out = resample_and_quantise(&input, 1.0);
        assert_eq!(out[0], i16::MAX);
        assert_eq!(out[1], i16::MIN + 1, "−1.0 clamp * i16::MAX = i16::MIN+1");
        assert_eq!(out[2], i16::MAX);
    }

    #[test]
    fn decimator_preserves_average_amplitude() {
        // Constant DC signal at 0.5 should stay near 0.5 after averaging.
        let input = vec![0.5f32; 4800]; // 100 ms at 48k
        let out = resample_and_quantise(&input, 3.0);
        let target_i16 = (0.5 * i16::MAX as f32) as i16;
        for &s in &out {
            assert!(
                (s as i32 - target_i16 as i32).abs() <= 1,
                "DC must survive decimation: got {} expected {}",
                s,
                target_i16
            );
        }
    }
}
