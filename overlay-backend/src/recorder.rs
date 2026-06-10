//! Session audio recorder (v0.13.0).
//!
//! Tees the live 16 kHz mono i16 PCM `AudioChunk` stream to per-channel WAV
//! files — `mic.wav` + `system.wav` — under
//! `%APPDATA%\overlay-mvp\recordings\<session_id>\`. Keeping the two channels
//! SEPARATE preserves "who spoke" at the audio level (the user vs the other
//! side) and is what a future offline re-transcribe / re-summary flow consumes.
//!
//! ## Decoupling from capture + STT (the load-bearing invariant)
//!
//! [`SessionRecorder::feed`] is NON-BLOCKING: it `try_send`s a cloned PCM buffer
//! into a bounded channel and, on overflow, DROPS the chunk and bumps a counter.
//! It never back-pressures the audio path, so a slow disk degrades the recording
//! (a few dropped chunks) but never the real-time transcript. A dedicated std
//! writer thread owns the `hound` WAV writers and finalises the headers when the
//! recorder is dropped (on session stop).
//!
//! ## Crash safety
//!
//! `hound` only writes the real RIFF/`data` chunk sizes on `finalize()`. A crash
//! (or force-kill) leaves a WAV whose header still claims zero samples even
//! though the PCM is on disk. [`repair_unfinalized_in`] runs at the next session
//! start and patches such headers from the actual file length, so a recording
//! interrupted by a crash is still playable.

use crate::audio::{AudioChunk, AudioSource};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Output sample rate — every `AudioChunk` is already 16 kHz mono i16
/// ([`crate::audio::TARGET_SAMPLE_RATE`]).
pub const SAMPLE_RATE: u32 = 16_000;

/// Bounded writer queue. ~256 chunks ≈ several seconds of audio; deep enough to
/// ride out a brief disk stall without dropping, shallow enough to bound memory.
const WRITER_QUEUE: usize = 256;

/// Canonical `hound` WAV header length (RIFF + fmt(16) + data) in bytes.
const WAV_HEADER_LEN: u64 = 44;

enum RecMsg {
    Chunk(AudioSource, Vec<i16>),
    Stop,
}

/// A live session recorder. Hold it for the lifetime of a session; drop it
/// (e.g. when the audio tee ends on session stop) to flush + finalise the WAVs.
pub struct SessionRecorder {
    tx: Option<SyncSender<RecMsg>>,
    dropped: Arc<AtomicU64>,
    join: Option<JoinHandle<()>>,
    dir: PathBuf,
}

impl SessionRecorder {
    /// Open a recorder for `session_id` under the per-user recordings root.
    /// First repairs any crash-truncated WAVs from prior sessions, then (after
    /// creating this session's dir) prunes to the newest `keep_sessions` so the
    /// recordings folder can't grow without bound. WAV files are created lazily
    /// on the first sample of each channel, so a mic-only or system-only session
    /// leaves just one file.
    ///
    /// # Errors
    /// Returns Err if the recordings directory can't be created or the writer
    /// thread can't be spawned. Callers treat this as "record disabled this
    /// session" — it must NOT abort the session.
    pub fn start(session_id: &str, keep_sessions: usize, keep_days: u32) -> Result<Self> {
        let root = recordings_dir()?;
        // Best-effort: a fixup failure must not block opening the recorder. The
        // 30s grace skips a prior session still finalising after a rapid restart.
        if let Err(e) = repair_unfinalized_in(&root, std::time::Duration::from_secs(30)) {
            log::warn!("recorder: WAV repair sweep failed (non-fatal): {e:#}");
        }
        let rec = Self::start_in(root.join(session_id))?;
        // Retention sweeps AFTER the new dir exists, so it's the newest and is
        // always kept. A 30 s grace protects the just-created dir AND any prior
        // session still finalising after a rapid restart. Best-effort — a prune
        // failure must not abort recording. Count-based and age-based bounds
        // (v0.15.0) are independent: whichever is set (non-zero) applies.
        match prune_old_recordings_in(&root, keep_sessions, std::time::Duration::from_secs(30)) {
            Ok(n) if n > 0 => log::info!("recorder: pruned {n} old recording session(s)"),
            Ok(_) => {}
            Err(e) => log::warn!("recorder: retention prune failed (non-fatal): {e:#}"),
        }
        match prune_recordings_older_than_in(&root, keep_days, std::time::Duration::from_secs(30)) {
            Ok(n) if n > 0 => {
                log::info!("recorder: pruned {n} recording(s) older than {keep_days} day(s)")
            }
            Ok(_) => {}
            Err(e) => log::warn!("recorder: age prune failed (non-fatal): {e:#}"),
        }
        Ok(rec)
    }

