//! CoreAudio render transport for synthesized speech via `cpal` (macOS sidecar).
//!
//! Continuous linear interpolation resampler + pre-buffered ring queue to prevent
//! buffer underruns, popping, or phase discontinuities.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmptyQueueAction {
    Finish,
    Drain,
    WriteSilence,
}

fn empty_queue_action(end_of_stream: bool, pending_samples: usize) -> EmptyQueueAction {
    match (end_of_stream, pending_samples) {
        (true, 0) => EmptyQueueAction::Finish,
        (true, _) => EmptyQueueAction::Drain,
        (false, _) => EmptyQueueAction::WriteSilence,
    }
}

pub struct Playback {
    feed_tx: Sender<Vec<f32>>,
    control_tx: Sender<PlaybackControl>,
    eos: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

enum PlaybackControl {
    SeekSeconds(i32),
    SetSpeed(f32),
}

struct ExitNotifier {
    notify: Option<Box<dyn FnOnce() + Send>>,
    enabled: Arc<AtomicBool>,
}

impl Drop for ExitNotifier {
    fn drop(&mut self) {
        if self.enabled.load(Ordering::Acquire) {
            if let Some(notify) = self.notify.take() {
                notify();
            }
        }
    }
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
    fn available(&self) -> usize {
        self.start
            .saturating_add(self.samples.len() as u64)
            .saturating_sub(self.cursor) as usize
    }
    fn append(&mut self, samples: Vec<f32>) {
        self.samples.extend(samples);
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
        let end = self.start.saturating_add(self.samples.len() as u64);
        let target = if delta.is_negative() {
            self.cursor.saturating_sub(delta.unsigned_abs())
        } else {
            self.cursor.saturating_add(delta as u64)
        };
        self.cursor = target.clamp(self.start, end);
    }
    fn rewind_to(&mut self, checkpoint: u64) {
        let end = self.start.saturating_add(self.samples.len() as u64);
        self.cursor = checkpoint.clamp(self.start, end);
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
    pub fn start(sample_rate: u32, on_exit: Option<Box<dyn FnOnce() + Send>>) -> Result<Self> {
        let (feed_tx, feed_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let (control_tx, control_rx) = std::sync::mpsc::channel::<PlaybackControl>();
        let (startup_tx, startup_rx) = std::sync::mpsc::channel::<Result<()>>();
        let eos = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let startup_ok = Arc::new(AtomicBool::new(false));

        let (eos2, stop2, paused2, failed2, startup_ok2) = (
            eos.clone(),
            stop.clone(),
            paused.clone(),
            failed.clone(),
            startup_ok.clone(),
        );
        let mut handle = Some(
            std::thread::Builder::new()
                .name("tts-playback".into())
                .spawn(move || {
                    let _exit_notifier = ExitNotifier {
                        notify: on_exit,
                        enabled: startup_ok2.clone(),
                    };
                    let result = render_loop(
                        sample_rate,
                        feed_rx,
                        control_rx,
                        eos2,
                        stop2,
                        paused2,
                        startup_tx,
                        startup_ok2,
                    );
                    if let Err(e) = result {
                        eprintln!("[suflyor-tts] macOS playback error: {e:#}");
                        failed2.store(true, Ordering::Release);
                    }
                })?,
        );

        match startup_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => {
                startup_ok.store(true, Ordering::Release);
                Ok(Self {
                    feed_tx,
                    control_tx,
                    eos,
                    stop,
                    paused,
                    failed,
                    handle,
                })
            }
            Ok(Err(err)) => {
                stop.store(true, Ordering::Release);
                if let Some(h) = handle.take() {
                    let _ = h.join();
                }
                Err(anyhow!("playback startup failed: {err:#}"))
            }
            Err(err) => {
                stop.store(true, Ordering::Release);
                if let Some(h) = handle.take() {
                    if h.is_finished() { let _ = h.join(); }
                    // Native initialization cannot be interrupted. It observes stop
                    // when it returns; this isolated sidecar remains responsive.
                }
                Err(anyhow!("playback startup timed out: {err}"))
            }
        }
    }

    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
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

/// Continuous fractional-phase linear resampler. Maintains exact phase `src_phase`
/// and previous edge sample across consecutive processing blocks.
struct ContinuousResampler {
    src_rate: u32,
    dst_rate: u32,
    src_phase: f64,
    prev_sample: f32,
}

impl ContinuousResampler {
    fn new(src_rate: u32, dst_rate: u32) -> Self {
        Self {
            src_rate,
            dst_rate,
            src_phase: 0.0,
            prev_sample: 0.0,
        }
    }

    fn process(&mut self, input: &[f32], output: &mut VecDeque<f32>) {
        if input.is_empty() {
            return;
        }
        if self.src_rate == self.dst_rate || self.src_rate == 0 || self.dst_rate == 0 {
            output.extend(input.iter().copied());
            if let Some(&last) = input.last() {
                self.prev_sample = last;
            }
            return;
        }

        let ratio = self.dst_rate as f64 / self.src_rate as f64;
        let step = 1.0 / ratio;

        while self.src_phase < input.len() as f64 {
            let idx0_i = self.src_phase.floor() as i64;
            let frac = (self.src_phase - idx0_i as f64) as f32;

            let s0 = if idx0_i < 0 {
                self.prev_sample
            } else {
                input[idx0_i as usize]
            };

            let idx1 = (idx0_i + 1) as usize;
            let s1 = if idx1 < input.len() { input[idx1] } else { s0 };

            output.push_back(s0 + frac * (s1 - s0));
            self.src_phase += step;
        }

        self.src_phase -= input.len() as f64;
        if let Some(&last) = input.last() {
            self.prev_sample = last;
        }
    }
}

fn choose_f32_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig> {
    if let Ok(default) = device.default_output_config() {
        if default.sample_format() == cpal::SampleFormat::F32 {
            return Ok(default);
        }
    }
    let supported = device
        .supported_output_configs()
        .map_err(|e| anyhow!("supported output configs error: {e}"))?;
    let mut best: Option<cpal::SupportedStreamConfigRange> = None;
    for range in supported {
        if range.sample_format() == cpal::SampleFormat::F32 {
            best = Some(range);
            break;
        }
    }
    let range = best.ok_or_else(|| anyhow!("no supported F32 output configuration found"))?;
    if let Ok(default) = device.default_output_config() {
        let rate = default.sample_rate();
        if rate >= range.min_sample_rate() && rate <= range.max_sample_rate() {
            return Ok(range.with_sample_rate(rate));
        }
    }
    Ok(range.with_max_sample_rate())
}

fn render_loop(
    sample_rate: u32,
    feed_rx: Receiver<Vec<f32>>,
    control_rx: Receiver<PlaybackControl>,
    eos: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    startup_tx: Sender<Result<()>>,
    startup_ok: Arc<AtomicBool>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            let err = anyhow!("no audio default output device");
            let _ = startup_tx.send(Err(anyhow!("no audio default output device")));
            return Err(err);
        }
    };

