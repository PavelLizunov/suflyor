//! Linux x86_64 NeMo-Speech.cpp V3 diarization runner.
//! Spawns the native CLI out-of-process without linking runtime code into the host.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::persistence::DiarSegment;

pub const MODEL_ID: &str = "nemotron-3-diarization/q8_0@f667ed73aee57d40cc39428eb768b4fd87a0a29e";
pub const MODEL_SIZE: u64 = 107_012_128;
pub const MODEL_SHA256: &str = "08456d9e22cd9a323c0364d98375f3746d6e68507ebb705cd46438c534c7a3a1";
pub const MODEL_URL: &str = "https://huggingface.co/nvidia/Nemotron-3-Diarization/resolve/f667ed73aee57d40cc39428eb768b4fd87a0a29e/Nemotron-3-Diarization.q8_0.gguf";
pub const MAX_RTTM_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_RUN_TIME: Duration = Duration::from_secs(3 * 60 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const MODEL_LABEL: &str = "Nemotron-3-Diarization";

static INSTALL_BUSY: AtomicBool = AtomicBool::new(false);

struct InstallGuard;

impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALL_BUSY.store(false, Ordering::Release);
    }
}

fn try_acquire_install() -> Result<InstallGuard> {
    if INSTALL_BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        bail!("установка моделей уже выполняется");
    }
    Ok(InstallGuard)
}

/// True on Linux x86_64 target architecture.
#[must_use]
pub fn supported() -> bool {
    cfg!(all(target_os = "linux", target_arch = "x86_64"))
}

/// Expected path to the downloaded Nemotron GGUF model.
#[must_use]
pub fn model_path() -> Option<PathBuf> {
    crate::paths::speech_model_dir("diar")
        .map(|d| d.join("nemotron-v3").join("Nemotron-3-Diarization.q8_0.gguf"))
}

/// Path to the staged official nemo-speech executable sibling to current_exe.
#[must_use]
pub fn runtime_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    Some(parent.join("nemotron").join("bin").join("nemo-speech"))
}

/// True only when the exact model file exists with size 107012128 and runtime executable exists.
#[must_use]
pub fn installed() -> bool {
    let Some(path) = model_path() else {
        return false;
    };
    let Ok(meta) = std::fs::metadata(&path) else {
        return false;
    };
    if !meta.is_file() || meta.len() != MODEL_SIZE {
        return false;
    }
    let Some(runtime) = runtime_path() else {
        return false;
    };
    runtime.is_file()
}

/// Verify exact file size and SHA-256 digest without removing the file.
fn verify_model_file(path: &Path) -> Result<()> {
    let meta = std::fs::metadata(path).context("read model metadata")?;
    if meta.len() != MODEL_SIZE {
        bail!(
            "Nemotron model size mismatch: expected {MODEL_SIZE}, got {}",
            meta.len()
        );
    }
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).context("read model to verify digest")?;
    let got = crate::download::hex(&Sha256::digest(&bytes));
    if !got.eq_ignore_ascii_case(MODEL_SHA256) {
        bail!("Nemotron model digest mismatch; reinstall the model");
    }
    Ok(())
}