    /// Open a recorder writing into an explicit directory (test seam).
    pub fn start_in(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create recordings dir {}", dir.display()))?;
        let (tx, rx) = sync_channel::<RecMsg>(WRITER_QUEUE);
        let dropped = Arc::new(AtomicU64::new(0));
        let dir_for_thread = dir.clone();
        let join = std::thread::Builder::new()
            .name("audio-recorder".into())
            .spawn(move || writer_loop(&rx, &dir_for_thread))
            .context("spawn audio-recorder thread")?;
        log::info!("audio recorder started: {}", dir.display());
        Ok(Self {
            tx: Some(tx),
            dropped,
            join: Some(join),
            dir,
        })
    }

    /// Non-blocking tee. Clones the chunk's PCM and queues it for the writer
    /// thread; on a full queue the chunk is DROPPED and counted (never blocks
    /// the audio path). A `Disconnected` writer (thread gone) is silently
    /// ignored — the session continues without recording.
    pub fn feed(&self, chunk: &AudioChunk) {
        if let Some(tx) = &self.tx {
            match tx.try_send(RecMsg::Chunk(chunk.source, chunk.pcm_i16.clone())) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                }
                Err(TrySendError::Disconnected(_)) => {}
            }
        }
    }

    /// How many chunks were dropped due to writer back-pressure so far.
    #[must_use]
    pub fn dropped_chunks(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// The session's recordings directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for SessionRecorder {
    fn drop(&mut self) {
        // Signal end-of-stream so the writer flushes queued chunks + finalises
        // the WAV headers, then wait for it (headers MUST be valid on disk).
        // try_send (not blocking send): if the queue is momentarily full during a
        // disk stall we don't park this (possibly tokio-worker) thread — dropping
        // `tx` right after disconnects the channel, and the writer finalises on
        // `recv()` returning Err either way (review v0.13.0 minor).
        if let Some(tx) = self.tx.take() {
            let _ = tx.try_send(RecMsg::Stop);
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        let d = self.dropped.load(Ordering::Relaxed);
        if d > 0 {
            log::warn!(
                "audio recorder finalised with {d} chunk(s) dropped under load: {}",
                self.dir.display()
            );
        }
    }
}

/// One output channel's writer state. `failed` is the load-bearing bit: once a
/// channel hits a write error we must NOT re-`create()` it later — `create`
/// truncates the file, which would WIPE all audio already recorded this session
/// (review v0.13.0). A failed channel stays permanently closed.
struct ChannelWriter {
    writer: Option<hound::WavWriter<BufWriter<File>>>,
    failed: bool,
}

impl ChannelWriter {
    fn new() -> Self {
        Self {
            writer: None,
            failed: false,
        }
    }

    /// Write one chunk, lazily creating the file on first sample. A create or
    /// write error marks the channel permanently failed (no truncate-and-retry).
    fn write_chunk(&mut self, dir: &Path, name: &str, spec: hound::WavSpec, pcm: &[i16]) {
        if self.failed {
            return; // dead channel — never re-create (would truncate prior audio)
        }
        if self.writer.is_none() {
            match hound::WavWriter::create(dir.join(name), spec) {
                Ok(w) => self.writer = Some(w),
                Err(e) => {
                    log::warn!("recorder: cannot create {name}: {e}");
                    self.failed = true;
                    return;
                }
            }
        }
        if let Some(w) = self.writer.as_mut() {
            for &s in pcm {
                if w.write_sample(s).is_err() {
                    // Mark failed + DROP the writer (so no re-create truncates the
                    // partial file); the samples written so far stay on disk and
                    // are finalised on Stop.
                    log::warn!(
                        "recorder: write error on {name} — channel closed (kept what was written)"
                    );
                    self.failed = true;
                    break;
                }
            }
        }
    }

    fn finalize(self, name: &str) {
        if let Some(w) = self.writer {
            if let Err(e) = w.finalize() {
                log::warn!("recorder: finalize {name} failed: {e}");
            }
        }
    }
}

/// Writer thread body — drains the queue, lazily opening one WAV per channel,
/// and finalises both on `Stop` or channel disconnect.
fn writer_loop(rx: &Receiver<RecMsg>, dir: &Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut mic = ChannelWriter::new();
    let mut sys = ChannelWriter::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            RecMsg::Chunk(source, pcm) => {
                let (ch, name) = match source {
                    AudioSource::Mic => (&mut mic, "mic.wav"),
                    AudioSource::System => (&mut sys, "system.wav"),
                };
                ch.write_chunk(dir, name, spec, &pcm);
            }
            RecMsg::Stop => break,
        }
    }
    mic.finalize("mic.wav");
    sys.finalize("system.wav");
    log::debug!("audio-recorder thread exit: {}", dir.display());
}

