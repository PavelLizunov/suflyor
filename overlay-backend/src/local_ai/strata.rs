//! Strata: the optional local engine for Qwen3.8-Flash-Next (ADR-003, phase P4).
//!
//! Strata is not one binary. Its own `START-HERE.bat` installs Python for the
//! user, a private `.venv`, the engine build that fits the GPU, and about 70 GB
//! of model files. Suflyor pins one source release (size + SHA-256), runs that
//! setup without questions and without starting the server, and then launches
//! the server itself on its own port, so it never collides with the managed
//! llama.cpp on :8080. Only one of the two engines runs at a time: both want
//! the whole GPU.
//!
//! Strata keeps its model files in `Strata-data` next to its folder, or in the
//! data folder an earlier Strata install remembered in
//! `%APPDATA%\Strata\settings.json`, so an existing download is reused.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use super::{
    curl_resumable, curl_success_body, detect_nvidia_memory_mib, ensure_disk_space, extract_zip,
    free_llama_port, is_loopback_v1_on_port, is_reachable, llama_reachable,
    llama_reply_has_text_content, pinned_file_matches, terminate_child_tree,
    wait_for_nvidia_vram_release, Progress, GIB,
};

/// The pinned Strata source release (GitHub tag `v0.1.44.1`, 2026-10-14).
pub const STRATA_VERSION: &str = "0.1.44.1";
const STRATA_SOURCE_URL: &str =
    "https://github.com/Niko1221/Strata/archive/refs/tags/v0.1.44.1.zip";
const STRATA_SOURCE_SIZE: u64 = 22_629_639;
const STRATA_SOURCE_SHA256: &str =
    "22be94124b4d37dc2ff76e742c49498a3e5f31873ed916a636b2a31707d09b31";

/// Strata's own default is :8080, the managed llama.cpp port (ADR-003, decision 3).
pub const STRATA_PORT: &str = "8090";
pub const STRATA_BASE_URL: &str = "http://127.0.0.1:8090/v1";
const STRATA_HEALTH_URL: &str = "http://127.0.0.1:8090/health";

/// The original model in the size Strata recommends for 48-64 GB of RAM.
/// Fine-tunes (Swift and others) carry their own licences and stay out until
/// those are checked (ADR-003, decision 5).
const STRATA_FAMILY: &str = "qwen";
const STRATA_SIZE: &str = "IQ2_XS";
pub const STRATA_MODEL_LABEL: &str = "Qwen3.8-Flash-Next IQ2_XS";
pub const STRATA_MODEL_LICENSE_URL: &str = "https://huggingface.co/Qwen/Qwen3.8-Flash-Next";

/// ~66 GB of model files, ~6 GB MTP draft layer, the engine and the venv.
const STRATA_DISK_BYTES: u64 = 76 * GIB;
/// Strata keeps the experts in RAM: 35.5 GB for IQ2_XS plus ~10 GB for Windows.
pub const STRATA_MIN_RAM_GIB: u64 = 48;
/// 8 GB cards start, slowly; Strata's documentation asks for 12 GB.
pub const STRATA_MIN_VRAM_GIB: u64 = 12;
/// Prefix of the error when Strata's own setup stops on a check it prints as
/// `[X] ...` (no NVIDIA driver, too little RAM): the UI shows that line.
pub const STRATA_SETUP_STOPPED: &str = "Strata setup stopped: ";
/// The first start loads 35-55 GB into RAM; later starts take 30-90 s.
const STRATA_READY_BUDGET: Duration = Duration::from_secs(600);

static STRATA_CHILD: Mutex<Option<Child>> = Mutex::new(None);
/// Set by a Settings switch that is waiting for the lifecycle lock, so a
/// boot or watchdog start of Strata (which holds that lock while the model
/// loads) gives up instead of making the user wait up to ten minutes.
static STRATA_START_ABORT: AtomicBool = AtomicBool::new(false);

/// Ask a Strata start that is still loading to stop.
pub fn abort_strata_start() {
    STRATA_START_ABORT.store(true, Ordering::Relaxed);
}

