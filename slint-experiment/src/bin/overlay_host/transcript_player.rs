//! ТЗ2b — in-app playback of a session's mixed recording, driving the transcript
//! window's mini-player (play/pause, seek-bar, click-line → seek). rodio provides
//! the output (OutputStream + Sink); we feed the already-mixed i16 PCM
//! (`overlay_backend::session_audio`) through a cheap `Arc<[i16]>` cursor source
//! and implement SEEK by restarting that source at a new offset — rodio's
//! per-source seek support varies, and restarting keeps the position math entirely
//! ours + deterministic. Position is derived from the current source's start
//! sample plus a wall-clock `Instant` while playing (sub-frame UI drift is fine
//! for a seek-bar / active-line highlight). The mapping (ms ↔ sample, clamp) is
//! the pure, unit-tested `session_audio::{sample_for_ms, ms_for_sample}`.
//!
//! SECURITY: playback is LOCAL output only — no network egress — so it is safe
//! under stealth / screen-share, like the clipboard copy.

use overlay_backend::session_audio::{ms_for_sample, sample_for_ms};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;

/// A rodio `Source` over a slice of the shared PCM starting at `pos`. Holds an
/// `Arc<[i16]>` so restarting at a new offset is a cheap pointer clone, never a
/// buffer copy (sessions can be tens of MB of samples).
struct PcmCursor {
    pcm: Arc<[i16]>,
    pos: usize,
    sample_rate: u32,
}

impl Iterator for PcmCursor {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        let s = self.pcm.get(self.pos).copied();
        if s.is_some() {
            self.pos += 1;
        }
        s
    }
}

impl rodio::Source for PcmCursor {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// Fresh input samples pulled per WSOLA `process()` call (~1 s at 16 kHz). Each call
/// is a few ms of CPU on the rodio pull thread.
const WSOLA_CHUNK: usize = 16_384;
/// Output samples crossfaded at chunk boundaries (~16 ms at 16 kHz) to hide the seam
/// between the two independent WSOLA renders of the re-fed overlap.
const XFADE_OUT: usize = 256;

/// Phase 2b (A) — pitch-preserving playback-rate adapter. Wraps `PcmCursor` (still
/// indexed in ORIGINAL samples) and time-stretches its output via the crate's
/// time-domain `Wsola` at ratio = 1/speed, so a 2× listen plays in half the time
/// WITHOUT the resample "chipmunk" pitch shift. Output is at the SAME sample_rate, so
/// original samples are consumed at sr×speed per real second — the position clock
/// (`samples_advanced`) is unchanged.
///
/// WSOLA (time-domain overlap-add) is used INSTEAD of the crate's phase-vocoder
/// `StreamProcessor`: the PV path smears speech into an echoey/hollow "recede" (a
/// 256 ms window at 16 kHz — `fft_size` isn't rescaled for the sample rate — plus an
/// onset-gated overlay + a pitch-shifted ghost blend at 3×), which the owner's live
/// listen rejected. `Wsola` is NOT stateful across calls, so each chunk re-feeds the
/// last `overlap_in` input samples and crossfades the doubly-rendered boundary,
/// keeping the timeline exact to < 1 sample per chunk (fable diagnosis 2026-07-05).
///
/// speed == 1.0 never builds one of these (the caller feeds a bare `PcmCursor`), so
/// normal playback stays bit-exact and zero-latency.
struct StretchSource {
    cursor: PcmCursor,
    wsola: timestretch::stretch::Wsola,
    /// Input samples re-fed into the next chunk so the boundary region is rendered
    /// twice and can be crossfaded without shortening the timeline.
    overlap_in: usize,
    prev_in_tail: Vec<f32>, // last `overlap_in` input samples of the previous chunk
    held_tail: Vec<f32>,    // last XFADE_OUT output samples, withheld for the blend
    out: Vec<f32>,
    read: usize,
    flushed: bool,
    sample_rate: u32,
}

impl StretchSource {
    fn new(pcm: Arc<[i16]>, from: usize, sample_rate: u32, speed: f32) -> Self {
        let ratio = 1.0 / f64::from(speed);
        // Speech-tuned WSOLA: 30 ms segments (≥ 2 pitch periods down to ~70 Hz F0),
        // ±15 ms search — the sizing the crate itself uses for speech-adjacent WSOLA.
        let seg = (f64::from(sample_rate) * 0.030).round() as usize;
        let search = (f64::from(sample_rate) * 0.015).round() as usize;
        Self {
            cursor: PcmCursor {
                pcm,
                pos: from,
                sample_rate,
            },
            wsola: timestretch::stretch::Wsola::new(seg, search, ratio),
            overlap_in: (XFADE_OUT as f64 / ratio).ceil() as usize,
            prev_in_tail: Vec::new(),
            held_tail: Vec::new(),
            out: Vec::new(),
            read: 0,
            flushed: false,
            sample_rate,
        }
    }