/// Per-user recordings root: `<config_dir>/overlay-mvp/recordings`. Sibling of
/// the journal's `sessions/` dir so `recordings/<id>/` pairs with
/// `sessions/<id>.jsonl` for a future re-summary flow.
///
/// # Errors
/// Returns Err if the platform config dir can't be resolved.
pub fn recordings_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("no config dir")?;
    Ok(base.join("overlay-mvp").join("recordings"))
}

/// Keep the newest `keep` session sub-directories of `root` (by directory
/// modified-time), deleting older ones recursively. Returns the number removed.
/// `keep == 0` means "unbounded" (no-op). A directory modified within `min_age`
/// of now is NEVER deleted — it may be a session still recording or one still
/// finalising after a rapid stop→start (an actively-written dir keeps bumping
/// its mtime, so it always looks "recent"); this removes the only real teardown
/// race (review v0.13.0). Best-effort: an undeletable dir is skipped, not fatal.
///
/// # Errors
/// Returns Err only if `root` exists but can't be enumerated.
pub fn prune_old_recordings_in(
    root: &Path,
    keep: usize,
    min_age: std::time::Duration,
) -> Result<usize> {
    if keep == 0 || !root.exists() {
        return Ok(0);
    }
    let now = std::time::SystemTime::now();
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, e.path()))
        })
        .collect();
    if dirs.len() <= keep {
        return Ok(0);
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.0)); // newest first
    let mut removed = 0usize;
    for (mtime, p) in dirs.into_iter().skip(keep) {
        // Too-recent (or future-dated) dir → possibly still being written; skip.
        if now
            .duration_since(mtime)
            .map(|age| age < min_age)
            .unwrap_or(true)
        {
            continue;
        }
        match std::fs::remove_dir_all(&p) {
            Ok(()) => {
                removed += 1;
                log::info!("recorder: pruned old recording {}", p.display());
            }
            Err(e) => log::warn!("recorder: cannot prune {}: {e}", p.display()),
        }
    }
    Ok(removed)
}

/// v0.15.0 — age-based retention: delete session sub-directories of `root`
/// whose modified-time is older than `max_age_days` days. `max_age_days == 0`
/// means "no age limit" (no-op). The same `min_age` grace as the count-based
/// prune applies (an actively-written dir keeps bumping its mtime, so the
/// current session is double-protected: it is both newest and recent).
/// Best-effort: an undeletable dir is skipped, not fatal.
///
/// # Errors
/// Returns Err only if `root` exists but can't be enumerated.
pub fn prune_recordings_older_than_in(
    root: &Path,
    max_age_days: u32,
    min_age: std::time::Duration,
) -> Result<usize> {
    prune_recordings_older_than_at(root, max_age_days, min_age, std::time::SystemTime::now())
}

/// Test seam for [`prune_recordings_older_than_in`]: `now` is injected so a
/// test can age fresh directories by passing a future clock instead of
/// rewriting filesystem mtimes.
fn prune_recordings_older_than_at(
    root: &Path,
    max_age_days: u32,
    min_age: std::time::Duration,
    now: std::time::SystemTime,
) -> Result<usize> {
    if max_age_days == 0 || !root.exists() {
        return Ok(0);
    }
    let max_age = std::time::Duration::from_secs(u64::from(max_age_days) * 24 * 60 * 60);
    let mut removed = 0usize;
    for e in std::fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .flatten()
    {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let Some(mtime) = e.metadata().ok().and_then(|m| m.modified().ok()) else {
            continue;
        };
        // Unknown / future-dated mtime → treat as recent, never delete.
        let Ok(age) = now.duration_since(mtime) else {
            continue;
        };
        if age < min_age || age <= max_age {
            continue;
        }
        match std::fs::remove_dir_all(&p) {
            Ok(()) => {
                removed += 1;
                log::info!("recorder: pruned aged recording {}", p.display());
            }
            Err(e) => log::warn!("recorder: cannot prune {}: {e}", p.display()),
        }
    }
    Ok(removed)
}

