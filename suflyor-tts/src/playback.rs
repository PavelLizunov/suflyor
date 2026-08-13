//! WASAPI render transport for synthesized speech (sidecar copy).
//!
//! One render thread plays a growing stream of mono f32 samples (fed chunk by
//! chunk as the engine synthesizes) to the default output device, with live
//! pause/resume and stop. The render client declares stereo f32 and duplicates
//! each synthesized mono sample to L+R, so shared-mode WASAPI plays both ears.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::{anyhow, Result};
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

fn samples_to_bytes(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 4);
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmptyQueueAction {
    Finish,
    Drain,
    WriteSilence,
}

/// Once synthesis reached EOS, letting WASAPI drain must not write a new
/// silence buffer: that would keep device padding non-zero forever.
fn empty_queue_action(end_of_stream: bool, padding: usize) -> EmptyQueueAction {
    match (end_of_stream, padding) {
        (true, 0) => EmptyQueueAction::Finish,
        (true, _) => EmptyQueueAction::Drain,
        (false, _) => EmptyQueueAction::WriteSilence,
    }
}

/// Handle to a running render thread. Drop or `stop` joins it.
pub struct Playback {
    feed_tx: Sender<Vec<f32>>,
    control_tx: Sender<PlaybackControl>,
    eos: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

enum PlaybackControl {
    SeekSeconds(i32),
    SetSpeed(f32),
}

const BACK_HISTORY_SECONDS: u64 = 30;
const STRETCH_INPUT_CHUNK: usize = 4096;

struct BufferedTimeline {
    samples: VecDeque<f32>,
    start: u64,
    cursor: u64,
}

impl BufferedTimeline {
    fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            start: 0,
            cursor: 0,
        }
    }
    fn end(&self) -> u64 {
        self.start.saturating_add(self.samples.len() as u64)
    }
    fn append(&mut self, samples: Vec<f32>) {
        self.samples.extend(samples);
    }
    fn available(&self) -> usize {
        self.end().saturating_sub(self.cursor) as usize
    }
    fn take(&mut self, count: usize) -> Vec<f32> {
        let count = count.min(self.available());
        let offset = self.cursor.saturating_sub(self.start) as usize;
        let out = self
            .samples
            .iter()
            .skip(offset)
            .take(count)
            .copied()
            .collect();
        self.cursor = self.cursor.saturating_add(count as u64);
        out
    }
    fn seek_seconds(&mut self, seconds: i32, sample_rate: u32) {
        let delta = i64::from(seconds).saturating_mul(i64::from(sample_rate));
        let target = if delta.is_negative() {
            self.cursor.saturating_sub(delta.unsigned_abs())
        } else {
            self.cursor.saturating_add(delta as u64)
        };
        self.cursor = target.clamp(self.start, self.end());
    }
    fn rewind_to(&mut self, checkpoint: u64) {
        self.cursor = checkpoint.clamp(self.start, self.end());
    }
    fn prune_played_history(&mut self, sample_rate: u32) {
        let keep = u64::from(sample_rate).saturating_mul(BACK_HISTORY_SECONDS);
        let drop = self.cursor.saturating_sub(self.start).saturating_sub(keep);
        for _ in 0..drop.min(self.samples.len() as u64) {
            self.samples.pop_front();
            self.start = self.start.saturating_add(1);
        }
    }
}

impl Playback {
    pub fn start(sample_rate: u32) -> Result<Self> {
        let (feed_tx, feed_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let (control_tx, control_rx) = std::sync::mpsc::channel::<PlaybackControl>();
        let eos = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));

        let (eos2, stop2, paused2) = (eos.clone(), stop.clone(), paused.clone());
        let handle = std::thread::Builder::new()
            .name("tts-playback".into())
            .spawn(move || {
                if let Err(e) = render_loop(sample_rate, feed_rx, control_rx, eos2, stop2, paused2)
                {
                    eprintln!("[suflyor-tts] playback render loop ended: {e:#}");
                }
            })
            .map_err(|e| anyhow!("spawn playback thread: {e}"))?;

