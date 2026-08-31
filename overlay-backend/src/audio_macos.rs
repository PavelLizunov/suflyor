//! macOS audio seam — production runtime for BOTH sources. Microphone: REAL
//! capture via AVAudioEngine (default input only) with a synchronous start
//! report. System audio: REAL capture through the private Core Audio process
//! tap + aggregate device in `native/macos/system_capture.m` (same bounded
//! ring C ABI as the mic bridge), started asynchronously on its own worker
//! because first-run TCC consent can block `system_capture_start` or restart
//! the audio stack. Device enumeration / fixed-duration recording entry
//! points still fail fast with explicit unsupported errors (mirrors
//! `audio_unavailable.rs`).
//!
//! Architecture: tiny Objective-C bridges (`native/macos/mic_capture.m`,
//! `native/macos/system_capture.m`, compiled by `build.rs`) each own one
//! capture pipeline + a preallocated bounded SPSC f32 mono ring. The
//! realtime tap only downmixes into the ring and updates C11 atomics — no
//! Rust callback, allocation, lock, I/O, or logging in the tap. One named
//! Rust worker thread per source drains its ring every ~20 ms, accumulates
//! ~200 ms at the native rate, average-decimates to 16 kHz mono i16 (same
//! small algorithm as the Windows path, duplicated locally), and
//! `try_send`s `AudioChunk` into the shared Tokio channel (capacity 128,
//! non-blocking — mirroring the Windows drop-don't-park policy).
//!
//! The MICROPHONE permission is only ever
//! requested by `request_microphone_permission`, after an explicit user
//! action; mic start still fails clearly when TCC status is not determined,
//! denied, or restricted — but a failed mic start only logs its safe
//! message, and capture continues with the system source alone. The SYSTEM
//! audio TCC flow may be entered by `start_capture`, but only on the
//! `audio-sys` worker thread — never on the caller/UI thread. Capture
//! returns an immediate error only when the mic native start failed AND
//! the system WORKER THREAD could not be spawned; the asynchronous native
//! `system_capture_start` can still fail later — that failure is logged
//! and may leave the session with no active audio source.

use crate::audio_metrics::{AudioMetrics, AudioMetricsSnapshot, Stream};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::ffi::{c_char, c_void, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Target format for downstream STT (identical to the Windows implementation).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Tap buffer size requested from AVAudioEngine (frames).
const NATIVE_BUFFER_FRAMES: u32 = 1024;

/// Worker drain period — the ring is polled on this cadence.
const DRAIN_INTERVAL: Duration = Duration::from_millis(20);

/// AirPods/Core Audio route transitions can take a few seconds to settle.
const ROUTE_RESTART_DELAY: Duration = Duration::from_millis(500);

/// Native category-only start errors, mirrored from `mic_capture.m`.
const MIC_START_PERMISSION: i32 = 1;
const MIC_START_NO_INPUT: i32 = 2;

/// Native category-only start errors, mirrored from `system_capture.m`.
const SYS_START_TAP: i32 = 1;
const SYS_START_AGGREGATE: i32 = 2;
const SYS_START_FORMAT: i32 = 3;
const SYS_START_IO_PROC: i32 = 4;
const SYS_START_DEVICE: i32 = 5; // AudioDeviceStart refused (incl. TCC)
const SYS_START_NO_MEMORY: i32 = 6;

/// System worker lifecycle states — see `CaptureHandle::drop` for why the
/// pending state is the only one that is not joinable.
const SYSTEM_WORKER_PENDING: u8 = 0; // inside system_capture_start (may block in TCC)
const SYSTEM_WORKER_RUNNING: u8 = 1; // start returned; draining or tearing down
const SYSTEM_WORKER_FINISHED: u8 = 2; // worker returned; native tap already gone

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    /// What the other party says — private Core Audio process tap mirroring
    /// all system output on macOS; WASAPI loopback on Windows.
    System,
    /// What you say — default input device on macOS.
    Mic,
}

/// One line in the rolling session transcript (shared type — identical to the
/// Windows implementation).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptLine {
    pub source: AudioSource,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub source: AudioSource,
    /// 16 kHz mono i16 PCM samples.
    pub pcm_i16: Vec<i16>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DeviceList {
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
}

// --- Native bridge (mic_capture.m) -----------------------------------------

/// Opaque controller owned by the native side; only handled as a pointer.
#[repr(C)]
struct MicController {
    _opaque: [u8; 0],
}

extern "C" {
    fn mic_capture_permission_status() -> u32;
    fn mic_capture_request_permission(
        callback: extern "C" fn(u32, *mut c_void),
        context: *mut c_void,
    );
    fn mic_capture_start(
        buffer_frames: u32,
        out_sample_rate: *mut f64,
        out_error: *mut i32,
    ) -> *mut MicController;
    fn mic_capture_ring_capacity(controller: *const MicController) -> u32;
    fn mic_capture_read(controller: *mut MicController, dst: *mut f32, max_frames: u32) -> u32;
    fn mic_capture_take_dropped(controller: *mut MicController) -> u64;
    fn mic_capture_take_route_change(controller: *mut MicController) -> u32;
    fn mic_capture_stop(controller: *mut MicController);
    fn mic_capture_copy_default_input_name() -> *mut c_char;
    fn mic_capture_copy_default_output_name() -> *mut c_char;
    fn mic_capture_free_string(ptr: *mut c_char);
}

/// Convert a native C string pointer returned by native bridges to `Option<String>`,
/// safely freeing the native memory before returning.
/// Missing, null, or empty/whitespace strings return `None`.
unsafe fn free_and_convert_c_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let c_str = unsafe { CStr::from_ptr(ptr) };
    let res = c_str.to_str().ok().map(|s| s.to_string());
    unsafe { mic_capture_free_string(ptr) };
    res.filter(|s| !s.trim().is_empty())
}