    /// Refill `out` with the next stretched chunk (crossfading the boundary against the
    /// previous chunk's withheld tail), or emit the withheld tail once at input EOF.
    /// Returns false only when fully drained. A stretch error degrades to silence for
    /// that chunk (`unwrap_or_default`) rather than crashing the audio thread.
    fn refill(&mut self) -> bool {
        loop {
            let mut inbuf = Vec::with_capacity(self.prev_in_tail.len() + WSOLA_CHUNK);
            inbuf.extend_from_slice(&self.prev_in_tail);
            let mut fresh = 0usize;
            for _ in 0..WSOLA_CHUNK {
                match self.cursor.next() {
                    Some(s) => {
                        inbuf.push(f32::from(s) / 32768.0);
                        fresh += 1;
                    }
                    None => break,
                }
            }
            if fresh == 0 {
                // Input EOF: emit the withheld boundary tail once, then finish.
                if self.flushed {
                    return false;
                }
                self.flushed = true;
                self.out = std::mem::take(&mut self.held_tail);
                self.read = 0;
                return !self.out.is_empty();
            }
            // Runt final chunk: zero-pad so WSOLA accepts it (needs ≥ one segment).
            let seg = self.wsola.segment_size();
            if inbuf.len() < seg {
                inbuf.resize(seg, 0.0);
            }
            let mut stretched = self.wsola.process(&inbuf).unwrap_or_default();
            // The chunk head re-renders the held tail's content (the re-fed input
            // overlap) — crossfade the two renders to hide the seam.
            let blend = self.held_tail.len().min(stretched.len());
            for (i, s) in stretched.iter_mut().take(blend).enumerate() {
                let t = (i as f32 + 0.5) / blend as f32;
                *s = self.held_tail[i] * (1.0 - t) + *s * t;
            }
            // Withhold this chunk's tail for the next boundary blend.
            let hold = XFADE_OUT.min(stretched.len());
            self.held_tail = stretched.split_off(stretched.len() - hold);
            // Remember the input tail to re-feed next chunk (rendered again + blended).
            let tail_from = inbuf.len().saturating_sub(self.overlap_in);
            self.prev_in_tail.clear();
            self.prev_in_tail.extend_from_slice(&inbuf[tail_from..]);

            self.out = stretched;
            self.read = 0;
            if !self.out.is_empty() {
                return true;
            }
        }
    }
}

impl Iterator for StretchSource {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.read >= self.out.len() && !self.refill() {
            return None;
        }
        let s = self.out[self.read];
        self.read += 1;
        Some((s.clamp(-1.0, 1.0) * 32767.0) as i16)
    }
}

impl rodio::Source for StretchSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// A loaded, controllable playback of one session's mixed PCM.
pub(crate) struct TranscriptPlayer {
    // Held ONLY to keep the audio device open — dropping it stops all playback —
    // so it is never read after construction.
    #[allow(dead_code)]
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    pcm: Arc<[i16]>,
    sample_rate: u32,
    /// Sample the current source started at (set on (re)start / seek).
    origin_sample: usize,
    /// `Some(t0)` while playing — position = origin + t0.elapsed()*sr*speed; `None`
    /// when paused (frozen at `cursor_sample`).
    play_started: Option<Instant>,
    /// Frozen position while paused, and the resume point.
    cursor_sample: usize,
    /// Playback rate (1.0 = normal). Applied via the pitch-preserving `StretchSource`
    /// (NOT rodio's resampling `set_speed`); still scales the position clock so the
    /// seek-bar / timecode track ORIGINAL-recording time.
    speed: f32,
    /// Output gain (1.0 = normal; >1 amplifies quiet recordings, can clip if loud).
    volume: f32,
}