        Ok(Self {
            feed_tx,
            control_tx,
            eos,
            stop,
            paused,
            handle: Some(handle),
        })
    }

    pub fn feed(&self, samples: Vec<f32>) {
        if samples.is_empty() {
            return;
        }
        let _ = self.feed_tx.send(samples);
    }

    pub fn end_of_stream(&self) {
        self.eos.store(true, Ordering::Release);
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Release);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Release);
    }

    pub fn seek_seconds(&self, seconds: i32) {
        let _ = self.control_tx.send(PlaybackControl::SeekSeconds(seconds));
    }

    pub fn set_speed(&self, speed: f32) {
        let _ = self.control_tx.send(PlaybackControl::SetSpeed(speed));
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn render_loop(
    sample_rate: u32,
    feed_rx: Receiver<Vec<f32>>,
    control_rx: Receiver<PlaybackControl>,
    eos: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) -> Result<()> {
    let _ = wasapi::initialize_mta();
    let device = DeviceEnumerator::new()
        .map_err(|e| anyhow!("device enumerator: {e}"))?
        .get_default_device(&Direction::Render)
        .map_err(|e| anyhow!("default render device: {e}"))?;
    let mut client = device
        .get_iaudioclient()
        .map_err(|e| anyhow!("iaudioclient: {e}"))?;
    let (_def, min_period) = client
        .get_device_period()
        .map_err(|e| anyhow!("device period: {e}"))?;
    // Declare STEREO and duplicate each mono sample to L+R. Declaring mono and
    // relying on WASAPI's mono→stereo upmix routed audio to a single channel on
    // some devices (one-ear playback); explicit stereo plays in both ears.
    let desired = WaveFormat::new(32, 32, &SampleType::Float, sample_rate as usize, 2, None);
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_period,
    };
    client
        .initialize_client(&desired, &Direction::Render, &mode)
        .map_err(|e| anyhow!("initialize_client: {e}"))?;
    let event = client
        .set_get_eventhandle()
        .map_err(|e| anyhow!("event handle: {e}"))?;
    let render_client = client
        .get_audiorenderclient()
        .map_err(|e| anyhow!("render client: {e}"))?;
    let buffer_frames = client
        .get_buffer_size()
        .map_err(|e| anyhow!("buffer size: {e}"))? as usize;

    let mut timeline = BufferedTimeline::new();
    let mut output: VecDeque<f32> = VecDeque::new();
    let mut output_checkpoint: Option<u64> = None;
    let mut speed = 1.0_f32;
    let mut stretcher: Option<suflyor_wsola::StreamingWsola> = None;
    let mut stretch_finished = false;
    // Stereo silence: 2 samples (L+R) per frame.
    let silence = samples_to_bytes(&vec![0.0_f32; buffer_frames * 2]);

    client
        .start_stream()
        .map_err(|e| anyhow!("start_stream: {e}"))?;

    loop {
        if stop.load(Ordering::Acquire) {
            break;
        }
        if event.wait_for_event(200).is_err() {
            drain_feed(&feed_rx, &mut timeline);
            drain_controls(
                &control_rx,
                sample_rate,
                &mut timeline,
                &mut output,
                &mut output_checkpoint,
                &mut speed,
                &mut stretcher,
                &mut stretch_finished,
            );
            continue;
        }
        drain_feed(&feed_rx, &mut timeline);
        drain_controls(
            &control_rx,
            sample_rate,
            &mut timeline,
            &mut output,
            &mut output_checkpoint,
            &mut speed,
            &mut stretcher,
            &mut stretch_finished,
        );

        let padding = match client.get_current_padding() {
            Ok(p) => p as usize,
            Err(_) => break,
        };
        let avail = buffer_frames.saturating_sub(padding);
        if avail == 0 {
            continue;
        }

        if paused.load(Ordering::Acquire) {
            let _ = render_client.write_to_device(avail, &silence[..avail * 8], None);
            continue;
        }

        if output.is_empty() {
            output_checkpoint = None;
            fill_output(
                sample_rate,
                speed,
                eos.load(Ordering::Acquire),
                &mut timeline,
                &mut output,
                &mut output_checkpoint,
                &mut stretcher,
                &mut stretch_finished,
            );
        }

        if output.is_empty() {
            match empty_queue_action(
                eos.load(Ordering::Acquire) && timeline.available() == 0,
                padding,
            ) {
                EmptyQueueAction::Finish => break,
                EmptyQueueAction::Drain => continue,
                EmptyQueueAction::WriteSilence => {
                    let _ = render_client.write_to_device(avail, &silence[..avail * 8], None);
                    continue;
                }
            }
        }

        let take = avail.min(output.len());
        // Duplicate each mono sample into an L+R stereo frame.
        let mut buf: Vec<f32> = Vec::with_capacity(take * 2);
        for _ in 0..take {
            if let Some(s) = output.pop_front() {
                buf.push(s);
                buf.push(s);
            }
        }
        let bytes = samples_to_bytes(&buf);
        let _ = render_client.write_to_device(take, &bytes, None);
        if output.is_empty() {
            output_checkpoint = None;
            timeline.prune_played_history(sample_rate);
        }
    }

    let _ = client.stop_stream();
    Ok(())
}