/// Install the Nemotron diarization model.
/// Verifies size and SHA-256 before committing, reuses verified existing model,
/// and requires the staged runtime to be present.
pub fn install(cancel: &AtomicBool, on: &dyn Fn(crate::diar_install::DiarProgress)) -> Result<()> {
    let _guard = try_acquire_install()?;

    let Some(runtime) = runtime_path() else {
        bail!("Nemotron runtime unavailable");
    };
    if !runtime.is_file() {
        bail!("Nemotron runtime unavailable");
    }

    let dest = model_path().context("Nemotron model path unavailable")?;
    let dest_dir = dest.parent().context("model directory unavailable")?;
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("create {}", dest_dir.display()))?;

    if dest.is_file() && verify_model_file(&dest).is_ok() {
        on(crate::diar_install::DiarProgress::AlreadyInstalled(MODEL_LABEL.to_string()));
        on(crate::diar_install::DiarProgress::AllInstalled);
        return Ok(());
    }

    if cancel.load(Ordering::Acquire) {
        bail!("отменено");
    }

    let tmp = dest_dir.join("Nemotron-3-Diarization.q8_0.gguf.download");
    let _ = std::fs::remove_file(&tmp);

    on(crate::diar_install::DiarProgress::Downloading(MODEL_LABEL.to_string()));
    if let Err(err) = crate::download::curl_download(MODEL_URL, &tmp) {
        on(crate::diar_install::DiarProgress::ModelFailed(MODEL_LABEL.to_string()));
        return Err(err).context("download Nemotron model");
    }

    if cancel.load(Ordering::Acquire) {
        let _ = std::fs::remove_file(&tmp);
        bail!("отменено");
    }

    on(crate::diar_install::DiarProgress::Verifying(MODEL_LABEL.to_string()));
    let meta = match std::fs::metadata(&tmp) {
        Ok(m) => m,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            on(crate::diar_install::DiarProgress::ModelFailed(MODEL_LABEL.to_string()));
            return Err(e).context("read metadata of downloaded model");
        }
    };
    if meta.len() != MODEL_SIZE {
        let _ = std::fs::remove_file(&tmp);
        on(crate::diar_install::DiarProgress::ModelFailed(MODEL_LABEL.to_string()));
        bail!(
            "размер модели Nemotron не совпал: ожидался {MODEL_SIZE}, получен {}",
            meta.len()
        );
    }

    if let Err(err) = crate::download::verify_sha256(&tmp, MODEL_SHA256, MODEL_LABEL) {
        on(crate::diar_install::DiarProgress::ModelFailed(MODEL_LABEL.to_string()));
        return Err(err);
    }

    #[cfg(windows)]
    if dest.exists() { std::fs::remove_file(&dest)?; }
    std::fs::rename(&tmp, &dest)
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
        .with_context(|| "commit Nemotron model")?;

    on(crate::diar_install::DiarProgress::AllInstalled);
    Ok(())
}

struct ChildGuard {
    child: std::process::Child,
    reaped: bool,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self {
            child,
            reaped: false,
        }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        if self.reaped {
            return Ok(None);
        }
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped = true;
        }
        Ok(status)
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.reaped {
            #[cfg(target_os = "linux")]
            {
                let pid = self.child.id() as libc::pid_t;
                if pid > 1 {
                    unsafe {
                        libc::kill(-pid, libc::SIGKILL);
                    }
                }
                let _ = self.child.kill();
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = self.child.kill();
            }
            let _ = self.child.wait();
            self.reaped = true;
        }
    }
}

enum PassOutcome {
    Success,
    Failed(std::process::ExitStatus),
    Canceled,
    TimedOut,
}