/// Sweep `root/*/*.wav` and patch any header whose stored `data` size doesn't
/// match the file length (a crash-truncated, never-finalised recording),
/// rewriting the RIFF + `data` chunk sizes from the actual file size so the WAV
/// becomes playable. Returns the number of files repaired. A file modified within
/// `min_age` of now is SKIPPED — it may be a prior session still being written
/// after a rapid restart, and patching its header concurrently is a spurious
/// (though self-healing) write (review v0.13.0). Best-effort: a single unreadable
/// file is skipped, not fatal.
///
/// # Errors
/// Returns Err only if `root` exists but can't be enumerated at all.
pub fn repair_unfinalized_in(root: &Path, min_age: std::time::Duration) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }
    let now = std::time::SystemTime::now();
    let mut repaired = 0usize;
    let sessions = std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))?;
    for session in sessions.flatten() {
        let sdir = session.path();
        if !sdir.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&sdir) else {
            continue;
        };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) != Some("wav") {
                continue;
            }
            // Skip too-recent files (a still-writing prior session keeps bumping
            // mtime, so it always looks recent → never touched while live).
            let recent = f
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|mt| {
                    now.duration_since(mt)
                        .map(|age| age < min_age)
                        .unwrap_or(true)
                })
                .unwrap_or(false);
            if recent {
                continue;
            }
            match repair_wav_header(&p) {
                Ok(true) => {
                    repaired += 1;
                    log::info!("recorder: repaired crash-truncated WAV {}", p.display());
                }
                Ok(false) => {}
                Err(e) => log::warn!("recorder: cannot inspect {}: {e}", p.display()),
            }
        }
    }
    Ok(repaired)
}