/// Samples of ORIGINAL audio consumed after `elapsed` real seconds at `speed`×.
/// At 2× the source is pulled twice as fast, so the seek-bar / timecode — which
/// track original-recording position — must scale by speed too, or they desync.
fn samples_advanced(elapsed_secs: f64, sample_rate: u32, speed: f32) -> usize {
    (elapsed_secs * f64::from(sample_rate) * f64::from(speed)) as usize
}

impl TranscriptPlayer {
    /// Build a player for already-mixed PCM. Errors if there is no default audio
    /// device (the caller then leaves the mini-player hidden).
    pub(crate) fn new(pcm: Vec<i16>, sample_rate: u32) -> anyhow::Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&handle)?;
        sink.pause();
        Ok(Self {
            stream,
            handle,
            sink,
            pcm: Arc::from(pcm),
            sample_rate,
            origin_sample: 0,
            play_started: None,
            cursor_sample: 0,
            speed: 1.0,
            volume: 1.0,
        })
    }

    fn total(&self) -> usize {
        self.pcm.len()
    }

    /// Live position in samples (clamped to total).
    fn position_sample(&self) -> usize {
        let s = match self.play_started {
            Some(t0) => {
                self.origin_sample
                    + samples_advanced(t0.elapsed().as_secs_f64(), self.sample_rate, self.speed)
            }
            None => self.cursor_sample,
        };
        s.min(self.total())
    }

    /// Current playback offset in ms (drives the time readout + seek-bar).
    pub(crate) fn position_ms(&self) -> i64 {
        ms_for_sample(self.position_sample(), self.sample_rate)
    }

    /// Total duration in ms.
    pub(crate) fn total_ms(&self) -> i64 {
        ms_for_sample(self.total(), self.sample_rate)
    }

    /// True while audio is actually advancing (playing AND not past the end).
    pub(crate) fn is_playing(&self) -> bool {
        self.play_started.is_some() && self.position_sample() < self.total()
    }

    /// Replace the source with one starting at sample `from` and reset the cursor.
    /// Caller sets the play/pause state afterwards.
    fn load_from(&mut self, from: usize) -> anyhow::Result<()> {
        let from = from.min(self.total());
        self.sink.stop();
        let sink = Sink::try_new(&self.handle)?;
        // A fresh sink defaults to gain 1.0 → re-apply the current volume.
        sink.set_volume(self.volume);
        // Phase 2b (A): 1.0× feeds the bare cursor (bit-exact, zero-latency);
        // otherwise route through the pitch-preserving time-stretch adapter. We do NOT
        // call sink.set_speed — that resample path (the "chipmunk") is exactly what the
        // adapter replaces; the sink stays at native rate and the position clock
        // (which already scales by speed) is unchanged.
        if (self.speed - 1.0).abs() < f32::EPSILON {
            sink.append(PcmCursor {
                pcm: self.pcm.clone(),
                pos: from,
                sample_rate: self.sample_rate,
            });
        } else {
            sink.append(StretchSource::new(
                self.pcm.clone(),
                from,
                self.sample_rate,
                self.speed,
            ));
        }
        self.sink = sink;
        self.origin_sample = from;
        self.cursor_sample = from;
        Ok(())
    }

    /// Start or resume playback. At/after the end → restart from the beginning.
    pub(crate) fn play(&mut self) -> anyhow::Result<()> {
        // Sync the cursor to the LIVE position first: a clip that reached its
        // natural end leaves `cursor_sample` at the last source's START (it is not
        // advanced while playing), so without this the end check below would miss
        // and we'd resume from that stale point instead of restarting from 0.
        self.cursor_sample = self.position_sample();
        let from = if self.cursor_sample >= self.total() {
            0
        } else {
            self.cursor_sample
        };
        // A fresh source from the cursor keeps origin/position in lockstep with the
        // audio (no drift between the sink's internal cursor and our clock).
        self.load_from(from)?;
        self.sink.play();
        self.play_started = Some(Instant::now());
        Ok(())
    }

    /// Halt playback, freezing the cursor at the current position.
    pub(crate) fn pause(&mut self) {
        self.cursor_sample = self.position_sample();
        self.play_started = None;
        self.sink.pause();
    }

    /// Play/pause toggle for the mini-player button.
    pub(crate) fn toggle(&mut self) -> anyhow::Result<()> {
        if self.is_playing() {
            self.pause();
            Ok(())
        } else {
            self.play()
        }
    }

    /// Seek to a session-relative offset (ms), preserving the play/pause state.
    /// A timecode past the end clamps to the end (via `sample_for_ms`).
    pub(crate) fn seek_ms(&mut self, ms: i64) -> anyhow::Result<()> {
        let sample = sample_for_ms(ms, self.sample_rate, self.total());
        if self.play_started.is_some() {
            self.load_from(sample)?;
            self.sink.play();
            self.play_started = Some(Instant::now());
        } else {
            self.cursor_sample = sample;
            self.origin_sample = sample;
        }
        Ok(())
    }

    /// Set the playback rate (clamped 0.5–3.0×). While playing, the elapsed time so
    /// far was clocked at the OLD rate, so we re-anchor: freeze the live position,
    /// then reload from there at the new rate (load_from resets origin + restarts the
    /// clock). Paused → just store it; the next play()/load_from applies it.
    pub(crate) fn set_speed(&mut self, speed: f32) -> anyhow::Result<()> {
        let speed = speed.clamp(0.5, 3.0);
        let cur = self.position_sample();
        self.speed = speed;
        if self.play_started.is_some() {
            self.load_from(cur)?;
            self.sink.play();
            self.play_started = Some(Instant::now());
        } else {
            self.cursor_sample = cur;
        }
        Ok(())
    }

    /// Set the output gain (clamped 0–3×, matching the UI slider max; >1 boosts quiet
    /// recordings). Applies live to the current sink; load_from re-applies it to any
    /// sink built by a seek.
    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 3.0);
        self.sink.set_volume(self.volume);
    }
}