    let supported_config = match choose_f32_config(&device) {
        Ok(c) => c,
        Err(e) => {
            let _ = startup_tx.send(Err(anyhow!("{e:#}")));
            return Err(e);
        }
    };
    let config: cpal::StreamConfig = supported_config.into();
    let channels = config.channels as usize;
    let device_rate = config.sample_rate.0;

    let mut timeline = BufferedTimeline::new();
    let mut output: VecDeque<f32> = VecDeque::new();
    let mut output_checkpoint: Option<u64> = None;
    let mut speed = 1.0_f32;
    let mut stretcher: Option<suflyor_wsola::StreamingWsola> = None;
    let mut stretch_finished = false;

    let pcm_queue = Arc::new(std::sync::Mutex::new(VecDeque::<f32>::new()));
    let pcm_queue_callback = Arc::clone(&pcm_queue);

    let stream = match device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut guard = pcm_queue_callback
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for frame in data.chunks_mut(channels) {
                let sample = guard.pop_front().unwrap_or(0.0);
                for out in frame.iter_mut() {
                    *out = sample;
                }
            }
        },
        |err| eprintln!("[suflyor-tts] cpal error: {err}"),
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            let err = anyhow!("build_output_stream: {e}");
            let _ = startup_tx.send(Err(anyhow!("build_output_stream: {e}")));
            return Err(err);
        }
    };

    if let Err(e) = stream.play() {
        let err = anyhow!("stream.play: {e}");
        let _ = startup_tx.send(Err(anyhow!("stream.play: {e}")));
        return Err(err);
    }

    startup_ok.store(true, Ordering::Release);
    let _ = startup_tx.send(Ok(()));

    let mut resampler = ContinuousResampler::new(sample_rate, device_rate);

    loop {
        if stop.load(Ordering::Acquire) {
            break;
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

        timeline.prune_played_history(sample_rate);

        let is_paused = paused.load(Ordering::Acquire);
        if is_paused {
            std::thread::sleep(std::time::Duration::from_millis(10));
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

        let pending_samples = {
            let guard = pcm_queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.len()
        };

        if output.is_empty() {
            let is_eos = eos.load(Ordering::Acquire) && timeline.available() == 0;
            match empty_queue_action(is_eos, pending_samples) {
                EmptyQueueAction::Finish => break,
                EmptyQueueAction::Drain | EmptyQueueAction::WriteSilence => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        } else {
            let mut guard = pcm_queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while guard.len() < 16384 && !output.is_empty() {
                let mut chunk = Vec::with_capacity(2048);
                while chunk.len() < 2048 && !output.is_empty() {
                    if let Some(s) = output.pop_front() {
                        chunk.push(s);
                    }
                }
                resampler.process(&chunk, &mut guard);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    drop(stream);
    Ok(())
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
    fn exit_notifier_runs_when_its_scope_ends() {
        let notified = Arc::new(AtomicBool::new(false));
        let flag = notified.clone();
        let enabled = Arc::new(AtomicBool::new(true));
        {
            let _notifier = ExitNotifier {
                notify: Some(Box::new(move || {
                    flag.store(true, Ordering::Release);
                })),
                enabled,
            };
        }
        assert!(notified.load(Ordering::Acquire));
    }

    #[test]
    fn exit_notifier_does_not_run_when_disabled() {
        let notified = Arc::new(AtomicBool::new(false));
        let flag = notified.clone();
        let enabled = Arc::new(AtomicBool::new(false));
        {
            let _notifier = ExitNotifier {
                notify: Some(Box::new(move || {
                    flag.store(true, Ordering::Release);
                })),
                enabled,
            };
        }
        assert!(!notified.load(Ordering::Acquire));
    }

    #[test]
    fn buffered_timeline_seek_and_take() {
        let mut timeline = BufferedTimeline::new();
        timeline.append(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(timeline.available(), 5);
        let taken = timeline.take(2);
        assert_eq!(taken, vec![1.0, 2.0]);
        assert_eq!(timeline.available(), 3);
        timeline.seek_seconds(-1, 1);
        assert_eq!(timeline.available(), 4);
    }
}
