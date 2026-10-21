//! PipeWire capture through its native PulseAudio transport.
//! Each stream owns its parec process; PCM is 16 kHz mono s16le throughout.

use crate::audio_metrics::{AudioMetrics, AudioMetricsSnapshot, Stream};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub const TARGET_SAMPLE_RATE: u32 = 16_000;
const CHUNK: usize = 3200;
const MAX_RECORDING: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    System,
    Mic,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptLine {
    pub source: AudioSource,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub source: AudioSource,
    pub pcm_i16: Vec<i16>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DeviceList {
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCaptureState {
    Pending,
    Running,
    Failed,
}

#[derive(Deserialize)]
struct Device {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    monitor_source: serde_json::Value,
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn command(executable: &str) -> Command {
    let mut cmd = Command::new(executable);
    crate::download::no_window(&mut cmd);
    // Capture tools must not survive an abrupt host termination either.
    let parent = std::process::id() as libc::pid_t;
    unsafe {
        cmd.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                libc::_exit(1);
            }
            Ok(())
        });
    }
    cmd
}

fn pactl(args: &[&str]) -> Result<Vec<u8>> {
    // timeout owns its process group and bounds inspection even if the server hangs.
    let out = command("timeout")
        .args(["--signal=KILL", "2s", "pactl"])
        .args(args)
        .stderr(Stdio::null())
        .output()
        .context("audio server inspection failed")?;
    if !out.status.success() || out.stdout.len() > 1024 * 1024 {
        bail!("audio server inspection unavailable");
    }
    Ok(out.stdout)
}

fn devices(source: AudioSource) -> Result<Vec<Device>> {
    let kind = if source == AudioSource::System {
        "sinks"
    } else {
        "sources"
    };
    serde_json::from_slice(&pactl(&["-f", "json", "list", kind])?)
        .context("invalid audio device response")
}

fn select_device<'a>(devices: &'a [Device], selected: &str) -> Result<&'a Device> {
    let mut matches = devices
        .iter()
        .filter(|d| d.name == selected || d.description == selected);
    let device = matches
        .next()
        .ok_or_else(|| anyhow!("selected audio device unavailable"))?;
    if matches.next().is_some() {
        bail!("audio device name is ambiguous");
    }
    Ok(device)
}

fn resolve(source: AudioSource, selected: Option<&str>) -> Result<(String, String)> {
    let mut all = devices(source)?;
    let source = if source == AudioSource::System
        && selected.is_some_and(|name| !all.iter().any(|d| d.name == name || d.description == name))
    {
        // A selected system stream may be a headset mix/capture input. Never
        // replace a missing pinned choice with the default output monitor.
        all = devices(AudioSource::Mic)?;
        AudioSource::Mic
    } else {
        source
    };
    let default;
    let name = match selected.filter(|s| !s.trim().is_empty()) {
        Some(name) => name,
        None => {
            let action = if source == AudioSource::System {
                "get-default-sink"
            } else {
                "get-default-source"
            };
            default =
                String::from_utf8(pactl(&[action])?).context("invalid default audio device")?;
            default.trim()
        }
    };
    let device = select_device(&all, name)?;
    let capture = if source == AudioSource::System {
        device
            .monitor_source
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}.monitor", device.name))
    } else {
        device.name.clone()
    };
    Ok((device.name.clone(), capture))
}

pub fn list_devices() -> Result<DeviceList> {
    Ok(DeviceList {
        outputs: devices(AudioSource::System)?
            .into_iter()
            .map(|d| d.name)
            .collect(),
        inputs: devices(AudioSource::Mic)?
            .into_iter()
            .filter(|d| !d.name.ends_with(".monitor"))
            .map(|d| d.name)
            .collect(),
    })
}