// ── UI-thread control surface ────────────────────────────────────────────────
// rodio's OutputStream is !Send + must live on the UI thread, and only ONE
// transcript window exists at a time, so the live player + its poll timer are
// thread-locals the window's Slint callbacks drive through these free fns (no
// param-threading). All access is on the Slint event loop, sequentially, so the
// RefCell borrows never overlap.

thread_local! {
    /// The single live transcript player. Cleared on window close / before
    /// loading another session.
    static PLAYER: RefCell<Option<TranscriptPlayer>> = const { RefCell::new(None) };
    /// The repeating position-poll timer driving the seek-bar / time / active
    /// line — held so it outlives the wiring call; dropped by `reset`.
    static POLL_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
}

/// Stop playback + polling and drop the loaded audio. Called on the transcript
/// window's close and before (re)loading a different session.
pub(crate) fn reset() {
    PLAYER.with(|p| *p.borrow_mut() = None);
    POLL_TIMER.with(|t| *t.borrow_mut() = None);
}

/// Lazily load the player for `session_id`'s mixed recording. Returns false when
/// there is no audio (recordings absent/unreadable) so the caller hides controls.
pub(crate) fn ensure(session_id: &str) -> bool {
    PLAYER.with(|p| {
        if p.borrow().is_some() {
            return true;
        }
        match overlay_backend::session_audio::load_mixed_session_audio(session_id) {
            Ok((pcm, sr)) => match TranscriptPlayer::new(pcm, sr) {
                Ok(player) => {
                    *p.borrow_mut() = Some(player);
                    true
                }
                Err(e) => {
                    eprintln!("[overlay-host] transcript player init failed: {e}");
                    false
                }
            },
            Err(_) => false, // no recordings for this session
        }
    })
}

/// True while audio is actually advancing.
pub(crate) fn is_playing() -> bool {
    PLAYER.with(|p| {
        p.borrow()
            .as_ref()
            .is_some_and(TranscriptPlayer::is_playing)
    })
}

/// Play/pause toggle (no-op if no player is loaded).
pub(crate) fn toggle() {
    PLAYER.with(|p| {
        if let Some(pl) = p.borrow_mut().as_mut() {
            if let Err(e) = pl.toggle() {
                eprintln!("[overlay-host] transcript player toggle failed: {e}");
            }
        }
    });
}