#[must_use]
pub fn strata_dir(root: &Path) -> PathBuf {
    root.join("strata").join(format!("Strata-{STRATA_VERSION}"))
}

fn strata_python(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        dir.join(".venv").join("Scripts").join("python.exe")
    } else {
        dir.join(".venv").join("bin").join("python")
    }
}

/// Setup names the run config after the family tag and the size; the original
/// model has an empty tag (`strata-iq2_xs.json`).
fn strata_config(dir: &Path) -> PathBuf {
    dir.join(format!("strata-{}.json", STRATA_SIZE.to_ascii_lowercase()))
}

/// Stat-only: the venv and the run config that setup writes last both exist.
#[must_use]
pub fn strata_installed(root: &Path) -> bool {
    let dir = strata_dir(root);
    strata_python(&dir).is_file() && strata_config(&dir).is_file()
}

/// True only for Suflyor's loopback Strata endpoint.
#[must_use]
pub fn is_managed_strata_endpoint(base_url: &str) -> bool {
    is_loopback_v1_on_port(base_url, STRATA_PORT)
}

/// What this PC has against Strata's needs. Detection runs PowerShell and
/// nvidia-smi, so callers keep it off the UI thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrataHardware {
    pub ram_gib: Option<u64>,
    pub vram_gib: Option<u64>,
}

impl StrataHardware {
    #[must_use]
    pub fn detect() -> Self {
        Self {
            ram_gib: super::detect_system_ram_gib(),
            vram_gib: detect_nvidia_memory_mib().map(|(_, total)| total.div_ceil(1024)),
        }
    }

    /// Below either limit Strata may still run (low-RAM mode, 8 GB cards), only
    /// slowly; the UI warns and leaves the choice to the user.
    #[must_use]
    pub fn meets_recommendation(self) -> bool {
        self.ram_gib.is_some_and(|ram| ram >= STRATA_MIN_RAM_GIB)
            && self
                .vram_gib
                .is_some_and(|vram| vram >= STRATA_MIN_VRAM_GIB)
    }

    #[must_use]
    pub fn describe(self, ru: bool) -> String {
        let ram = self
            .ram_gib
            .map_or_else(|| "?".to_string(), |v| v.to_string());
        let gpu = match (self.vram_gib, ru) {
            (Some(vram), true) => format!("видеокарта NVIDIA {vram} ГБ"),
            (Some(vram), false) => format!("NVIDIA GPU {vram} GB"),
            (None, true) => "видеокарта NVIDIA не найдена".to_string(),
            (None, false) => "no NVIDIA GPU found".to_string(),
        };
        match (ru, self.meets_recommendation()) {
            (true, true) => format!("Этот ПК: {ram} ГБ RAM, {gpu} — подходит."),
            (true, false) => format!(
                "Этот ПК: {ram} ГБ RAM, {gpu}. Нужно от {STRATA_MIN_RAM_GIB} ГБ RAM и от {STRATA_MIN_VRAM_GIB} ГБ VRAM: модель может не запуститься или работать очень медленно."
            ),
            (false, true) => format!("This PC: {ram} GB RAM, {gpu} — fits."),
            (false, false) => format!(
                "This PC: {ram} GB RAM, {gpu}. Strata needs {STRATA_MIN_RAM_GIB} GB RAM and {STRATA_MIN_VRAM_GIB} GB VRAM or more: the model may not start or may run very slowly."
            ),
        }
    }
}

// ---- install ---------------------------------------------------------------