fn run_pass(
    exe: &Path,
    wav: &Path,
    model: &Path,
    rttm: &Path,
    device: &str,
    cancel: &AtomicBool,
) -> Result<PassOutcome> {
    let data_dir = crate::paths::data_root().context("data root unavailable")?;
    let _ = crate::paths::ensure_private_dir(&data_dir);
    let log_path = data_dir.join("nemotron-diar.log");
    let err_file = crate::paths::open_private_append(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?;

    let mut cmd = Command::new(exe);
    cmd.arg("diarize")
        .arg(wav)
        .arg("--verbose")
        .arg("--device")
        .arg(device)
        .args([
            "--preset",
            "v3-offline",
            "--format",
            "rttm",
            "--recording-id",
            "suflyor",
            "--output",
        ])
        .arg(rttm)
        .arg("--model")
        .arg(model)
        .env("GGML_BACKEND_DL_PATH", exe.parent().unwrap_or(Path::new(".")).join("../lib"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(err_file));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    crate::download::no_window(&mut cmd);

    let child = cmd.spawn().context("spawn Nemotron runtime")?;
    let mut guard = ChildGuard::new(child);
    let started = Instant::now();

    loop {
        if cancel.load(Ordering::Acquire) {
            return Ok(PassOutcome::Canceled);
        }
        if let Some(status) = guard.try_wait().context("wait for Nemotron")? {
            if status.success() {
                return Ok(PassOutcome::Success);
            }
            return Ok(PassOutcome::Failed(status));
        }
        if started.elapsed() > MAX_RUN_TIME {
            return Ok(PassOutcome::TimedOut);
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Diarize the given audio WAV using the native NeMo-Speech CLI.
/// Checks model size/digest before execution, attempts CUDA with CPU fallback on failure,
/// enforces execution timeout and cancellation polling, and parses output RTTM.
pub fn diarize(
    wav: &Path,
    duration_ms: i64,
    cancel: &AtomicBool,
) -> Result<(Vec<DiarSegment>, i64, String)> {
    if !wav.is_file() {
        bail!("audio recording unavailable");
    }

    let model = model_path().context("Nemotron model path unavailable")?;
    if !installed() {
        bail!("Nemotron model unavailable");
    }
    verify_model_file(&model)?;

    let exe = runtime_path().context("resolve Nemotron runtime path")?;
    if !exe.is_file() {
        bail!("Nemotron runtime unavailable");
    }

    let work = tempfile::tempdir().context("create private diarization workspace")?;
    let rttm = work.path().join("result.rttm");

    let first = run_pass(&exe, wav, &model, &rttm, "cuda:0", cancel)?;
    match first {
        PassOutcome::Success => {}
        PassOutcome::Canceled => {
            bail!("Nemotron canceled");
        }
        PassOutcome::TimedOut => {
            bail!("Nemotron timed out");
        }
        PassOutcome::Failed(status) => {
            if cancel.load(Ordering::Acquire) {
                bail!("Nemotron canceled");
            }
            log::warn!("Nemotron CUDA execution failed ({status}); retrying on CPU");
            let _ = std::fs::remove_file(&rttm);
            let second = run_pass(&exe, wav, &model, &rttm, "cpu", cancel)?;
            match second {
                PassOutcome::Success => {}
                PassOutcome::Canceled => bail!("Nemotron canceled"),
                PassOutcome::TimedOut => bail!("Nemotron timed out"),
                PassOutcome::Failed(cpu_status) => {
                    bail!("Nemotron CPU retry failed: {cpu_status}");
                }
            }
        }
    }

    let file = std::fs::File::open(&rttm).context("open Nemotron RTTM")?;
    let meta = file.metadata().context("read RTTM size")?;
    if meta.len() > MAX_RTTM_BYTES {
        bail!("RTTM too large");
    }

    let parsed = parse_rttm(BufReader::new(file), duration_ms)?;
    Ok((parsed.0, parsed.1, MODEL_ID.to_string()))
}

fn parse_rttm(reader: impl BufRead, duration_ms: i64) -> Result<(Vec<DiarSegment>, i64)> {
    if duration_ms <= 0 {
        bail!("invalid audio length");
    }
    let mut raw: Vec<(i64, i64, String)> = Vec::new();
    for line in reader.lines() {
        let line = line.context("read RTTM")?;
        if line.trim().is_empty() { continue; }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 10 || fields[0] != "SPEAKER" || fields[1] != "suflyor" {
            bail!("invalid RTTM record");
        }
        let start: f64 = fields[3].parse().context("RTTM start")?;
        let length: f64 = fields[4].parse().context("RTTM duration")?;
        if !start.is_finite()
            || !length.is_finite()
            || start < 0.0
            || length <= 0.0
            || (start + length) * 1000.0 > duration_ms as f64 + 20.0
            || fields[7].is_empty()
        {
            bail!("invalid RTTM segment");
        }
        let s = (start * 1000.0).round() as i64;
        let mut e = ((start + length) * 1000.0).round() as i64;
        if e > duration_ms {
            e = duration_ms;
        }
        if s >= e {
            bail!("RTTM segment too short");
        }
        raw.push((s, e, fields[7].to_string()));
    }
    if raw.is_empty() {
        bail!("no speakers detected");
    }
    raw.sort_by_key(|(start, end, _)| (*start, *end));
    let ids: BTreeSet<&str> = raw.iter().map(|(_, _, id)| id.as_str()).collect();
    if ids.len() > 8 {
        bail!("too many Nemotron speakers");
    }
    let mut map: BTreeMap<String, i32> = BTreeMap::new();
    let segments = raw
        .into_iter()
        .map(|(start_ms, end_ms, name)| {
            let next = map.len() as i32;
            let speaker = *map.entry(name).or_insert(next);
            DiarSegment {
                start_ms,
                end_ms,
                speaker,
            }
        })
        .collect();
    Ok((segments, map.len() as i64))
}