fn nonblocking(stdout: &ChildStdout) -> Result<()> {
    let fd = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

struct Capture {
    child: OwnedChild,
    stdout: ChildStdout,
    route: String,
    tail: Option<u8>,
}

impl Capture {
    fn open(route: String) -> Result<Self> {
        let mut child = OwnedChild(
            command("parec")
                .args([
                    "--raw",
                    "--format=s16le",
                    "--rate=16000",
                    "--channels=1",
                    "--latency-msec=100",
                    "--client-name=Suflyor",
                    "--device",
                    &route,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .context("audio capture tool unavailable")?,
        );
        let stdout = child
            .0
            .stdout
            .take()
            .ok_or_else(|| anyhow!("audio capture pipe unavailable"))?;
        nonblocking(&stdout)?;
        Ok(Self {
            child,
            stdout,
            route,
            tail: None,
        })
    }

    fn read(&mut self, into: &mut Vec<i16>) -> Result<()> {
        let mut bytes = [0u8; 8192];
        match self.stdout.read(&mut bytes) {
            Ok(0) => bail!("audio capture stream closed"),
            Ok(n) => decode(&bytes[..n], &mut self.tail, into),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if self.child.0.try_wait()?.is_some() {
                    bail!("audio capture process stopped");
                }
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }
}

fn decode(bytes: &[u8], tail: &mut Option<u8>, samples: &mut Vec<i16>) {
    let mut bytes = bytes;
    if let Some(low) = tail.take() {
        if let Some((&high, rest)) = bytes.split_first() {
            samples.push(i16::from_le_bytes([low, high]));
            bytes = rest;
        } else {
            *tail = Some(low);
            return;
        }
    }
    let (pairs, remainder) = bytes.as_chunks::<2>();
    samples.extend(pairs.iter().map(|pair| i16::from_le_bytes(*pair)));
    *tail = remainder.first().copied();
}

pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
    system: Arc<AtomicU8>,
    metrics: Arc<AudioMetrics>,
}
impl CaptureHandle {
    pub fn metrics_snapshot(&self) -> AudioMetricsSnapshot {
        self.metrics.snapshot()
    }
    pub fn system_capture_state(&self) -> SystemCaptureState {
        match self.system.load(Ordering::Acquire) {
            0 => SystemCaptureState::Pending,
            1 => SystemCaptureState::Running,
            _ => SystemCaptureState::Failed,
        }
    }
}
impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

pub fn start_capture(
    mic_device: Option<String>,
    sys_device: Option<String>,
) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)> {
    // Resolve at least one actual route before claiming a capture session can start.
    if resolve(AudioSource::Mic, mic_device.as_deref()).is_err()
        && resolve(AudioSource::System, sys_device.as_deref()).is_err()
    {
        bail!("no audio input or output route is available");
    }
    let (tx, rx) = mpsc::channel(128);
    let mut handle = CaptureHandle {
        stop: Arc::new(AtomicBool::new(false)),
        workers: Vec::new(),
        system: Arc::new(AtomicU8::new(0)),
        metrics: Arc::new(AudioMetrics::default()),
    };
    let start = Instant::now();
    for (source, selected) in [
        (AudioSource::Mic, mic_device),
        (AudioSource::System, sys_device),
    ] {
        let (stop, system, metrics, tx) = (
            handle.stop.clone(),
            handle.system.clone(),
            handle.metrics.clone(),
            tx.clone(),
        );
        let worker = thread::Builder::new()
            .name(format!("audio-{source:?}"))
            .spawn(move || capture_worker(source, selected, tx, stop, system, metrics, start))?;
        handle.workers.push(worker);
    }
    Ok((rx, handle))
}

fn capture_worker(
    source: AudioSource,
    selected: Option<String>,
    tx: mpsc::Sender<AudioChunk>,
    stop: Arc<AtomicBool>,
    system: Arc<AtomicU8>,
    metrics: Arc<AudioMetrics>,
    start: Instant,
) {
    let stream = if source == AudioSource::Mic {
        Stream::Mic
    } else {
        Stream::System
    };
    let mut capture: Option<Capture> = None;
    let mut samples = Vec::new();
    let mut next_route = Instant::now();
    while !stop.load(Ordering::Acquire) && !tx.is_closed() {
        if Instant::now() >= next_route {
            next_route = Instant::now() + Duration::from_secs(1);
            let desired = resolve(source, selected.as_deref());
            match desired {
                Ok((_, route)) if capture.as_ref().is_none_or(|c| c.route != route) => {
                    drop(capture.take());
                    samples.clear();
                    capture = Capture::open(route).ok();
                }
                Err(_) => {
                    capture = None;
                    samples.clear();
                }
                _ => {}
            }
            if source == AudioSource::System && capture.is_none() {
                system.store(0, Ordering::Release);
            }
        }
        if let Some(active) = capture.as_mut() {
            if active.read(&mut samples).is_err() {
                capture = None;
                samples.clear();
                if source == AudioSource::System {
                    system.store(0, Ordering::Release);
                }
            }
        }
        metrics.observe_pending_samples(stream, samples.len() as u64);
        while samples.len() >= CHUNK {
            if source == AudioSource::System {
                system.store(1, Ordering::Release);
            }
            let timestamp_ms = start.elapsed().as_millis() as u64;
            let chunk = AudioChunk {
                source,
                pcm_i16: samples.drain(..CHUNK).collect(),
                timestamp_ms,
            };
            match tx.try_send(chunk) {
                Ok(()) => metrics.record_emitted_chunk(stream, timestamp_ms),
                Err(mpsc::error::TrySendError::Full(_)) => metrics.record_queue_drop(stream),
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    stop.store(true, Ordering::Release);
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    if source == AudioSource::System {
        system.store(2, Ordering::Release);
    }
}

fn record(
    source: AudioSource,
    mic: Option<String>,
    sys: Option<String>,
    stop: Arc<AtomicBool>,
    duration: Duration,
) -> Result<Vec<i16>> {
    if stop.load(Ordering::Acquire) || duration.is_zero() {
        return Ok(Vec::new());
    }
    let selected = if source == AudioSource::Mic { mic } else { sys };
    let (_, route) = resolve(source, selected.as_deref())?;
    if stop.load(Ordering::Acquire) {
        return Ok(Vec::new());
    }
    let mut capture = Capture::open(route)?;
    let start = Instant::now();
    let mut samples = Vec::new();
    while !stop.load(Ordering::Acquire) && start.elapsed() < duration.min(MAX_RECORDING) {
        capture.read(&mut samples)?;
        thread::sleep(Duration::from_millis(10));
    }
    Ok(samples)
}

pub fn record_source_until_stop(
    source: AudioSource,
    mic: Option<String>,
    sys: Option<String>,
    stop: Arc<AtomicBool>,
) -> Result<Vec<i16>> {
    record(source, mic, sys, stop, MAX_RECORDING)
}
pub fn record_sys_blocking(duration_ms: u64, device: Option<String>) -> Result<Vec<i16>> {
    record(
        AudioSource::System,
        None,
        device,
        Arc::new(AtomicBool::new(false)),
        Duration::from_millis(duration_ms),
    )
}
pub fn record_mic_blocking(duration_ms: u64, device: Option<String>) -> Result<Vec<i16>> {
    record(
        AudioSource::Mic,
        device,
        None,
        Arc::new(AtomicBool::new(false)),
        Duration::from_millis(duration_ms),
    )
}
#[must_use]
pub fn rms_dbfs(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return f64::NEG_INFINITY;
    }
    let power = samples
        .iter()
        .map(|s| (f64::from(*s) / 32768.0).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    10.0 * power.log10()
}

pub fn play_tone_and_capture(device: Option<String>) -> Result<Vec<i16>> {
    let (selected, route) = resolve(AudioSource::System, device.as_deref())?;
    let sink = if devices(AudioSource::System)?
        .iter()
        .any(|d| d.name == selected)
    {
        selected
    } else {
        String::from_utf8(pactl(&["get-default-sink"])?)?
            .trim()
            .to_owned()
    };
    let mut capture = Capture::open(route)?;
    let mut player = OwnedChild(
        command("paplay")
            .args([
                "--raw",
                "--format=s16le",
                "--rate=16000",
                "--channels=1",
                "--device",
                &sink,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut stdin = player
        .0
        .stdin
        .take()
        .ok_or_else(|| anyhow!("audio playback pipe unavailable"))?;
    let writer = thread::spawn(move || {
        let pcm: Vec<u8> = (0..32000)
            .flat_map(|n| {
                let value =
                    ((n as f32 * 440.0 * std::f32::consts::TAU / 16000.0).sin() * 12000.0) as i16;
                value.to_le_bytes()
            })
            .collect();
        stdin.write_all(&pcm)
    });
    let start = Instant::now();
    let mut samples = Vec::new();
    let result = (|| -> Result<()> {
        while start.elapsed() < Duration::from_millis(2500) {
            capture.read(&mut samples)?;
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    })();
    drop(player);
    let written = writer
        .join()
        .map_err(|_| anyhow!("audio tone writer failed"))?;
    result?;
    written?;
    Ok(samples)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn pcm_keeps_partial_bytes() {
        let (mut tail, mut samples) = (None, Vec::new());
        decode(&[1], &mut tail, &mut samples);
        decode(&[], &mut tail, &mut samples);
        decode(&[2, 3, 4, 5], &mut tail, &mut samples);
        assert_eq!(samples, [513, 1027]);
        assert_eq!(tail, Some(5));
    }
    #[test]
    fn pinned_selection_does_not_fallback_and_rejects_ambiguity() {
        let all: Vec<Device> = serde_json::from_str(
            r#"[{"name":"one","description":"Mic"},{"name":"two","description":"Mic"}]"#,
        )
        .unwrap();
        assert_eq!(select_device(&all, "one").unwrap().name, "one");
        assert!(select_device(&all, "missing").is_err());
        assert!(select_device(&all, "Mic").is_err());
    }
    #[test]
    fn precancelled_capture_never_starts_tools() {
        for source in [AudioSource::Mic, AudioSource::System] {
            assert!(
                record_source_until_stop(source, None, None, Arc::new(AtomicBool::new(true)))
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(record_mic_blocking(0, None).unwrap().is_empty());
    }
    #[test]
    fn rms_reports_signal_and_silence() {
        assert_eq!(rms_dbfs(&[]), f64::NEG_INFINITY);
        assert_eq!(rms_dbfs(&[0; 10]), f64::NEG_INFINITY);
        assert!((rms_dbfs(&[16384, -16384]) + 6.0206).abs() < 0.001);
    }
    #[test]
    #[ignore = "requires dedicated PipeWire worker; records real system loopback"]
    fn live_tone_loopback() {
        let pcm = play_tone_and_capture(None).unwrap();
        assert!(pcm.len() > 16000, "short capture: {}", pcm.len());
        assert!(rms_dbfs(&pcm) > -30.0, "tone absent");
    }
    #[test]
    #[ignore = "requires dedicated PipeWire worker with microphone fixture"]
    fn live_system_capture_input_route() {
        let input = String::from_utf8(pactl(&["get-default-source"]).unwrap()).unwrap();
        let input = input.trim().to_owned();
        let (_, route) = resolve(AudioSource::System, Some(&input)).unwrap();
        assert_eq!(route, input);
        let samples = record_sys_blocking(500, Some(input)).unwrap();
        assert!(
            samples.len() >= 3200,
            "selected system input produced no PCM"
        );
    }
    #[test]
    #[ignore = "requires dedicated PipeWire worker with microphone fixture"]
    fn live_dual_capture_and_cleanup() {
        let (mut rx, handle) = start_capture(None, None).unwrap();
        let start = Instant::now();
        let (mut mic, mut sys) = (0, 0);
        while start.elapsed() < Duration::from_secs(4) {
            if let Ok(c) = rx.try_recv() {
                match c.source {
                    AudioSource::Mic => mic += c.pcm_i16.len(),
                    AudioSource::System => sys += c.pcm_i16.len(),
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(mic >= 16000 && sys >= 16000, "missing streams: {mic}/{sys}");
        let start = Instant::now();
        drop(handle);
        assert!(start.elapsed() < Duration::from_secs(5));
        while rx.try_recv().is_ok() {}
        assert!(rx.is_closed());
    }
}