/// Download the pinned Strata source, check it, and run its setup without
/// questions and without starting the server. Resumable: a second run reuses
/// the unpacked source, and Strata's setup resumes its own downloads.
pub fn install_strata(root: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress)) -> Result<()> {
    let base = root.join("strata");
    std::fs::create_dir_all(&base).with_context(|| format!("create {}", base.display()))?;
    let dir = strata_dir(root);
    if !dir.join("START-HERE.bat").is_file() {
        // Only a fresh install: a resumed one already holds part of the model
        // files, possibly in a data folder on another drive.
        ensure_disk_space(&base, STRATA_DISK_BYTES, on)?;
        let zip = base.join(format!("strata-{STRATA_VERSION}.zip"));
        on(Progress::Step(format!(
            "Downloading Strata {STRATA_VERSION}"
        )));
        curl_resumable(
            STRATA_SOURCE_URL,
            &zip,
            STRATA_SOURCE_SIZE,
            "Strata",
            cancel,
            on,
        )?;
        if !pinned_file_matches(&zip, STRATA_SOURCE_SIZE, STRATA_SOURCE_SHA256) {
            let _ = std::fs::remove_file(&zip);
            bail!("the Strata download failed its integrity check");
        }
        extract_zip(&zip, &base)?;
        let _ = std::fs::remove_file(&zip);
        if !dir.join("START-HERE.bat").is_file() {
            bail!("the Strata archive did not contain START-HERE.bat");
        }
    }
    run_setup(&dir, cancel, on)?;
    if !strata_installed(root) {
        bail!("Strata setup finished without an installed model");
    }
    Ok(())
}

#[cfg(windows)]
fn run_setup(dir: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress)) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    use super::CANCEL_SENTINEL;

    let mut cmd = Command::new("cmd.exe");
    cmd.args([
        "/d",
        "/c",
        "START-HERE.bat",
        "--yes",
        "--family",
        STRATA_FAMILY,
        "--model",
        STRATA_SIZE,
        "--no-start",
        "--no-browser",
        "--port",
        STRATA_PORT,
    ])
    .current_dir(dir)
    .env("PYTHONUNBUFFERED", "1")
    .env("PYTHONIOENCODING", "utf-8")
    // A closed stdin makes the batch file's `pause` return at once.
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    crate::download::no_window(&mut cmd);
    let mut child = cmd.spawn().context("start Strata setup")?;
    super::assign_to_lifetime_job(&child);

    let (tx, rx) = mpsc::channel::<String>();
    let mut readers = Vec::new();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || forward_lines(out, &tx)));
    }
    if let Some(err) = child.stderr.take() {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || forward_lines(err, &tx)));
    }
    drop(tx);

    let mut log = std::fs::File::create(dir.join("suflyor-setup.log")).ok();
    let mut tail: Vec<String> = Vec::new();
    let mut stopped_on: Option<String> = None;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            terminate_child_tree(child);
            bail!("{CANCEL_SENTINEL}");
        }
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(line) => {
                if let Some(file) = log.as_mut() {
                    let _ = writeln!(file, "{line}");
                }
                if let Some(reason) = setup_stop_reason(&line) {
                    stopped_on = Some(reason);
                }
                on(Progress::Step(line.clone()));
                tail.push(line);
                if tail.len() > 6 {
                    tail.remove(0);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            // Both pipes closed: the setup has ended or is about to.
            Err(mpsc::RecvTimeoutError::Disconnected) => break child.wait()?,
        }
        if let Some(status) = child.try_wait()? {
            // Drain what the readers still hold.
            for line in rx.try_iter() {
                if let Some(file) = log.as_mut() {
                    let _ = writeln!(file, "{line}");
                }
                if let Some(reason) = setup_stop_reason(&line) {
                    stopped_on = Some(reason);
                }
                tail.push(line);
            }
            break status;
        }
    };
    // Not joined: a process the setup left behind could hold a pipe open; the
    // readers end when the last writer closes it.
    drop(readers);
    // START-HERE.bat ends with `pause` after a failure, so its exit code can be
    // 0; the `[X]` line is the reliable sign.
    if let Some(reason) = stopped_on {
        bail!("{STRATA_SETUP_STOPPED}{reason}");
    }
    if !status.success() {
        let last = tail
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .cloned()
            .unwrap_or_default();
        bail!("Strata setup failed ({status}): {last}");
    }
    Ok(())
}

#[cfg(not(windows))]
fn run_setup(_dir: &Path, _cancel: &AtomicBool, _on: &dyn Fn(Progress)) -> Result<()> {
    bail!("the Strata engine is installed from the Windows app only")
}