fn drain_feed(feed_rx: &Receiver<Vec<f32>>, timeline: &mut BufferedTimeline) {
    while let Ok(chunk) = feed_rx.try_recv() {
        timeline.append(chunk);
    }
}

#[allow(clippy::too_many_arguments)]
fn drain_controls(
    control_rx: &Receiver<PlaybackControl>,
    sample_rate: u32,
    timeline: &mut BufferedTimeline,
    output: &mut VecDeque<f32>,
    output_checkpoint: &mut Option<u64>,
    speed: &mut f32,
    stretcher: &mut Option<suflyor_wsola::StreamingWsola>,
    stretch_finished: &mut bool,
) {
    while let Ok(control) = control_rx.try_recv() {
        if let Some(checkpoint) = output_checkpoint.take() {
            timeline.rewind_to(checkpoint);
        }
        output.clear();
        *stretcher = None;
        *stretch_finished = false;
        match control {
            PlaybackControl::SeekSeconds(seconds) => {
                timeline.seek_seconds(seconds, sample_rate);
            }
            PlaybackControl::SetSpeed(next) => {
                *speed = next.clamp(0.5, 3.0);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_output(
    sample_rate: u32,
    speed: f32,
    eos: bool,
    timeline: &mut BufferedTimeline,
    output: &mut VecDeque<f32>,
    output_checkpoint: &mut Option<u64>,
    stretcher: &mut Option<suflyor_wsola::StreamingWsola>,
    stretch_finished: &mut bool,
) {
    let available = timeline.available();
    if available == 0 {
        if eos && !*stretch_finished {
            if let Some(active) = stretcher.as_mut() {
                output.extend(active.finish());
            }
            *stretch_finished = true;
        }
        return;
    }

    *output_checkpoint = Some(timeline.cursor);
    if (speed - 1.0).abs() < f32::EPSILON {
        output.extend(timeline.take(available.min(STRETCH_INPUT_CHUNK)));
        return;
    }
    if available < STRETCH_INPUT_CHUNK && !eos {
        *output_checkpoint = None;
        return;
    }
    let fresh = timeline.take(available.min(STRETCH_INPUT_CHUNK));
    let active =
        stretcher.get_or_insert_with(|| suflyor_wsola::StreamingWsola::new(sample_rate, speed));
    match active.process(&fresh) {
        Ok(stretched) => output.extend(stretched),
        Err(_) => output.extend(fresh),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn end_of_stream_drains_without_refilling_silence() {
        assert_eq!(empty_queue_action(false, 0), EmptyQueueAction::WriteSilence);
        assert_eq!(empty_queue_action(true, 8), EmptyQueueAction::Drain);
        assert_eq!(empty_queue_action(true, 0), EmptyQueueAction::Finish);
    }

    #[test]
    fn seek_clamps_to_retained_history_and_buffered_horizon() {
        let mut timeline = BufferedTimeline::new();
        timeline.append((0..100).map(|sample| sample as f32).collect());
        assert_eq!(timeline.take(60).len(), 60);
        timeline.seek_seconds(-10, 5);
        assert_eq!(timeline.cursor, 10);
        timeline.seek_seconds(15, 5);
        assert_eq!(timeline.cursor, 85);
        timeline.seek_seconds(15, 5);
        assert_eq!(timeline.cursor, 100);
    }

    #[test]
    fn played_history_is_bounded_without_discarding_future_audio() {
        let mut timeline = BufferedTimeline::new();
        timeline.append(vec![0.0; 200]);
        timeline.take(180);
        timeline.prune_played_history(5);
        assert_eq!(timeline.start, 30);
        assert_eq!(timeline.end(), 200);
    }
}
