//! WASAPI render transport for synthesized speech (sidecar copy).
//!
//! One render thread plays a growing stream of mono f32 samples (fed chunk by
//! chunk as the engine synthesizes) to the default output device, with live
//! pause/resume and stop. Declared format is mono f32 at the synth rate; WASAPI
//! shared-mode `autoconvert` resamples to the device.

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

/// Handle to a running render thread. Drop or `stop` joins it.
pub struct Playback {
    feed_tx: Sender<Vec<f32>>,
    eos: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Playback {
    pub fn start(sample_rate: u32) -> Result<Self> {
        let (feed_tx, feed_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let eos = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));

        let (eos2, stop2, paused2) = (eos.clone(), stop.clone(), paused.clone());
        let handle = std::thread::Builder::new()
            .name("tts-playback".into())
            .spawn(move || {
                if let Err(e) = render_loop(sample_rate, feed_rx, eos2, stop2, paused2) {
                    eprintln!("[suflyor-tts] playback render loop ended: {e:#}");
                }
            })
            .map_err(|e| anyhow!("spawn playback thread: {e}"))?;

        Ok(Self {
            feed_tx,
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

    let mut queue: VecDeque<f32> = VecDeque::new();
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
            drain_feed(&feed_rx, &mut queue);
            continue;
        }
        drain_feed(&feed_rx, &mut queue);

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

        if queue.is_empty() {
            if eos.load(Ordering::Acquire) && padding == 0 {
                break;
            }
            let _ = render_client.write_to_device(avail, &silence[..avail * 8], None);
            continue;
        }

        let take = avail.min(queue.len());
        // Duplicate each mono sample into an L+R stereo frame.
        let mut buf: Vec<f32> = Vec::with_capacity(take * 2);
        for _ in 0..take {
            if let Some(s) = queue.pop_front() {
                buf.push(s);
                buf.push(s);
            }
        }
        let bytes = samples_to_bytes(&buf);
        let _ = render_client.write_to_device(take, &bytes, None);
    }

    let _ = client.stop_stream();
    Ok(())
}

fn drain_feed(feed_rx: &Receiver<Vec<f32>>, queue: &mut VecDeque<f32>) {
    while let Ok(chunk) = feed_rx.try_recv() {
        queue.extend(chunk);
    }
}