/// Patch one WAV's RIFF + `data` chunk sizes from its actual length if they're
/// stale. Returns `Ok(true)` if a fix was written. A correctly finalised file
/// (sizes already match) returns `Ok(false)`.
fn repair_wav_header(path: &Path) -> Result<bool> {
    let mut file = File::options()
        .read(true)
        .write(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let len = file.metadata().context("stat")?.len();
    if len < WAV_HEADER_LEN {
        return Ok(false); // not even a full header — leave it
    }
    // Confirm the EXACT canonical 44-byte PCM layout hound writes (RIFF/WAVE +
    // a 16-byte `fmt ` chunk + a `data` chunk at offset 36) before touching any
    // bytes. A foreign WAV (WAVEFORMATEXTENSIBLE has data-size at offset 64; a
    // LIST/fact chunk shifts `data` past 36) must be left ALONE — otherwise we'd
    // rewrite the wrong offsets and corrupt someone else's file (review v0.13.0).
    let mut head = [0u8; 44];
    file.read_exact(&mut head).context("read header")?;
    let canonical = &head[0..4] == b"RIFF"
        && &head[8..12] == b"WAVE"
        && &head[12..16] == b"fmt "
        && u32::from_le_bytes([head[16], head[17], head[18], head[19]]) == 16
        && &head[36..40] == b"data";
    if !canonical {
        return Ok(false);
    }
    let data_bytes_actual = ((len - WAV_HEADER_LEN) & !1) as u32; // round to whole i16
                                                                  // RIFF size = everything after the first 8 bytes = data + the 36-byte
                                                                  // fmt/header remainder. Derive from the ROUNDED data size so the two agree
                                                                  // for an odd-length crash-truncated file (review v0.13.0 minor).
    let riff_size_actual = data_bytes_actual + 36;
    // Read currently-stored data-chunk size (offset 40, u32 LE).
    file.seek(SeekFrom::Start(40)).context("seek data size")?;
    let mut cur = [0u8; 4];
    file.read_exact(&mut cur).context("read data size")?;
    if u32::from_le_bytes(cur) == data_bytes_actual {
        return Ok(false); // already correct (finalised normally)
    }
    file.seek(SeekFrom::Start(4)).context("seek riff size")?;
    file.write_all(&riff_size_actual.to_le_bytes())
        .context("write riff size")?;
    file.seek(SeekFrom::Start(40)).context("seek data size 2")?;
    file.write_all(&data_bytes_actual.to_le_bytes())
        .context("write data size")?;
    file.flush().context("flush header")?;
    Ok(true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn chunk(source: AudioSource, samples: &[i16]) -> AudioChunk {
        AudioChunk {
            source,
            pcm_i16: samples.to_vec(),
            timestamp_ms: 0,
        }
    }

    #[test]
    fn records_two_channels_to_separate_finalised_wavs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("sess1");
        {
            let rec = SessionRecorder::start_in(dir.clone()).unwrap();
            rec.feed(&chunk(AudioSource::Mic, &[100, -100, 200, -200]));
            rec.feed(&chunk(AudioSource::System, &[1, 2, 3]));
            rec.feed(&chunk(AudioSource::Mic, &[300, -300]));
            // drop → Stop + join → headers finalised
        }
        // mic.wav has 6 samples, system.wav has 3 — both valid + readable.
        let mic = hound::WavReader::open(dir.join("mic.wav")).expect("mic readable");
        assert_eq!(mic.spec().sample_rate, SAMPLE_RATE);
        assert_eq!(mic.spec().channels, 1);
        let mic_samples: Vec<i16> = mic.into_samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(mic_samples, vec![100, -100, 200, -200, 300, -300]);

        let sys = hound::WavReader::open(dir.join("system.wav")).expect("sys readable");
        let sys_samples: Vec<i16> = sys.into_samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(sys_samples, vec![1, 2, 3]);
    }

    #[test]
    fn failed_channel_never_retries_create_so_it_cannot_truncate() {
        // The load-bearing invariant behind the v0.13.0 write-error fix: once a
        // channel fails, it must NOT re-`create()` (which truncates + wipes the
        // audio already written). Force a create failure (parent dir absent) and
        // verify the channel latches failed + short-circuits the next chunk.
        let tmp = tempfile::tempdir().unwrap();
        let bad_dir = tmp.path().join("missing_parent");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut ch = ChannelWriter::new();
        ch.write_chunk(&bad_dir, "system.wav", spec, &[1, 2, 3]);
        assert!(ch.failed, "a create failure latches the channel failed");
        assert!(ch.writer.is_none());
        // Second chunk must short-circuit on `failed` (no second create attempt).
        ch.write_chunk(&bad_dir, "system.wav", spec, &[4, 5, 6]);
        assert!(ch.failed);
        assert!(!bad_dir.exists(), "a failed channel creates nothing");
    }

    #[test]
    fn channel_with_no_audio_creates_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("mic_only");
        {
            let rec = SessionRecorder::start_in(dir.clone()).unwrap();
            rec.feed(&chunk(AudioSource::Mic, &[1, 2, 3]));
        }
        assert!(dir.join("mic.wav").exists());
        assert!(
            !dir.join("system.wav").exists(),
            "no system audio → no file"
        );
    }

    #[test]
    fn repair_fixes_a_crash_truncated_header() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let sdir = root.join("crashed");
        std::fs::create_dir_all(&sdir).unwrap();
        // Write a real WAV, then CORRUPT its header sizes to simulate a crash
        // that wrote samples but never finalised (sizes still claim 0 data).
        let wav = sdir.join("system.wav");
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: SAMPLE_RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut w = hound::WavWriter::create(&wav, spec).unwrap();
            for i in 0..1000i16 {
                w.write_sample(i).unwrap();
            }
            w.finalize().unwrap();
        }
        // Zero out the RIFF (offset 4) + data (offset 40) sizes.
        {
            let mut f = File::options().write(true).open(&wav).unwrap();
            f.seek(SeekFrom::Start(4)).unwrap();
            f.write_all(&0u32.to_le_bytes()).unwrap();
            f.seek(SeekFrom::Start(40)).unwrap();
            f.write_all(&0u32.to_le_bytes()).unwrap();
        }
        // A reader now sees zero samples (header lies).
        let broken = hound::WavReader::open(&wav)
            .unwrap()
            .into_samples::<i16>()
            .count();
        assert_eq!(broken, 0, "corrupted header should hide the samples");

        let n = repair_unfinalized_in(&root, std::time::Duration::ZERO).unwrap();
        assert_eq!(n, 1, "exactly one file repaired");

        // After repair the 1000 samples are visible again.
        let fixed: Vec<i16> = hound::WavReader::open(&wav)
            .unwrap()
            .into_samples::<i16>()
            .map(|s| s.unwrap())
            .collect();
        assert_eq!(fixed.len(), 1000);
        assert_eq!(fixed[0], 0);
        assert_eq!(fixed[999], 999);

        // Idempotent — a second sweep finds nothing to fix.
        assert_eq!(
            repair_unfinalized_in(&root, std::time::Duration::ZERO).unwrap(),
            0
        );
    }

    #[test]
    fn repair_skips_recent_files_under_grace() {
        // A broken canonical WAV, just written → a large grace SKIPS it (it may be
        // a prior session still finalising after a rapid restart).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let sdir = root.join("s");
        std::fs::create_dir_all(&sdir).unwrap();
        let canon = sdir.join("system.wav");
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: SAMPLE_RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut w = hound::WavWriter::create(&canon, spec).unwrap();
            for i in 0..100i16 {
                w.write_sample(i).unwrap();
            }
            w.finalize().unwrap();
            let mut f = File::options().write(true).open(&canon).unwrap();
            f.seek(SeekFrom::Start(40)).unwrap();
            f.write_all(&0u32.to_le_bytes()).unwrap(); // break it
        }
        let before = std::fs::read(&canon).unwrap();
        assert_eq!(
            repair_unfinalized_in(&root, std::time::Duration::from_secs(3600)).unwrap(),
            0,
            "a just-written file is skipped under grace"
        );
        assert_eq!(
            std::fs::read(&canon).unwrap(),
            before,
            "skipped file untouched"
        );
    }

    #[test]
    fn repair_leaves_foreign_non_canonical_wav_untouched() {
        // A FOREIGN WAV (fmt size 18, not the canonical 16) dropped into a session
        // dir must NEVER be patched — repair must leave it byte-identical even with
        // no grace, so we can't corrupt another program's file.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let sdir = root.join("s");
        std::fs::create_dir_all(&sdir).unwrap();
        let foreign = sdir.join("foreign.wav");
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[4..8].copy_from_slice(&999u32.to_le_bytes()); // wrong riff size
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes[12..16].copy_from_slice(b"fmt ");
        bytes[16..20].copy_from_slice(&18u32.to_le_bytes()); // NON-canonical fmt size
        std::fs::write(&foreign, &bytes).unwrap();
        let foreign_before = std::fs::read(&foreign).unwrap();
        assert_eq!(
            repair_unfinalized_in(&root, std::time::Duration::ZERO).unwrap(),
            0,
            "foreign non-canonical WAV is not 'repaired'"
        );
        assert_eq!(
            std::fs::read(&foreign).unwrap(),
            foreign_before,
            "foreign WAV must be byte-identical after the sweep"
        );
    }

    #[test]
    fn prune_keeps_newest_and_removes_older() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        // Four session dirs created oldest→newest; small sleeps give distinct
        // mtimes so "newest" is unambiguous.
        let mut paths = vec![];
        for i in 0..4 {
            let d = root.join(format!("sess{i}"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("system.wav"), b"x").unwrap();
            paths.push(d);
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        // Keep newest 2 → sess0, sess1 removed; sess2, sess3 survive. min_age=0
        // disables the "too recent" grace so this test is deterministic.
        let zero = std::time::Duration::ZERO;
        let removed = prune_old_recordings_in(&root, 2, zero).unwrap();
        assert_eq!(removed, 2);
        assert!(!paths[0].exists());
        assert!(!paths[1].exists());
        assert!(paths[2].exists());
        assert!(paths[3].exists());
        // Idempotent + keep>=count is a no-op.
        assert_eq!(prune_old_recordings_in(&root, 2, zero).unwrap(), 0);
        assert_eq!(prune_old_recordings_in(&root, 10, zero).unwrap(), 0);
        // keep==0 means unbounded → never prunes.
        assert_eq!(prune_old_recordings_in(&root, 0, zero).unwrap(), 0);
    }

    #[test]
    fn prune_skips_recently_written_dirs_under_grace() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        for i in 0..4 {
            let d = root.join(format!("sess{i}"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("system.wav"), b"x").unwrap();
        }
        // All four dirs were just written → a large grace protects ALL of them,
        // so even with keep=1 nothing is pruned (mirrors a rapid restart where
        // the prior session is still finalising).
        let removed =
            prune_old_recordings_in(&root, 1, std::time::Duration::from_secs(3600)).unwrap();
        assert_eq!(removed, 0, "recently-written dirs are never pruned");
        for i in 0..4 {
            assert!(root.join(format!("sess{i}")).exists());
        }
    }

    #[test]
    fn repair_ignores_missing_root_and_non_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let z = std::time::Duration::ZERO;
        assert_eq!(
            repair_unfinalized_in(&tmp.path().join("nope"), z).unwrap(),
            0
        );
        let sdir = tmp.path().join("s");
        std::fs::create_dir_all(&sdir).unwrap();
        std::fs::write(sdir.join("notes.txt"), b"hello").unwrap();
        assert_eq!(repair_unfinalized_in(tmp.path(), z).unwrap(), 0);
    }

    // ── prune_recordings_older_than_* — v0.15.0 age-based retention ──

    #[test]
    fn age_prune_zero_days_is_noop_and_fresh_dirs_survive() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        for i in 0..3 {
            let d = root.join(format!("sess{i}"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("mic.wav"), b"x").unwrap();
        }
        let zero = std::time::Duration::ZERO;
        // days == 0 → "no age limit": no-op even with a far-future clock.
        let far = std::time::SystemTime::now() + std::time::Duration::from_secs(365 * 24 * 3600);
        assert_eq!(
            prune_recordings_older_than_at(&root, 0, zero, far).unwrap(),
            0
        );
        // Real clock: nothing is older than a day → all kept.
        assert_eq!(prune_recordings_older_than_in(&root, 1, zero).unwrap(), 0);
        for i in 0..3 {
            assert!(root.join(format!("sess{i}")).exists());
        }
    }

    #[test]
    fn age_prune_removes_dirs_older_than_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        for i in 0..3 {
            std::fs::create_dir_all(root.join(format!("sess{i}"))).unwrap();
        }
        std::fs::write(root.join("loose.txt"), b"not a dir").unwrap();
        // Inject a clock 8 days ahead: every fresh dir is now "8 days old",
        // which exceeds the 7-day limit → all pruned; the loose file survives.
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(8 * 24 * 3600);
        let removed =
            prune_recordings_older_than_at(&root, 7, std::time::Duration::ZERO, future).unwrap();
        assert_eq!(removed, 3);
        for i in 0..3 {
            assert!(!root.join(format!("sess{i}")).exists());
        }
        assert!(root.join("loose.txt").exists());
    }

    #[test]
    fn age_prune_keeps_dir_at_exact_age_boundary() {
        // age == max_age must be KEPT (the comparison is `age <= max_age` →
        // skip): "older than N days" is strict. Pins the boundary semantics.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let d = root.join("sess0");
        std::fs::create_dir_all(&d).unwrap();
        let mtime = std::fs::metadata(&d).unwrap().modified().unwrap();
        let exactly_one_day = mtime + std::time::Duration::from_secs(24 * 3600);
        let removed =
            prune_recordings_older_than_at(&root, 1, std::time::Duration::ZERO, exactly_one_day)
                .unwrap();
        assert_eq!(removed, 0, "a dir exactly max_age old is kept");
        assert!(d.exists());
    }

    #[test]
    fn age_prune_grace_window_beats_age_limit() {
        // The min_age grace must win over the age limit: with a +2-day clock a
        // fresh dir looks 2 days old (> the 1-day limit), but a 3-day grace
        // still protects it (mirrors the count-prune's still-finalising guard).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let d = root.join("sess0");
        std::fs::create_dir_all(&d).unwrap();
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(2 * 24 * 3600);
        let removed = prune_recordings_older_than_at(
            &root,
            1,
            std::time::Duration::from_secs(3 * 24 * 3600),
            future,
        )
        .unwrap();
        assert_eq!(removed, 0);
        assert!(d.exists());
    }
}