/// Seek to a session-relative offset (ms) and ensure playback is running
/// (click-on-line). No-op if no player is loaded.
pub(crate) fn seek_and_play(ms: i64) {
    PLAYER.with(|p| {
        if let Some(pl) = p.borrow_mut().as_mut() {
            if let Err(e) = pl.seek_ms(ms) {
                eprintln!("[overlay-host] transcript player seek failed: {e}");
                return;
            }
            if !pl.is_playing() {
                if let Err(e) = pl.play() {
                    eprintln!("[overlay-host] transcript player play failed: {e}");
                }
            }
        }
    });
}

/// Seek to a fraction (0..1) of the total duration (seek-bar click).
pub(crate) fn seek_fraction(frac: f32) {
    PLAYER.with(|p| {
        if let Some(pl) = p.borrow_mut().as_mut() {
            let total = pl.total_ms();
            let ms = (f64::from(frac.clamp(0.0, 1.0)) * total as f64) as i64;
            if let Err(e) = pl.seek_ms(ms) {
                eprintln!("[overlay-host] transcript player seek failed: {e}");
            }
        }
    });
}

/// Set playback rate (no-op if no player loaded — the wiring calls `ensure` first
/// so a rate chosen before pressing play still takes effect on the loaded player).
pub(crate) fn set_speed(speed: f32) {
    PLAYER.with(|p| {
        if let Some(pl) = p.borrow_mut().as_mut() {
            if let Err(e) = pl.set_speed(speed) {
                eprintln!("[overlay-host] transcript player set_speed failed: {e}");
            }
        }
    });
}

/// Set output gain (no-op if no player loaded — the wiring calls `ensure` first).
pub(crate) fn set_volume(volume: f32) {
    PLAYER.with(|p| {
        if let Some(pl) = p.borrow_mut().as_mut() {
            pl.set_volume(volume);
        }
    });
}

/// Snapshot for the position poll: (progress 0..1, position ms, total ms,
/// playing). `None` when no player is loaded.
pub(crate) fn snapshot() -> Option<(f32, i64, i64, bool)> {
    PLAYER.with(|p| {
        p.borrow().as_ref().map(|pl| {
            let total = pl.total_ms();
            let pos = pl.position_ms();
            let progress = if total > 0 {
                (pos as f64 / total as f64).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            (progress, pos, total, pl.is_playing())
        })
    })
}

/// Store the repeating poll timer so it outlives the wiring call.
pub(crate) fn set_poll_timer(timer: slint::Timer) {
    POLL_TIMER.with(|t| *t.borrow_mut() = Some(timer));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn speed_scales_playback_advance() {
        // The seek-bar / timecode track ORIGINAL-recording position, so at N× the
        // consumed-sample count must scale by N. Regression guard against dropping
        // the speed factor → the bar racing ahead of / lagging the audio.
        assert_eq!(samples_advanced(1.0, 16_000, 1.0), 16_000);
        assert_eq!(samples_advanced(1.0, 16_000, 2.0), 32_000);
        assert_eq!(samples_advanced(2.0, 16_000, 1.5), 48_000);
    }

    #[test]
    fn stretch_source_compresses_length_2x_and_3x_multichunk() {
        // 3 s of a 16 kHz sawtooth (> WSOLA_CHUNK) → several chunk boundaries + the
        // re-feed/crossfade harness. At N× the adapter emits ~1/N the samples
        // (ratio = 1/speed). ±20% absorbs WSOLA's boundary overlap; the point is to
        // catch ratio inversion (speed vs 1/speed → would MULTIPLY, not divide) and any
        // chunk-boundary dropping/duplication.
        let sr = 16_000u32;
        let n_in = 3 * sr as usize; // 48000, > WSOLA_CHUNK (16384) → ~3 chunks
        let pcm: Arc<[i16]> = Arc::from(
            (0..n_in)
                .map(|i| (((i % 200) as i32 - 100) * 100) as i16)
                .collect::<Vec<i16>>(),
        );
        let n2 = StretchSource::new(pcm.clone(), 0, sr, 2.0).count();
        assert!(
            (n_in / 2 * 8 / 10..=n_in / 2 * 12 / 10).contains(&n2),
            "2x output len {n2} not ~half of {n_in}"
        );
        let n3 = StretchSource::new(pcm, 0, sr, 3.0).count();
        assert!(
            (n_in / 3 * 8 / 10..=n_in / 3 * 12 / 10).contains(&n3),
            "3x output len {n3} not ~third of {n_in}"
        );
    }
}
