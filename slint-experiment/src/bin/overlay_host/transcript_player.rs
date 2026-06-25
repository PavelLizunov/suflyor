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

// TEMPORARY: the mini-player UI (next ТЗ2b sub-increment) is the engine's caller;
// until it lands these pub(crate) methods have no production call site in this
// bin (the engine can't be unit-tested — `new()` needs an audio device). Removed
// when the UI wires play/pause/seek/position.
#![allow(dead_code)]

use overlay_backend::session_audio::{ms_for_sample, sample_for_ms};
use rodio::{OutputStream, OutputStreamHandle, Sink};
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
    /// `Some(t0)` while playing — position = origin + t0.elapsed()*sr; `None` when
    /// paused (frozen at `cursor_sample`).
    play_started: Option<Instant>,
    /// Frozen position while paused, and the resume point.
    cursor_sample: usize,
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
                    + (t0.elapsed().as_secs_f64() * f64::from(self.sample_rate)) as usize
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
}