// --- Native bridge (system_capture.m) — private Core Audio process tap -----
//
// `system_capture_start` runs ONLY on the `audio-sys` worker thread and is
// never awaited by `start_capture`: first-run TCC consent can block or
// restart the audio stack (see `CaptureHandle::drop` for the matching
// detach rule).

#[repr(C)]
struct SystemCaptureController {
    _opaque: [u8; 0],
}

extern "C" {
    fn system_capture_start(
        buffer_frames: u32,
        out_sample_rate: *mut f64,
        out_error: *mut i32,
    ) -> *mut SystemCaptureController;
    fn system_capture_ring_capacity(controller: *const SystemCaptureController) -> u32;
    fn system_capture_read(
        controller: *mut SystemCaptureController,
        dst: *mut f32,
        max_frames: u32,
    ) -> u32;
    fn system_capture_take_dropped(controller: *mut SystemCaptureController) -> u64;
    fn system_capture_take_route_change(controller: *mut SystemCaptureController) -> u32;
    fn system_capture_stop(controller: *mut SystemCaptureController);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MicrophonePermission {
    NotDetermined = 0,
    Restricted = 1,
    Denied = 2,
    Authorized = 3,
}

impl MicrophonePermission {
    fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Restricted,
            2 => Self::Denied,
            3 => Self::Authorized,
            _ => Self::NotDetermined,
        }
    }
}

extern "C" fn permission_callback<F>(raw: u32, context: *mut c_void)
where
    F: FnOnce(MicrophonePermission) + Send + 'static,
{
    // SAFETY: request_microphone_permission creates this Box and the native
    // bridge invokes its completion exactly once, including terminal states.
    let callback = unsafe { Box::from_raw(context.cast::<F>()) };
    let _ = catch_unwind(AssertUnwindSafe(|| {
        callback(MicrophonePermission::from_raw(raw));
    }));
}

/// Current TCC microphone state. This never opens a prompt.
pub fn microphone_permission() -> MicrophonePermission {
    // SAFETY: status has no pointers and only queries AVFoundation.
    MicrophonePermission::from_raw(unsafe { mic_capture_permission_status() })
}

/// Request microphone permission after an explicit user action.
///
/// AVFoundation owns the native completion until it fires. The boxed Rust
/// closure is then reclaimed exactly once by `permission_callback`.
///
/// A process without an attributable bundle identity and non-empty purpose
/// string cannot own a microphone TCC prompt, so the bridge answers
/// `Restricted` synchronously instead of forwarding the request (see
/// `mic_capture.m`).
pub fn request_microphone_permission<F>(callback: F)
where
    F: FnOnce(MicrophonePermission) + Send + 'static,
{
    let context = Box::into_raw(Box::new(callback)).cast::<c_void>();
    // SAFETY: context remains valid until the exactly-once native callback.
    unsafe { mic_capture_request_permission(permission_callback::<F>, context) };
}

struct MicStartupError {
    error: anyhow::Error,
    terminal: bool,
}

fn start_error_message(code: i32) -> anyhow::Error {
    match code {
        MIC_START_PERMISSION => anyhow!(
            "microphone capture unavailable: macOS microphone permission is not granted \
             (not determined, denied, or restricted) — enable it in System Settings > \
             Privacy & Security > Microphone"
        ),
        MIC_START_NO_INPUT => anyhow!("microphone capture unavailable: no input device"),
        _ => anyhow!("microphone capture unavailable: AVAudioEngine failed to start"),
    }
}

/// Map a `system_capture.m` start category to a safe, generic user-facing
/// message. No URLs, no raw audio, no private host data — category only.
fn system_start_error_message(code: i32) -> &'static str {
    match code {
        SYS_START_DEVICE => {
            "system audio capture unavailable: Core Audio could not start system audio \
             capture — check that Suflyor is allowed in System Settings > Privacy & \
             Security > Screen & System Audio Recording, then restart Suflyor"
        }
        SYS_START_TAP | SYS_START_AGGREGATE => {
            "system audio capture unavailable: macOS refused to create the system audio tap"
        }
        SYS_START_FORMAT => {
            "system audio capture unavailable: system audio tap format is not readable"
        }
        SYS_START_IO_PROC => {
            "system audio capture unavailable: system audio tap callback was refused"
        }
        SYS_START_NO_MEMORY => {
            "system audio capture unavailable: not enough memory for the system audio tap"
        }
        _ => "system audio capture unavailable: Core Audio failed to start",
    }
}

/// The only state that is NOT joinable: while `system_capture_start` is
/// still pending, joining would park the caller inside native TCC/Core Audio
/// code we do not control. Running/finished workers observe the stop flag
/// and exit promptly, so joining them is bounded.
fn system_worker_joinable(state: u8) -> bool {
    state != SYSTEM_WORKER_PENDING
}

// --- Public API (mirrors audio_unavailable.rs) ------------------------------

/// Enumerate default render + capture endpoint names on macOS.
pub fn list_devices() -> Result<DeviceList> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    let in_ptr = unsafe { mic_capture_copy_default_input_name() };
    if let Some(name) = unsafe { free_and_convert_c_string(in_ptr) } {
        inputs.push(name);
    }

    let out_ptr = unsafe { mic_capture_copy_default_output_name() };
    if let Some(name) = unsafe { free_and_convert_c_string(out_ptr) } {
        outputs.push(name);
    }

    Ok(DeviceList { outputs, inputs })
}