/// Read a pipe and send one cleaned line per `\n` or `\r` (download meters
/// redraw one line with `\r`).
#[cfg_attr(not(windows), allow(dead_code))]
fn forward_lines<R: std::io::Read>(mut pipe: R, tx: &std::sync::mpsc::Sender<String>) {
    let mut splitter = LineSplitter::default();
    let mut buf = [0_u8; 4096];
    loop {
        let n = match pipe.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for line in splitter.push(&buf[..n]) {
            if tx.send(line).is_err() {
                return;
            }
        }
    }
    if let Some(line) = splitter.finish() {
        let _ = tx.send(line);
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Default)]
struct LineSplitter {
    pending: Vec<u8>,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl LineSplitter {
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for &b in bytes {
            if b == b'\n' || b == b'\r' {
                if let Some(line) = clean_setup_line(&self.pending) {
                    lines.push(line);
                }
                self.pending.clear();
            } else {
                self.pending.push(b);
            }
        }
        lines
    }

    fn finish(&mut self) -> Option<String> {
        let line = clean_setup_line(&self.pending);
        self.pending.clear();
        line
    }
}

/// Strata's setup prints a failed check as `[X]  <reason>`.
#[cfg_attr(not(windows), allow(dead_code))]
fn setup_stop_reason(line: &str) -> Option<String> {
    line.strip_prefix("[X]")
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(str::to_owned)
}

/// Drop ANSI escape sequences, trim, and cap the length for a status line.
#[cfg_attr(not(windows), allow(dead_code))]
fn clean_setup_line(raw: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(raw);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if !c.is_control() {
            out.push(c);
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        return None;
    }
    const MAX: usize = 160;
    Some(if trimmed.chars().count() > MAX {
        let mut cut: String = trimmed.chars().take(MAX).collect();
        cut.push('…');
        cut
    } else {
        trimmed.to_string()
    })
}

// ---- server ----------------------------------------------------------------

/// Outcome of starting Strata, so the UI never claims success it did not see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrataStart {
    /// `/health` reports the model loaded and a short completion answered.
    Ready,
    /// The venv or the run config is missing.
    NotInstalled,
    /// Something we did not start already listens on :8090.
    PortBusy,
    /// The server exited or did not become ready in time.
    FailedToStart,
    /// The user cancelled while the model was loading.
    Cancelled,
}

