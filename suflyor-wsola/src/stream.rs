//! Small streaming wrapper around [`crate::Wsola`] for live speech playback.
//!
//! Callers feed consecutive fresh mono samples. The wrapper re-feeds the input
//! overlap required at chunk boundaries, crossfades the duplicate render, and
//! withholds one short output tail until the next chunk. `finish` releases that
//! tail at end-of-stream. A seek or live speed change creates a fresh wrapper.

use crate::{Wsola, WsolaError};

/// Boundary blend duration. Keeping it time-based matters because Tera plays
/// at 44.1 kHz; the old fixed 256 samples shrank to 5.8 ms and sounded bubbly.
const XFADE_MS: f64 = 16.0;

pub struct StreamingWsola {
    wsola: Wsola,
    overlap_in: usize,
    crossfade_out: usize,
    previous_input_tail: Vec<f32>,
    held_output_tail: Vec<f32>,
}

impl StreamingWsola {
    /// Speech-tuned WSOLA at `sample_rate`, with `speed > 1` playing faster.
    #[must_use]
    pub fn new(sample_rate: u32, speed: f32) -> Self {
        let speed = speed.clamp(0.5, 3.0);
        let ratio = 1.0 / f64::from(speed);
        let segment = (f64::from(sample_rate) * 0.030).round() as usize;
        let search = (f64::from(sample_rate) * 0.015).round() as usize;
        let crossfade_out = (f64::from(sample_rate) * XFADE_MS / 1000.0).round() as usize;
        Self {
            wsola: Wsola::new(segment.max(1), search.max(1), ratio),
            overlap_in: (crossfade_out as f64 / ratio).ceil() as usize,
            crossfade_out: crossfade_out.max(1),
            previous_input_tail: Vec::new(),
            held_output_tail: Vec::new(),
        }
    }

    /// Stretch the next consecutive fresh input chunk.
    pub fn process(&mut self, fresh: &[f32]) -> Result<Vec<f32>, WsolaError> {
        if fresh.is_empty() {
            return Ok(Vec::new());
        }
        let mut input = Vec::with_capacity(self.previous_input_tail.len() + fresh.len());
        input.extend_from_slice(&self.previous_input_tail);
        input.extend_from_slice(fresh);
        if input.len() < self.wsola.segment_size() {
            input.resize(self.wsola.segment_size(), 0.0);
        }

        let mut output = self.wsola.process(&input)?;
        let blend = self.held_output_tail.len().min(output.len());
        for (index, sample) in output.iter_mut().take(blend).enumerate() {
            let mix = (index as f32 + 0.5) / blend as f32;
            *sample = self.held_output_tail[index] * (1.0 - mix) + *sample * mix;
        }

        let hold = self.crossfade_out.min(output.len());
        self.held_output_tail = output.split_off(output.len() - hold);
        let tail_from = input.len().saturating_sub(self.overlap_in);
        self.previous_input_tail.clear();
        self.previous_input_tail
            .extend_from_slice(&input[tail_from..]);
        Ok(output)
    }

    /// Release the final crossfade tail once input has ended.
    pub fn finish(&mut self) -> Vec<f32> {
        self.previous_input_tail.clear();
        std::mem::take(&mut self.held_output_tail)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn consecutive_chunks_and_finish_produce_finite_audio() {
        let mut stream = StreamingWsola::new(16_000, 1.5);
        let a: Vec<f32> = (0..4096).map(|i| ((i as f32) * 0.01).sin()).collect();
        let b: Vec<f32> = (4096..8192).map(|i| ((i as f32) * 0.01).sin()).collect();
        let mut out = stream.process(&a).unwrap();
        out.extend(stream.process(&b).unwrap());
        out.extend(stream.finish());
        assert!(!out.is_empty());
        assert!(out.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn empty_input_is_a_noop() {
        let mut stream = StreamingWsola::new(44_100, 2.0);
        assert!(stream.process(&[]).unwrap().is_empty());
        assert!(stream.finish().is_empty());
    }

    #[test]
    fn boundary_crossfade_keeps_the_same_duration_at_tera_sample_rate() {
        let stream = StreamingWsola::new(44_100, 1.5);
        assert_eq!(stream.crossfade_out, 706);
    }
}