/// Handle returned to caller — dropping it sets the shared stop flag and
/// joins every worker that was started (microphone and/or system; the
/// system worker only when joinable); each joined worker synchronously
/// stops its engine/tap and destroys the native controller before
/// returning. Idempotent.
///
/// Narrow unavoidable macOS behavior: if the handle is dropped while the
/// system worker is still INSIDE `system_capture_start` (first-run TCC
/// consent can block `AudioDeviceStart` indefinitely), the worker is
/// DETACHED instead of joined — the caller must not park on native code we
/// do not control. If `system_capture_start` returns, the detached worker
/// observes the stop flag immediately, tears the tap down synchronously and
/// exits on its own. If Apple's consent flow never returns, the detached
/// thread and any partial native state live until OS process cleanup — so
/// in that pending case the handle drop does NOT guarantee all native
/// resources are gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCaptureState {
    Pending,
    Running,
    Failed,
}

pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    mic_worker: Option<thread::JoinHandle<()>>,
    system_worker: Option<(Arc<AtomicU8>, thread::JoinHandle<()>)>,
    metrics: Arc<AudioMetrics>,
}

impl CaptureHandle {
    /// Read-only copy of the per-stream health counters; safe to call from
    /// any thread while capture runs.
    pub fn metrics_snapshot(&self) -> AudioMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Verified asynchronous system-capture worker state.
    #[must_use]
    pub fn system_capture_state(&self) -> SystemCaptureState {
        match self
            .system_worker
            .as_ref()
            .map(|(state, _)| state.load(Ordering::Acquire))
        {
            Some(SYSTEM_WORKER_PENDING) => SystemCaptureState::Pending,
            Some(SYSTEM_WORKER_RUNNING) => SystemCaptureState::Running,
            Some(SYSTEM_WORKER_FINISHED) | None | Some(_) => SystemCaptureState::Failed,
        }
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.mic_worker.take() {
            let _ = worker.join();
        }
        if let Some((state, worker)) = self.system_worker.take() {
            if system_worker_joinable(state.load(Ordering::Acquire)) {
                let _ = worker.join();
            } else {
                log::warn!(
                    "[Sys] system audio start still pending at stop — worker detached; \
                     it tears the tap down itself as soon as start returns"
                );
            }
        }
    }
}

/// Start capture on both sources, each independent of the other.
///
/// The microphone keeps its synchronous start attempt on the default input
/// device. Permission failure is terminal for that worker; missing or changing
/// hardware keeps the worker armed until a default input becomes available.
/// The system worker starts asynchronously and is never awaited:
/// `system_capture_start` executes only on the `audio-sys` thread, because
/// first-run macOS TCC consent can block or restart the audio stack. The
/// asynchronous native start can still fail later; that failure is logged
/// and may leave the session with no active audio source. `start_capture`
/// returns an immediate error only when the mic native start failed AND
/// the system worker thread could not be spawned; it never synchronously
/// knows whether the asynchronous system start succeeds.
///
/// Device selection by name cannot be honoured on macOS yet: a persisted
/// `mic_device` / `sys_device` (usually a Windows friendly name) is logged
/// once and ignored instead of faking a selection. The system tap mirrors
/// all system output.
pub fn start_capture(
    mic_device: Option<String>,
    sys_device: Option<String>,
) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)> {
    if mic_device.is_some_and(|d| !d.trim().is_empty()) {
        log::warn!(
            "audio_macos: persisted mic device selection is not supported on macOS; \
             falling back to the default input"
        );
    }
    if sys_device.is_some_and(|d| !d.trim().is_empty()) {
        log::warn!(
            "audio_macos: persisted system device selection is not supported on macOS; \
             the process tap mirrors all system output"
        );
    }

    let (tx, rx) = mpsc::channel::<AudioChunk>(128);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel::<Result<u32, MicStartupError>>();
    // One shared timestamp origin for the whole session: Mic and System
    // chunk timestamps stay aligned even when the system TCC flow returns
    // much later than the microphone start.
    let session_start = Instant::now();
    let metrics = Arc::new(AudioMetrics::default());

    let mic_tx = tx.clone();
    let mic_metrics = metrics.clone();
    let worker = thread::Builder::new()
        .name("audio-mic".into())
        .spawn(move || {
            capture_worker(mic_tx, worker_stop, started_tx, session_start, mic_metrics);
        })?;

    // Synchronous startup report: the native start attempt still runs on the
    // mic worker, but a failure no longer fails the whole capture. Permission
    // failure is terminal; a missing or unstable default input keeps this
    // worker armed while system capture continues independently.
    let mic_worker = match started_rx.recv() {
        Ok(Ok(rate)) => {
            log::info!("[Mic] audio_route open mode=default device=default_input rate={rate} Hz");
            Some(worker)
        }
        Ok(Err(startup_err)) => {
            if startup_err.terminal {
                log::warn!(
                    "[Mic] {} — continuing with system audio only",
                    startup_err.error
                );
                let _ = worker.join();
                None
            } else {
                log::warn!("[Mic] {} — keeping mic worker armed", startup_err.error);
                Some(worker)
            }
        }
        Err(_) => {
            // The channel only disconnects after the worker has exited, so
            // this join is bounded; clear the thread before we continue.
            let _ = worker.join();
            // Fixed generic warning: no raw channel/OS details in logs.
            log::warn!(
                "[Mic] microphone worker exited before reporting startup — \
                 continuing with system audio only"
            );
            None
        }
    };

    // System worker: start is NEVER awaited here (may block in first-run
    // TCC). Spawn failure degrades by what is already running.
    let sys_state = Arc::new(AtomicU8::new(SYSTEM_WORKER_PENDING));
    let sys_stop = stop.clone();
    let sys_state_worker = sys_state.clone();
    let sys_metrics = metrics.clone();
    let system_worker = thread::Builder::new()
        .name("audio-sys".into())
        .spawn(move || {
            system_capture_worker(tx, sys_stop, sys_state_worker, session_start, sys_metrics);
        });
    let system_worker = match system_worker {
        Ok(handle) => handle,
        Err(_) => {
            // Fixed generic messages: the raw OS error can carry host
            // specifics we do not want surfaced in tiles/logs.
            if mic_worker.is_some() {
                // Mic is running: degrade to microphone-only capture.
                log::warn!(
                    "[Sys] system audio worker failed to start — \
                     continuing with microphone-only capture"
                );
                return Ok((
                    rx,
                    CaptureHandle {
                        stop,
                        mic_worker,
                        system_worker: None,
                        metrics,
                    },
                ));
            }
            // Neither source started: the mic worker was already stopped
            // and joined above, so there is nothing left to tear down.
            return Err(anyhow!(
                "audio capture unavailable: no capture worker could start"
            ));
        }
    };

    Ok((
        rx,
        CaptureHandle {
            stop,
            mic_worker,
            system_worker: Some((sys_state, system_worker)),
            metrics,
        },
    ))
}