fn health_reports_loaded(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|json| json.get("loaded").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

fn first_model_id(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("data")?
        .as_array()?
        .first()?
        .get("id")?
        .as_str()
        .map(str::to_owned)
}

/// True if anything answers on :8090 (even while the model loads).
#[must_use]
pub fn strata_reachable() -> bool {
    is_reachable(STRATA_HEALTH_URL)
}

/// The model id the running server reports, used as `ai_local_model`.
#[must_use]
pub fn served_strata_model() -> Option<String> {
    curl_success_body(&[
        "-f",
        "-sS",
        "--max-time",
        "3",
        &format!("{STRATA_BASE_URL}/models"),
    ])
    .and_then(|body| first_model_id(&body))
}

fn strata_answers() -> bool {
    let health = curl_success_body(&["-f", "-sS", "--max-time", "3", STRATA_HEALTH_URL]);
    if !health.as_deref().is_some_and(health_reports_loaded) {
        return false;
    }
    let body = serde_json::json!({
        "model": "strata",
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 4,
        "reasoning_effort": "none",
        "chat_template_kwargs": {"enable_thinking": false},
    })
    .to_string();
    curl_success_body(&[
        "-f",
        "-sS",
        "--max-time",
        "60",
        "-X",
        "POST",
        &format!("{STRATA_BASE_URL}/chat/completions"),
        "-H",
        "Content-Type: application/json",
        "-d",
        &body,
    ])
    .as_deref()
    .is_some_and(llama_reply_has_text_content)
}

/// True while the server this process started is alive.
#[must_use]
pub fn strata_running() -> bool {
    STRATA_CHILD
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_mut()
        .is_some_and(|child| matches!(child.try_wait(), Ok(None)))
}

/// Stop the server this process started (the whole tree: setup's Python
/// launcher, the server and the engine). A no-op when none runs.
pub fn stop_strata() {
    let child = STRATA_CHILD
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    if let Some(child) = child {
        terminate_child_tree(child);
    }
}

fn launch_strata(root: &Path) -> Result<Child> {
    use std::process::{Command, Stdio};

    let dir = strata_dir(root);
    let config = strata_config(&dir);
    let log = std::fs::File::create(dir.join("suflyor-server.log")).context("server log")?;
    let log_err = log.try_clone().context("server log")?;
    // The same command line as Strata's own run-<model>.bat.
    let mut cmd = Command::new(strata_python(&dir));
    cmd.arg(dir.join("serve").join("server.py"))
        .args(["--engine", "strata", "--config"])
        .arg(&config)
        .args(["--port", STRATA_PORT])
        .current_dir(&dir)
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    crate::download::no_window(&mut cmd);
    let child = cmd.spawn().context("start the Strata server")?;
    #[cfg(windows)]
    super::assign_to_lifetime_job(&child);
    Ok(child)
}

/// Start Strata and wait until it answers. Stops the managed llama.cpp first,
/// because both engines need the GPU. On any failure the half-started server
/// is stopped; restoring the previous engine is the caller's decision.
pub fn start_strata(root: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress)) -> StrataStart {
    if !strata_installed(root) {
        return StrataStart::NotInstalled;
    }
    if strata_running() && strata_answers() {
        return StrataStart::Ready;
    }
    stop_strata();
    if strata_reachable() {
        return StrataStart::PortBusy;
    }
    let had_llama = llama_reachable();
    let used_vram = had_llama
        .then(detect_nvidia_memory_mib)
        .flatten()
        .map(|(used, _)| used);
    // Owner-aware: a foreign :8080 listener stays; only ours is stopped.
    let _ = free_llama_port(root);
    if had_llama && !wait_for_nvidia_vram_release(used_vram, Duration::from_secs(10)) {
        log::warn!("strata: VRAM did not release after stopping llama-server");
    }
    STRATA_START_ABORT.store(false, Ordering::Relaxed);
    on(Progress::Step("Starting Strata".to_string()));
    let child = match launch_strata(root) {
        Ok(child) => child,
        Err(e) => {
            log::warn!("strata: launch failed: {e:#}");
            return StrataStart::FailedToStart;
        }
    };
    *STRATA_CHILD.lock().unwrap_or_else(|p| p.into_inner()) = Some(child);
    let started = Instant::now();
    while started.elapsed() < STRATA_READY_BUDGET {
        if cancel.load(Ordering::Relaxed) || STRATA_START_ABORT.load(Ordering::Relaxed) {
            stop_strata();
            return StrataStart::Cancelled;
        }
        if !strata_running() {
            log::warn!("strata: the server exited while loading");
            stop_strata();
            return StrataStart::FailedToStart;
        }
        if strata_answers() {
            log::info!("strata ready: load_ms={}", started.elapsed().as_millis());
            return StrataStart::Ready;
        }
        on(Progress::Step(format!(
            "Loading the model ({} s)",
            started.elapsed().as_secs()
        )));
        std::thread::sleep(Duration::from_secs(2));
    }
    log::warn!("strata: not ready within {STRATA_READY_BUDGET:?}");
    stop_strata();
    StrataStart::FailedToStart
}

