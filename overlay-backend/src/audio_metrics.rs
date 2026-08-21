//! Per-stream audio health counters (M13 groundwork).
//!
//! Plain saturating accumulators for the two capture streams (mic, system),
//! readable from ordinary Rust worker/snapshot threads. They are NOT meant to
//! be touched from realtime native audio callbacks — the native seam keeps its
//! own ring state and reports overflow frames through the existing capture
//! path when it is wired up.
//!
//! The high-watermark tracks pending samples in the Rust-side accumulator,
//! NOT native ring fill.

use std::sync::{Mutex, PoisonError};

/// Which capture stream a counter update belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Microphone capture.
    Mic,
    /// System/loopback audio capture.
    System,
}

/// Counters for one stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamSnapshot {
    /// Chunks received from capture and emitted downstream.
    pub emitted_chunks: u64,
    /// Drop events on the bounded Rust queue.
    pub queue_drops: u64,
    /// Frames lost to native ring-buffer overflow, as reported by the seam.
    pub ring_overflow_frames: u64,
    /// High-watermark of pending samples in the Rust accumulator.
    ///
    /// This is the Rust-side backlog, not the native ring fill. Max-only:
    /// observing a smaller pending count never lowers it.
    pub max_pending_samples: u64,
    /// Session timestamp (ms) of the last emitted chunk; 0 until the first.
    pub last_emitted_session_ms: u64,
}

/// Stable value copies of both streams' counters.
///
/// Each stream is copied under its own lock, but the two locks are taken
/// sequentially, so one stream's counters may advance between the two
/// acquisitions. No cross-stream consistency is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AudioMetricsSnapshot {
    /// Microphone stream counters.
    pub mic: StreamSnapshot,
    /// System stream counters.
    pub system: StreamSnapshot,
}

#[derive(Debug, Default)]
struct StreamState(StreamSnapshot);

/// Owner of the independent mic and system counters. Every method takes
/// `&self`, so it can sit behind an `Arc` shared by workers and snapshots.
#[derive(Debug, Default)]
pub struct AudioMetrics {
    mic: Mutex<StreamState>,
    system: Mutex<StreamState>,
}

impl AudioMetrics {
    fn state(&self, stream: Stream) -> &Mutex<StreamState> {
        match stream {
            Stream::Mic => &self.mic,
            Stream::System => &self.system,
        }
    }

    fn with_state(&self, stream: Stream, update: impl FnOnce(&mut StreamSnapshot)) {
        let mut guard = self
            .state(stream)
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        update(&mut guard.0);
    }

    /// Count one chunk emitted downstream and remember its session timestamp.
    pub fn record_emitted_chunk(&self, stream: Stream, session_ms: u64) {
        self.with_state(stream, |s| {
            s.emitted_chunks = s.emitted_chunks.saturating_add(1);
            s.last_emitted_session_ms = session_ms;
        });
    }

    /// Count one drop event on the bounded Rust queue.
    pub fn record_queue_drop(&self, stream: Stream) {
        self.with_state(stream, |s| {
            s.queue_drops = s.queue_drops.saturating_add(1);
        });
    }

    /// Add frames lost to native ring-buffer overflow.
    pub fn record_ring_overflow(&self, stream: Stream, frames: u64) {
        self.with_state(stream, |s| {
            s.ring_overflow_frames = s.ring_overflow_frames.saturating_add(frames);
        });
    }

    /// Observe the current pending-sample backlog; only a new maximum is kept.
    pub fn observe_pending_samples(&self, stream: Stream, samples: u64) {
        self.with_state(stream, |s| {
            s.max_pending_samples = s.max_pending_samples.max(samples);
        });
    }

    /// Stable value copy of each stream's counters. The mic and system
    /// locks are acquired sequentially, so values may advance between the
    /// two acquisitions; each per-stream copy is exact at its read time.
    pub fn snapshot(&self) -> AudioMetricsSnapshot {
        let mic = self.mic.lock().unwrap_or_else(PoisonError::into_inner).0;
        let system = self.system.lock().unwrap_or_else(PoisonError::into_inner).0;
        AudioMetricsSnapshot { mic, system }
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioMetrics, Stream};

    #[test]
    fn emitted_count_and_timestamp() {
        let m = AudioMetrics::default();
        m.record_emitted_chunk(Stream::Mic, 1200);
        m.record_emitted_chunk(Stream::Mic, 1250);
        let snap = m.snapshot();
        assert_eq!(snap.mic.emitted_chunks, 2);
        assert_eq!(snap.mic.last_emitted_session_ms, 1250);
    }

    #[test]
    fn every_counter_saturates_at_u64_max() {
        let m = AudioMetrics::default();
        for stream in [Stream::Mic, Stream::System] {
            m.with_state(stream, |s| {
                s.emitted_chunks = u64::MAX;
                s.queue_drops = u64::MAX;
                s.ring_overflow_frames = u64::MAX;
            });
            m.record_emitted_chunk(stream, 1200);
            m.record_queue_drop(stream);
            m.record_ring_overflow(stream, 7);
            let snap = m.snapshot();
            let s = match stream {
                Stream::Mic => snap.mic,
                Stream::System => snap.system,
            };
            assert_eq!(s.emitted_chunks, u64::MAX);
            assert_eq!(s.queue_drops, u64::MAX);
            assert_eq!(s.ring_overflow_frames, u64::MAX);
        }
    }

    #[test]
    fn high_watermark_keeps_max_only() {
        let m = AudioMetrics::default();
        m.observe_pending_samples(Stream::System, 960);
        m.observe_pending_samples(Stream::System, 320);
        m.observe_pending_samples(Stream::System, 640);
        assert_eq!(m.snapshot().system.max_pending_samples, 960);
    }

    #[test]
    fn mic_and_system_state_is_independent() {
        let m = AudioMetrics::default();
        m.record_emitted_chunk(Stream::Mic, 10);
        m.record_queue_drop(Stream::System);
        let snap = m.snapshot();
        assert_eq!(snap.mic.emitted_chunks, 1);
        assert_eq!(snap.mic.queue_drops, 0);
        assert_eq!(snap.system.queue_drops, 1);
        assert_eq!(snap.system.emitted_chunks, 0);
    }

    #[test]
    fn snapshot_is_a_stable_copy() {
        let m = AudioMetrics::default();
        m.record_emitted_chunk(Stream::Mic, 42);
        let before = m.snapshot();
        m.record_emitted_chunk(Stream::Mic, 43);
        m.record_queue_drop(Stream::Mic);
        assert_eq!(before.mic.emitted_chunks, 1);
        assert_eq!(before.mic.queue_drops, 0);
        let after = m.snapshot();
        assert_eq!(after.mic.emitted_chunks, 2);
        assert_eq!(after.mic.queue_drops, 1);
    }
}