/// Push-to-talk capture until `stop` flips.
pub fn record_source_until_stop(
    source: AudioSource,
    _mic_device: Option<String>,
    _sys_device: Option<String>,
    stop: Arc<AtomicBool>,
) -> Result<Vec<i16>> {
    let mut native_rate: f64 = 0.0;
    let mut error_code: i32 = 0;
    let (mic_ctrl, sys_ctrl) = match source {
        AudioSource::Mic => {
            let ctrl = unsafe {
                mic_capture_start(NATIVE_BUFFER_FRAMES, &mut native_rate, &mut error_code)
            };
            if ctrl.is_null() {
                return Err(start_error_message(error_code));
            }
            (Some(ctrl), None)
        }
        AudioSource::System => {
            let ctrl = unsafe {
                system_capture_start(NATIVE_BUFFER_FRAMES, &mut native_rate, &mut error_code)
            };
            if ctrl.is_null() {
                return Err(anyhow!("{}", system_start_error_message(error_code)));
            }
            (None, Some(ctrl))
        }
    };

    let rate = if native_rate > 0.0 {
        native_rate
    } else {
        48000.0
    };
    let ring_capacity = if let Some(ctrl) = mic_ctrl {
        (unsafe { mic_capture_ring_capacity(ctrl) }) as usize
    } else if let Some(ctrl) = sys_ctrl {
        (unsafe { system_capture_ring_capacity(ctrl) }) as usize
    } else {
        4096
    };

    let mut scratch = vec![0.0_f32; ring_capacity.max(NATIVE_BUFFER_FRAMES as usize)];
    let mut accum: Vec<f32> = Vec::new();

    while !stop.load(Ordering::Acquire) {
        thread::sleep(DRAIN_INTERVAL);
        let n = if let Some(ctrl) = mic_ctrl {
            (unsafe { mic_capture_read(ctrl, scratch.as_mut_ptr(), scratch.len() as u32) }) as usize
        } else if let Some(ctrl) = sys_ctrl {
            (unsafe { system_capture_read(ctrl, scratch.as_mut_ptr(), scratch.len() as u32) })
                as usize
        } else {
            0
        };
        if n > 0 {
            accum.extend_from_slice(&scratch[..n]);
        }
    }

    if let Some(ctrl) = mic_ctrl {
        unsafe { mic_capture_stop(ctrl) };
    }
    if let Some(ctrl) = sys_ctrl {
        unsafe { system_capture_stop(ctrl) };
    }

    if accum.is_empty() {
        return Ok(Vec::new());
    }

    let ratio = rate / f64::from(TARGET_SAMPLE_RATE);
    Ok(resample_and_quantise(&accum, ratio))
}

/// Record the system (loopback) audio for a fixed duration.
pub fn record_sys_blocking(duration_ms: u64, _sys_device: Option<String>) -> Result<Vec<i16>> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(duration_ms));
        stop_clone.store(true, Ordering::Release);
    });
    record_source_until_stop(AudioSource::System, None, None, stop)
}

/// RMS energy of 16-bit PCM samples in dBFS (0 = full-scale, −∞ = silence).
/// Pure math — identical to the Windows implementation.
#[must_use]
pub fn rms_dbfs(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return f64::NEG_INFINITY;
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|s| {
            let v = f64::from(*s) / 32768.0;
            v * v
        })
        .sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    if rms <= 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * rms.log10()
    }
}

/// Diagnostics system-audio self-test.
pub fn play_tone_and_capture(_sys_device: Option<String>) -> Result<Vec<i16>> {
    record_sys_blocking(3000, None)
}

/// Record the microphone for a fixed duration.
pub fn record_mic_blocking(duration_ms: u64, _mic_device: Option<String>) -> Result<Vec<i16>> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(duration_ms));
        stop_clone.store(true, Ordering::Release);
    });
    record_source_until_stop(AudioSource::Mic, None, None, stop)
}

// --- Worker -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryResult<T> {
    Success(T),
    Retry,
    Stop,
}

fn retry_route_start<T>(
    stop: &AtomicBool,
    delay: Duration,
    mut start: impl FnMut() -> RetryResult<T>,
) -> Option<T> {
    loop {
        if stop.load(Ordering::Acquire) {
            return None;
        }
        match start() {
            RetryResult::Success(value) => return Some(value),
            RetryResult::Stop => return None,
            RetryResult::Retry => {}
        }
        if stop.load(Ordering::Acquire) {
            return None;
        }
        thread::sleep(delay);
    }
}

fn reopen_mic(stop: &AtomicBool) -> Option<(*mut MicController, f64)> {
    retry_route_start(stop, ROUTE_RESTART_DELAY, || {
        if matches!(
            microphone_permission(),
            MicrophonePermission::Denied | MicrophonePermission::Restricted
        ) {
            return RetryResult::Stop;
        }
        let mut native_rate = 0.0;
        let mut error_code = 0;
        // SAFETY: the returned controller is owned by this worker and stopped
        // before replacement or worker exit.
        let controller =
            unsafe { mic_capture_start(NATIVE_BUFFER_FRAMES, &mut native_rate, &mut error_code) };
        if !controller.is_null() {
            RetryResult::Success((controller, native_rate))
        } else if error_code == MIC_START_PERMISSION {
            RetryResult::Stop
        } else {
            RetryResult::Retry
        }
    })
}