/// Point the local provider at Strata after a confirmed start.
pub fn apply_strata_choice(cfg: &mut crate::config::Config, model_id: &str) {
    cfg.ai_local_base_url = STRATA_BASE_URL.to_string();
    cfg.ai_local_model = model_id.to_string();
    cfg.ai_local_prep_model.clear();
    cfg.ai_local_quality = false;
    // Setup ran without the image encoder.
    cfg.ai_local_vision = false;
    if cfg.vision_provider == "same" {
        cfg.vision_provider = "off".to_string();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn strata_endpoint_is_loopback_8090_only() {
        assert!(is_managed_strata_endpoint("http://127.0.0.1:8090/v1"));
        assert!(is_managed_strata_endpoint("http://localhost:8090/v1/"));
        assert!(!is_managed_strata_endpoint("http://127.0.0.1:8080/v1"));
        assert!(!is_managed_strata_endpoint("http://192.168.1.5:8090/v1"));
        assert!(!is_managed_strata_endpoint("https://127.0.0.1:8090/v1"));
        assert!(!super::super::is_managed_llama_endpoint(STRATA_BASE_URL));
    }

    #[test]
    fn health_needs_loaded_true() {
        assert!(health_reports_loaded(r#"{"status":"ok","loaded":true}"#));
        assert!(!health_reports_loaded(r#"{"status":"ok","loaded":false}"#));
        assert!(!health_reports_loaded(r#"{"status":"ok"}"#));
        assert!(!health_reports_loaded("not json"));
    }

    #[test]
    fn first_model_id_reads_the_openai_list() {
        assert_eq!(
            first_model_id(r#"{"object":"list","data":[{"id":"qwen3.8-flash-next-iq2_xs"}]}"#)
                .as_deref(),
            Some("qwen3.8-flash-next-iq2_xs")
        );
        assert_eq!(first_model_id(r#"{"data":[]}"#), None);
    }

    #[test]
    fn setup_output_splits_on_carriage_returns_and_drops_ansi() {
        let mut s = LineSplitter::default();
        let lines = s.push(b"\x1b[32mstep 1\x1b[0m\r\n  12%\r  13%\rpart");
        assert_eq!(lines, vec!["step 1", "12%", "13%"]);
        assert_eq!(s.push(b"ial\n"), vec!["partial"]);
        assert_eq!(s.finish(), None);
    }

    #[test]
    fn setup_stop_reason_reads_the_failed_check() {
        assert_eq!(
            setup_stop_reason("[X]  no NVIDIA GPU found (nvidia-smi did not answer)").as_deref(),
            Some("no NVIDIA GPU found (nvidia-smi did not answer)")
        );
        assert_eq!(setup_stop_reason("=== Step 1: checking your PC ==="), None);
        assert_eq!(setup_stop_reason("[X]"), None);
    }

    #[test]
    fn long_setup_lines_are_capped() {
        let line = clean_setup_line("x".repeat(400).as_bytes()).unwrap();
        assert_eq!(line.chars().count(), 161);
        assert!(line.ends_with('…'));
    }

    #[test]
    fn installed_needs_venv_and_run_config() {
        let root = std::env::temp_dir().join(format!("suflyor-strata-test-{}", std::process::id()));
        let dir = strata_dir(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert!(!strata_installed(&root));
        std::fs::create_dir_all(strata_python(&dir).parent().unwrap()).unwrap();
        std::fs::write(strata_python(&dir), b"").unwrap();
        assert!(!strata_installed(&root));
        std::fs::write(strata_config(&dir), b"{}").unwrap();
        assert!(strata_installed(&root));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn hardware_recommendation() {
        let fits = StrataHardware {
            ram_gib: Some(64),
            vram_gib: Some(16),
        };
        assert!(fits.meets_recommendation());
        assert!(!StrataHardware {
            ram_gib: Some(32),
            vram_gib: Some(16)
        }
        .meets_recommendation());
        assert!(!StrataHardware {
            ram_gib: Some(64),
            vram_gib: None
        }
        .meets_recommendation());
        assert!(fits.describe(false).contains("fits"));
        assert!(StrataHardware {
            ram_gib: Some(16),
            vram_gib: None
        }
        .describe(false)
        .contains("no NVIDIA GPU found"));
    }

    #[test]
    fn applying_strata_turns_off_local_vision() {
        let mut cfg = crate::config::Config {
            ai_local_vision: true,
            vision_provider: "same".to_string(),
            ai_local_quality: true,
            ai_local_prep_model: "x".to_string(),
            ..Default::default()
        };
        apply_strata_choice(&mut cfg, "qwen3.8-flash-next-iq2_xs");
        assert_eq!(cfg.ai_local_base_url, STRATA_BASE_URL);
        assert_eq!(cfg.ai_local_model, "qwen3.8-flash-next-iq2_xs");
        assert!(!cfg.ai_local_vision);
        assert!(!cfg.ai_local_quality);
        assert!(cfg.ai_local_prep_model.is_empty());
        assert_eq!(cfg.vision_provider, "off");
    }
}
