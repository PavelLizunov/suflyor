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
    /// Playback rate (1.0 = normal). rodio resamples → mild pitch shift; scales the
    /// position clock so the seek-bar / timecode still track ORIGINAL-recording time.
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
        // Each seek/resume builds a fresh sink — re-apply the current rate + gain
        // (a new Sink defaults to 1.0/1.0, so without this a seek resets them).
        sink.set_speed(self.speed);
        sink.set_volume(self.volume);
        sink.append(PcmCursor {
            pcm: self.pcm.clone(),
            pos: from,
            sample_rate: self.sample_rate,
        });
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

    /// Set the output gain (clamped 0–4×; >1 boosts quiet recordings). Applies live
    /// to the current sink; load_from re-applies it to any sink built by a seek.
    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 4.0);
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
    use super::samples_advanced;

    #[test]
    fn speed_scales_playback_advance() {
        // The seek-bar / timecode track ORIGINAL-recording position, so at N× the
        // consumed-sample count must scale by N. Regression guard against dropping
        // the speed factor → the bar racing ahead of / lagging the audio.
        assert_eq!(samples_advanced(1.0, 16_000, 1.0), 16_000);
        assert_eq!(samples_advanced(1.0, 16_000, 2.0), 32_000);
        assert_eq!(samples_advanced(2.0, 16_000, 1.5), 48_000);
    }
}