fn reopen_system(stop: &AtomicBool) -> Option<(*mut SystemCaptureController, f64)> {
    retry_route_start(stop, ROUTE_RESTART_DELAY, || {
        let mut native_rate = 0.0;
        let mut error_code = 0;
        // SAFETY: the returned controller is owned by this worker and stopped
        // before replacement or worker exit.
        let controller = unsafe {
            system_capture_start(NATIVE_BUFFER_FRAMES, &mut native_rate, &mut error_code)
        };
        if !controller.is_null() {
            RetryResult::Success((controller, native_rate))
        } else {
            RetryResult::Retry
        }
    })
}

fn capture_worker(
    tx: mpsc::Sender<AudioChunk>,
    stop: Arc<AtomicBool>,
    started: std::sync::mpsc::Sender<Result<u32, MicStartupError>>,
    session_start: Instant,
    metrics: Arc<AudioMetrics>,
) {
    let mut native_rate: f64 = 0.0;
    let mut error_code: i32 = 0;
    let mut controller =
        unsafe { mic_capture_start(NATIVE_BUFFER_FRAMES, &mut native_rate, &mut error_code) };
    if controller.is_null() {
        let is_terminal = error_code == MIC_START_PERMISSION;
        let err = start_error_message(error_code);
        if started
            .send(Err(MicStartupError {
                error: err,
                terminal: is_terminal,
            }))
            .is_err()
            || is_terminal
        {
            return;
        }
        let Some((next_controller, next_rate)) = reopen_mic(stop.as_ref()) else {
            log::info!("[Mic] macOS capture worker exit");
            return;
        };
        controller = next_controller;
        native_rate = next_rate;
        log::info!("[Mic] macOS default input became available");
    } else if started.send(Ok(native_rate as u32)).is_err() {
        unsafe { mic_capture_stop(controller) };
        return;
    }

    let mut rate = native_rate as usize;
    let ring_capacity = unsafe { mic_capture_ring_capacity(controller) } as usize;
    let mut scratch = vec![0.0_f32; ring_capacity.max(NATIVE_BUFFER_FRAMES as usize)];
    let mut accum: Vec<f32> = Vec::with_capacity(rate / 5 + NATIVE_BUFFER_FRAMES as usize);
    let mut chunk_target = (rate / 5).max(1); // ~200 ms at the native rate
    let mut ratio = native_rate / f64::from(TARGET_SAMPLE_RATE);
    let mut dropped_chunks: u64 = 0;

    while !stop.load(Ordering::Acquire) {
        thread::sleep(DRAIN_INTERVAL);

        if unsafe { mic_capture_take_route_change(controller) } != 0 {
            log::info!("[Mic] macOS audio route changed — reopening current input");
            unsafe { mic_capture_stop(controller) };
            let Some((next_controller, next_rate)) = reopen_mic(stop.as_ref()) else {
                if !stop.load(Ordering::Acquire) {
                    log::warn!("[Mic] current input did not recover after route change");
                }
                log::info!("[Mic] macOS capture worker exit");
                return;
            };
            controller = next_controller;
            native_rate = next_rate;
            rate = native_rate as usize;
            let ring_capacity = unsafe { mic_capture_ring_capacity(controller) } as usize;
            scratch.resize(ring_capacity.max(NATIVE_BUFFER_FRAMES as usize), 0.0);
            accum.clear();
            chunk_target = (rate / 5).max(1);
            ratio = native_rate / f64::from(TARGET_SAMPLE_RATE);
            log::info!("[Mic] macOS audio route recovery complete rate={rate} Hz");
            continue;
        }

        let n = unsafe { mic_capture_read(controller, scratch.as_mut_ptr(), scratch.len() as u32) }
            as usize;
        let ring_dropped = unsafe { mic_capture_take_dropped(controller) };
        if ring_dropped > 0 {
            metrics.record_ring_overflow(Stream::Mic, ring_dropped);
            // Logged here — outside the realtime tap — and without payloads.
            log::warn!("[Mic] macOS native ring overflow — dropped {ring_dropped} frames");
        }
        if n > 0 {
            accum.extend_from_slice(&scratch[..n]);
            metrics.observe_pending_samples(Stream::Mic, accum.len() as u64);
        }

        // Emit when we've buffered ~200 ms of native audio.
        if accum.len() >= chunk_target {
            let pcm_i16 = resample_and_quantise(&accum, ratio);
            accum.clear();
            if pcm_i16.is_empty() {
                continue;
            }
            let timestamp_ms = session_start.elapsed().as_millis() as u64;
            let chunk = AudioChunk {
                source: AudioSource::Mic,
                pcm_i16,
                timestamp_ms,
            };
            // Non-blocking send, mirroring the Windows policy: on a full
            // queue DROP the chunk and count it instead of parking the
            // capture worker (which would back-pressure the native ring).
            match tx.try_send(chunk) {
                Ok(()) => {
                    metrics.record_emitted_chunk(Stream::Mic, timestamp_ms);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    dropped_chunks += 1;
                    metrics.record_queue_drop(Stream::Mic);
                    // ~ every 5 s of dropped audio, so the stall is observable.
                    if dropped_chunks % 25 == 1 {
                        log::warn!(
                            "[Mic] STT feed stalled — dropped ~{}s of audio ({dropped_chunks} chunks)",
                            dropped_chunks / 5
                        );
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    log::info!("[Mic] receiver dropped, exiting");
                    break;
                }
            }
        }
    }

    // Synchronous teardown: removes the tap, stops the engine and destroys
    // the controller + ring. CaptureHandle::drop joins this thread, so the
    // native resources are gone before the handle finishes dropping.
    unsafe { mic_capture_stop(controller) };
    log::info!("[Mic] macOS capture worker exit");
}

/// System-audio worker — private process tap. Deliberately a small copy of
/// `capture_worker` (two short drain loops beat a shared-callback
/// abstraction). `system_capture_start` runs ONLY here; first-run TCC can
/// block inside it, which is why `CaptureHandle` never joins while `state`
/// is still PENDING.
fn system_capture_worker(
    tx: mpsc::Sender<AudioChunk>,
    stop: Arc<AtomicBool>,
    state: Arc<AtomicU8>,
    session_start: Instant,
    metrics: Arc<AudioMetrics>,
) {
    let mut native_rate: f64 = 0.0;
    let mut error_code: i32 = 0;
    let mut controller =
        unsafe { system_capture_start(NATIVE_BUFFER_FRAMES, &mut native_rate, &mut error_code) };
    if controller.is_null() {
        // A default-route transition can make the first tap attempt fail.
        // Keep the sole system worker armed until Core Audio settles or the
        // session stops; never fail the whole capture from this thread.
        log::warn!("[Sys] {}", system_start_error_message(error_code));
        let Some((next_controller, next_rate)) = reopen_system(stop.as_ref()) else {
            state.store(SYSTEM_WORKER_FINISHED, Ordering::Release);
            return;
        };
        controller = next_controller;
        native_rate = next_rate;
        log::info!("[Sys] macOS default output became available");
    }

    if stop.load(Ordering::Acquire) {
        // Stopped while start was pending: tear the tap down immediately,
        // before allocating any drain buffers.
        unsafe { system_capture_stop(controller) };
        state.store(SYSTEM_WORKER_FINISHED, Ordering::Release);
        return;
    }

    // Native startup is verified; from this point the worker is joinable and
    // must observe `stop` promptly.
    state.store(SYSTEM_WORKER_RUNNING, Ordering::Release);

    let mut rate = native_rate as usize;
    let ring_capacity = unsafe { system_capture_ring_capacity(controller) } as usize;
    let mut scratch = vec![0.0_f32; ring_capacity.max(NATIVE_BUFFER_FRAMES as usize)];
    let mut accum: Vec<f32> = Vec::with_capacity(rate / 5 + NATIVE_BUFFER_FRAMES as usize);
    let mut chunk_target = (rate / 5).max(1); // ~200 ms at the native rate
    let mut ratio = native_rate / f64::from(TARGET_SAMPLE_RATE);
    let mut dropped_chunks: u64 = 0;

    while !stop.load(Ordering::Acquire) {
        thread::sleep(DRAIN_INTERVAL);

        if unsafe { system_capture_take_route_change(controller) } != 0 {
            log::info!("[Sys] macOS audio route changed — rebuilding system tap");
            state.store(SYSTEM_WORKER_PENDING, Ordering::Release);
            unsafe { system_capture_stop(controller) };
            let Some((next_controller, next_rate)) = reopen_system(stop.as_ref()) else {
                if !stop.load(Ordering::Acquire) {
                    log::warn!("[Sys] system tap did not recover after route change");
                }
                state.store(SYSTEM_WORKER_FINISHED, Ordering::Release);
                log::info!("[Sys] macOS capture worker exit");
                return;
            };
            if stop.load(Ordering::Acquire) {
                unsafe { system_capture_stop(next_controller) };
                state.store(SYSTEM_WORKER_FINISHED, Ordering::Release);
                return;
            }
            controller = next_controller;
            native_rate = next_rate;
            rate = native_rate as usize;
            let ring_capacity = unsafe { system_capture_ring_capacity(controller) } as usize;
            scratch.resize(ring_capacity.max(NATIVE_BUFFER_FRAMES as usize), 0.0);
            accum.clear();
            chunk_target = (rate / 5).max(1);
            ratio = native_rate / f64::from(TARGET_SAMPLE_RATE);
            state.store(SYSTEM_WORKER_RUNNING, Ordering::Release);
            log::info!("[Sys] macOS audio route recovery complete rate={rate} Hz");
            continue;
        }

        let n =
            unsafe { system_capture_read(controller, scratch.as_mut_ptr(), scratch.len() as u32) }
                as usize;
        let ring_dropped = unsafe { system_capture_take_dropped(controller) };
        if ring_dropped > 0 {
            metrics.record_ring_overflow(Stream::System, ring_dropped);
            // Logged here — outside the realtime tap — and without payloads.
            log::warn!("[Sys] macOS native ring overflow — dropped {ring_dropped} frames");
        }
        if n > 0 {
            accum.extend_from_slice(&scratch[..n]);
            metrics.observe_pending_samples(Stream::System, accum.len() as u64);
        }

        // Emit when we've buffered ~200 ms of native audio.
        if accum.len() >= chunk_target {
            let pcm_i16 = resample_and_quantise(&accum, ratio);
            accum.clear();
            if pcm_i16.is_empty() {
                continue;
            }
            let timestamp_ms = session_start.elapsed().as_millis() as u64;
            let chunk = AudioChunk {
                source: AudioSource::System,
                pcm_i16,
                timestamp_ms,
            };
            // Non-blocking send, mirroring the Windows policy: on a full
            // queue DROP the chunk and count it instead of parking the
            // capture worker (which would back-pressure the native ring).
            match tx.try_send(chunk) {
                Ok(()) => {
                    metrics.record_emitted_chunk(Stream::System, timestamp_ms);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    dropped_chunks += 1;
                    metrics.record_queue_drop(Stream::System);
                    // ~ every 5 s of dropped audio, so the stall is observable.
                    if dropped_chunks % 25 == 1 {
                        log::warn!(
                            "[Sys] STT feed stalled — dropped ~{}s of audio ({dropped_chunks} chunks)",
                            dropped_chunks / 5
                        );
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    log::info!("[Sys] receiver dropped, exiting");
                    break;
                }
            }
        }
    }

    // Synchronous teardown: stops the device, destroys the IOProc, aggregate
    // device, process tap and frees the controller + ring.
    unsafe { system_capture_stop(controller) };
    state.store(SYSTEM_WORKER_FINISHED, Ordering::Release);
    log::info!("[Sys] macOS capture worker exit");
}

/// Average-decimate `input` (f32, native rate, mono) → i16 at 16 kHz.
/// Duplicated from the Windows implementation — small and self-contained.
fn resample_and_quantise(input: &[f32], ratio: f64) -> Vec<i16> {
    if input.is_empty() || ratio <= 0.0 {
        return Vec::new();
    }
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let start = (i as f64 * ratio).floor() as usize;
        let end = (((i + 1) as f64 * ratio).floor() as usize).min(input.len());
        if start >= end {
            continue;
        }
        let mean: f32 = input[start..end].iter().copied().sum::<f32>() / (end - start) as f32;
        let clamped = mean.clamp(-1.0, 1.0);
        out.push((clamped * i16::MAX as f32) as i16);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn list_devices_returns_default_device_names_and_handles_missing_safely() {
        let null_res = unsafe { free_and_convert_c_string(std::ptr::null_mut()) };
        assert!(null_res.is_none());
        assert!(list_devices().is_ok());
    }

    #[test]
    fn record_source_until_stop_supported_shape() {
        let stop = Arc::new(AtomicBool::new(true));
        let _ = record_source_until_stop(AudioSource::Mic, None, None, stop);
    }

    #[test]
    fn start_error_categories_are_distinct_and_descriptive() {
        let permission = start_error_message(MIC_START_PERMISSION).to_string();
        let no_input = start_error_message(MIC_START_NO_INPUT).to_string();
        let engine = start_error_message(99).to_string();
        assert!(permission.contains("permission is not granted"));
        assert!(no_input.contains("no input device"));
        assert!(engine.contains("AVAudioEngine"));
        assert_ne!(permission, no_input);
        assert_ne!(permission, engine);
    }

    #[test]
    fn native_bridge_symbol_links_without_opening_microphone() {
        let symbols = [
            mic_capture_start as *const (),
            mic_capture_permission_status as *const (),
            mic_capture_request_permission as *const (),
            mic_capture_take_route_change as *const (),
            mic_capture_copy_default_input_name as *const (),
            mic_capture_copy_default_output_name as *const (),
            mic_capture_free_string as *const (),
        ];
        let _ = std::hint::black_box(symbols);
    }

    #[test]
    fn system_capture_bridge_symbols_link_without_starting_capture() {
        // Link-only: taking every function pointer proves the system seam is
        // compiled in. Calling `system_capture_start` is deliberately NOT
        // done here — it reaches the Core Audio TCC path and is only ever
        // invoked on the `audio-sys` worker thread.
        let symbols = [
            system_capture_start as *const (),
            system_capture_ring_capacity as *const (),
            system_capture_read as *const (),
            system_capture_take_dropped as *const (),
            system_capture_take_route_change as *const (),
            system_capture_stop as *const (),
        ];
        let _ = std::hint::black_box(symbols);
    }

    #[test]
    fn route_restart_retry_is_stop_aware_and_recovers_on_success() {
        let stop = AtomicBool::new(false);
        let attempts = std::cell::Cell::new(0);
        let recovered = retry_route_start(&stop, Duration::ZERO, || {
            let next = attempts.get() + 1;
            attempts.set(next);
            if next == 3 {
                RetryResult::Success(next)
            } else {
                RetryResult::Retry
            }
        });
        assert_eq!(recovered, Some(3));
        assert_eq!(attempts.get(), 3);

        stop.store(true, Ordering::Release);
        attempts.set(0);
        let stopped = retry_route_start(&stop, Duration::ZERO, || {
            attempts.set(attempts.get() + 1);
            RetryResult::Retry
        });
        assert!(stopped.is_none());
        assert_eq!(attempts.get(), 0);

        let stop2 = AtomicBool::new(false);
        attempts.set(0);
        let terminal = retry_route_start(&stop2, Duration::ZERO, || {
            attempts.set(attempts.get() + 1);
            RetryResult::Stop
        });
        assert!(terminal.is_none());
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn system_start_error_categories_map_to_safe_generic_messages() {
        // The device-start/TCC category must point at the exact System
        // Settings pane and tell the user to restart Suflyor — without
        // asserting that permission was definitely denied.
        let device = system_start_error_message(SYS_START_DEVICE);
        assert!(
            device.contains("Screen & System Audio Recording"),
            "TCC message must name the pane, got: {device}"
        );
        assert!(
            device.contains("restart Suflyor"),
            "TCC message must ask for a restart, got: {device}"
        );
        assert!(
            device.contains("Core Audio could not start"),
            "TCC message must describe a failed start, got: {device}"
        );
        assert!(
            !device.contains("not granted"),
            "TCC message must not claim a definite denial, got: {device}"
        );

        let all = [
            device,
            system_start_error_message(SYS_START_TAP),
            system_start_error_message(SYS_START_AGGREGATE),
            system_start_error_message(SYS_START_FORMAT),
            system_start_error_message(SYS_START_IO_PROC),
            system_start_error_message(SYS_START_NO_MEMORY),
            system_start_error_message(99), // unknown -> generic
        ];
        for msg in all {
            assert!(
                msg.starts_with("system audio capture unavailable"),
                "every message must be generic + safe, got: {msg}"
            );
            assert!(
                !msg.contains("http"),
                "no URL may leak into a user-facing error, got: {msg}"
            );
        }
    }

    #[test]
    fn capture_handle_metrics_snapshot_is_zeroed_before_any_capture() {
        // Pure construction: no native start, no worker threads. Dropping
        // this handle only sets the stop flag (no workers to join).
        let handle = CaptureHandle {
            stop: Arc::new(AtomicBool::new(false)),
            mic_worker: None,
            system_worker: None,
            metrics: Arc::new(AudioMetrics::default()),
        };
        assert_eq!(handle.metrics_snapshot(), AudioMetricsSnapshot::default());
    }

    #[test]
    fn system_worker_join_decision_never_joins_pending_start() {
        // Pending == still inside system_capture_start (may block in TCC) —
        // must be detached, never joined. Running/finished join is bounded.
        assert!(!system_worker_joinable(SYSTEM_WORKER_PENDING));
        assert!(system_worker_joinable(SYSTEM_WORKER_RUNNING));
        assert!(system_worker_joinable(SYSTEM_WORKER_FINISHED));
    }

    #[test]
    fn permission_request_from_an_unbundled_process_answers_restricted() {
        // The test runner is a bare binary: no bundle identifier, no
        // Info.plist usage description. Such a process cannot own a
        // microphone TCC prompt, so the bridge must answer restricted
        // synchronously instead of forwarding the request.
        let captured = Arc::new(std::sync::Mutex::new(None));
        let slot = captured.clone();
        request_microphone_permission(move |permission| {
            *slot.lock().unwrap() = Some(permission);
        });
        assert_eq!(
            *captured.lock().unwrap(),
            Some(MicrophonePermission::Restricted),
            "unbundled requests must be refused synchronously as restricted"
        );
    }

    #[test]
    fn permission_status_mapping_is_closed_and_stable() {
        assert_eq!(
            MicrophonePermission::from_raw(0),
            MicrophonePermission::NotDetermined
        );
        assert_eq!(
            MicrophonePermission::from_raw(1),
            MicrophonePermission::Restricted
        );
        assert_eq!(
            MicrophonePermission::from_raw(2),
            MicrophonePermission::Denied
        );
        assert_eq!(
            MicrophonePermission::from_raw(3),
            MicrophonePermission::Authorized
        );
        assert_eq!(
            MicrophonePermission::from_raw(u32::MAX),
            MicrophonePermission::NotDetermined
        );
    }

    #[test]
    fn rms_dbfs_matches_windows_helper() {
        assert_eq!(rms_dbfs(&[]), f64::NEG_INFINITY);
        assert_eq!(rms_dbfs(&[0, 0, 0, 0]), f64::NEG_INFINITY);
        let full = [i16::MAX, i16::MIN, i16::MAX, i16::MIN];
        let d = rms_dbfs(&full);
        assert!(d > -0.5 && d <= 0.0, "full-scale ~0 dBFS, got {d}");
    }

    #[test]
    fn decimator_48k_to_16k_is_3_to_1() {
        let input: Vec<f32> = (0..48).map(|i| (i as f32) / 100.0).collect();
        let out = resample_and_quantise(&input, 3.0);
        assert_eq!(out.len(), 16);
    }

    #[test]
    fn decimator_handles_empty_and_bad_ratio() {
        assert!(resample_and_quantise(&[], 3.0).is_empty());
        assert!(resample_and_quantise(&[0.5; 16], 0.0).is_empty());
        assert!(resample_and_quantise(&[0.5; 16], -1.0).is_empty());
    }

    #[test]
    fn decimator_oversaturation_clamped() {
        // f32 input > 1.0 must be clamped, not wrap around to negative.
        let input = vec![2.0_f32, -2.0, 5.5];
        let out = resample_and_quantise(&input, 1.0);
        assert_eq!(out[0], i16::MAX);
        assert_eq!(out[1], i16::MIN + 1, "−1.0 clamp * i16::MAX = i16::MIN+1");
        assert_eq!(out[2], i16::MAX);
    }

    #[test]
    fn decimator_preserves_average_amplitude() {
        // Constant DC signal at 0.5 stays near 0.5 after averaging.
        let input = vec![0.5f32; 4800]; // 100 ms at 48k
        let out = resample_and_quantise(&input, 3.0);
        let target_i16 = (0.5 * i16::MAX as f32) as i16;
        for &s in &out {
            assert!(
                (i32::from(s) - i32::from(target_i16)).abs() <= 1,
                "DC must survive decimation: got {s} expected {target_i16}"
            );
        }
    }

    /// INTEGRATION: a 1 kHz sine at 48 kHz, decimated 3:1, keeps its peak
    /// autocorrelation at ~16 samples (= 1 kHz at 16 kHz) — frequency
    /// content survives, not just length.
    #[test]
    fn decimator_preserves_1khz_sine_frequency() {
        let sample_rate = 48_000.0;
        let target_rate = 16_000.0;
        let freq = 1000.0;
        let n_samples = 9600; // 200 ms
        let input: Vec<f32> = (0..n_samples)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
            .collect();

        let out = resample_and_quantise(&input, (sample_rate / target_rate) as f64);
        assert_eq!(out.len(), 3200, "200 ms at 16 kHz = 3200 samples");

        let signal: Vec<f32> = out.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        let mut best_lag = 0usize;
        let mut best_corr = f32::MIN;
        for lag in 8..=32 {
            let mut corr = 0.0;
            for i in 0..(signal.len() - lag) {
                corr += signal[i] * signal[i + lag];
            }
            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }
        assert!(
            (16i32 - best_lag as i32).abs() <= 1,
            "expected period ≈16 samples (1 kHz at 16 kHz), got lag={best_lag}"
        );
    }
}
