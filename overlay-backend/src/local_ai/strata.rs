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

#[path = "strata_vision.rs"]
mod vision;
pub use vision::*;

use super::{
    curl_resumable, curl_success_body, detect_nvidia_memory_mib, ensure_disk_space, extract_zip,
    free_llama_port, is_loopback_v1_on_port, is_reachable, llama_reachable,
    llama_reply_has_text_content, pinned_file_matches, spawn_owned, terminate_child_tree,
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
#[cfg(any(windows, target_os = "linux"))]
const STRATA_FAMILY: &str = "qwen";
const STRATA_SIZE: &str = "IQ2_XS";
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
/// The run config the running server was started with: choosing another
/// Strata install restarts the server.
static STRATA_CHILD_CONFIG: Mutex<Option<PathBuf>> = Mutex::new(None);
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

/// Stat-only: Suflyor's own install has the venv and the run config that
/// setup writes last.
#[must_use]
pub fn own_strata_installed(root: &Path) -> bool {
    let dir = strata_dir(root);
    strata_python(&dir).is_file() && strata_config(&dir).is_file()
}

/// The Strata that starts: a folder the user installed and chose, while it
/// is still usable, or else Suflyor's own install.
#[must_use]
pub fn strata_installed(root: &Path) -> bool {
    adopted_strata().is_some() || own_strata_installed(root)
}

// ---- a Strata the user installed before ------------------------------------

/// A Strata folder and the model run config in it (`strata-<model>.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundStrata {
    pub dir: PathBuf,
    pub config: PathBuf,
    /// The config's `model_name`, else the tag in its file name.
    pub model: String,
}

impl FoundStrata {
    /// The venv, the server script and the model config are all in place.
    #[must_use]
    pub fn usable(&self) -> bool {
        strata_python(&self.dir).is_file()
            && self.dir.join("serve").join("server.py").is_file()
            && model_config_name(&self.config).is_some()
    }
}

static ADOPTED: Mutex<Option<FoundStrata>> = Mutex::new(None);

/// Use the user's own Strata (`dir` + `config` from the saved config), or,
/// with an empty `dir`, Suflyor's install. Called at boot and on a choice.
pub fn set_adopted_strata(dir: &str, config: &str) {
    let found = (!dir.trim().is_empty() && !config.trim().is_empty()).then(|| {
        let config = PathBuf::from(config);
        FoundStrata {
            dir: PathBuf::from(dir),
            model: model_config_name(&config).unwrap_or_default(),
            config,
        }
    });
    *ADOPTED.lock().unwrap_or_else(|p| p.into_inner()) = found;
}

/// The chosen Strata while its folder is still usable. A moved or deleted
/// folder, or paths from another PC's profile, fall back to Suflyor's own.
#[must_use]
pub fn adopted_strata() -> Option<FoundStrata> {
    ADOPTED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .filter(FoundStrata::usable)
}

/// A Strata model run config is a JSON object with `exe` and `args`; the
/// server keeps its chat settings beside it as `*.shared-settings.json`.
/// Returns the model name to show.
fn model_config_name(path: &Path) -> Option<String> {
    let file = path.file_name()?.to_str()?;
    let tag = file.strip_prefix("strata-")?.strip_suffix(".json")?;
    if tag.ends_with(".shared-settings") {
        return None;
    }
    let cfg = parse_json_file(&std::fs::read_to_string(path).ok()?);
    if cfg.get("exe").and_then(serde_json::Value::as_str).is_none()
        || !cfg.get("args").is_some_and(serde_json::Value::is_array)
    {
        return None;
    }
    Some(
        cfg.get("model_name")
            .and_then(serde_json::Value::as_str)
            .filter(|n| !n.trim().is_empty())
            .map_or_else(|| tag.to_ascii_uppercase(), str::to_owned),
    )
}

/// JSON a user may have saved from Notepad (with a UTF-8 BOM); Null when
/// it does not parse.
fn parse_json_file(text: &str) -> serde_json::Value {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or(serde_json::Value::Null)
}

/// The usable model configs of one Strata folder, newest first.
#[must_use]
pub fn strata_in_folder(dir: &Path) -> Vec<FoundStrata> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(std::time::SystemTime, FoundStrata)> = entries
        .flatten()
        .map(|e| e.path())
        .filter_map(|config| {
            let model = model_config_name(&config)?;
            let at = std::fs::metadata(&config)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            Some((
                at,
                FoundStrata {
                    dir: dir.to_path_buf(),
                    config,
                    model,
                },
            ))
        })
        .filter(|(_, f)| f.usable())
        .collect();
    found.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    found.into_iter().map(|(_, f)| f).collect()
}

/// Strata's per-user file: the folders this user ran Strata from, and where
/// the model files are.
fn strata_settings_file() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(dirs::config_dir)
            .map(|d| d.join("Strata").join("settings.json"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .map(|d| d.join("strata").join("settings.json"))
    }
}

/// Fixed drive roots (`C:\`, `D:\`…). Network and removable drives are left
/// out: listing them can hang or wake a disk.
#[cfg(windows)]
fn fixed_drives() -> Vec<PathBuf> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_FIXED: u32 = 3;
    // SAFETY: no arguments; returns a bit mask of drive letters.
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", char::from(b'A' + i)))
        .filter(|root| {
            let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call.
            unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) == DRIVE_FIXED }
        })
        .map(PathBuf::from)
        .collect()
}

#[cfg(not(windows))]
fn fixed_drives() -> Vec<PathBuf> {
    Vec::new()
}

/// A folder that may hold Strata: named `Strata…`, or with Strata's server
/// script inside.
fn looks_like_strata(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        n.to_ascii_lowercase().starts_with("strata") && !n.eq_ignore_ascii_case("strata-data")
    }) || path.join("serve").join("server.py").is_file()
}

/// Strata-looking folders in `base` and, with `depth` 2, one level below
/// (`D:\AI\Strata`). Folders the user does not keep programs in are skipped.
fn strata_folders_under(base: &Path, depth: u8, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    // Real folders only: a link or junction may point at a slow or missing
    // network share.
    for path in entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
    {
        if looks_like_strata(&path) {
            out.push(path);
            continue;
        }
        let skip = path.file_name().and_then(|n| n.to_str()).is_none_or(|n| {
            n.starts_with(['$', '.'])
                || [
                    "windows",
                    "programdata",
                    "appdata",
                    "node_modules",
                    "system volume information",
                ]
                .contains(&n.to_ascii_lowercase().as_str())
        });
        if depth > 1 && !skip {
            strata_folders_under(&path, depth - 1, out);
        }
    }
}

/// Where people unpack a downloaded Strata: the drive roots and a folder
/// below them, the home folder, Desktop, Downloads and Documents.
fn usual_strata_folders() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for drive in fixed_drives() {
        strata_folders_under(&drive, 2, &mut out);
    }
    if let Some(home) = dirs::home_dir() {
        strata_folders_under(&home, 2, &mut out);
    }
    for dir in [
        dirs::desktop_dir(),
        dirs::download_dir(),
        dirs::document_dir(),
    ]
    .into_iter()
    .flatten()
    {
        strata_folders_under(&dir, 2, &mut out);
    }
    out
}

/// Strata installs on this PC other than Suflyor's own: the folders Strata's
/// own setup remembered, `Strata*` folders next to them and next to its data
/// folder, then the usual places on fixed drives (two folder levels deep).
/// Lists a few hundred folders; callers keep it off the UI thread.
#[must_use]
pub fn find_existing_strata(root: &Path) -> Vec<FoundStrata> {
    let settings = strata_settings_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map_or(serde_json::Value::Null, |t| parse_json_file(&t));
    let mut folders: Vec<PathBuf> = settings
        .get("installs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(PathBuf::from)
        .collect();
    let mut parents: Vec<PathBuf> = folders
        .iter()
        .filter_map(|f| f.parent().map(Path::to_path_buf))
        .collect();
    if let Some(data) = settings.get("data_dir").and_then(serde_json::Value::as_str) {
        if let Some(parent) = Path::new(data).parent() {
            parents.push(parent.to_path_buf());
        }
    }
    for parent in parents {
        if let Ok(entries) = std::fs::read_dir(&parent) {
            folders.extend(entries.flatten().map(|e| e.path()).filter(|p| {
                p.is_dir()
                    && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.to_ascii_lowercase().starts_with("strata")
                            && !n.eq_ignore_ascii_case("strata-data")
                    })
            }));
        }
    }
    folders.extend(usual_strata_folders());
    let own = strata_dir(root);
    let mut seen = Vec::new();
    let mut found = Vec::new();
    for folder in folders {
        let key = folder.to_string_lossy().to_ascii_lowercase();
        if seen.contains(&key) || same_dir(&folder, &own) {
            continue;
        }
        seen.push(key);
        if let Some(first) = strata_in_folder(&folder).into_iter().next() {
            found.push(first);
        }
    }
    found
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
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

    /// Compact Settings caption; call on the hardware-discovery thread.
    #[must_use]
    pub fn describe_short(self, ru: bool) -> String {
        let ram = self
            .ram_gib
            .map_or_else(|| "?".to_string(), |v| v.to_string());
        let name = self.vram_gib.and_then(|_| {
            let out = super::run_capture(
                "nvidia-smi",
                &[
                    "--query-gpu=name,memory.total",
                    "--format=csv,noheader,nounits",
                ],
            )
            .ok()?;
            if !out.status.success() {
                return None;
            }
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let (name, memory) = line.rsplit_once(',')?;
                    Some((name.trim().to_string(), memory.trim().parse::<u64>().ok()?))
                })
                .max_by_key(|(_, memory)| *memory)
                .map(|(name, _)| name)
        });
        let gpu = match (self.vram_gib, ru) {
            (Some(vram), true) => format!("{} с {vram} ГБ", name.as_deref().unwrap_or("NVIDIA")),
            (Some(vram), false) => {
                format!("{} with {vram} GB", name.as_deref().unwrap_or("NVIDIA GPU"))
            }
            (None, true) => "нет видеокарты NVIDIA".to_string(),
            (None, false) => "no NVIDIA graphics card".to_string(),
        };
        if ru {
            format!("Этот ПК: {ram} ГБ RAM, {gpu}")
        } else {
            format!("This PC: {ram} GB RAM, {gpu}")
        }
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
    if !own_strata_installed(root) {
        bail!("Strata setup finished without an installed model");
    }
    Ok(())
}

#[cfg(any(windows, target_os = "linux"))]
fn run_setup(dir: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress)) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    use super::CANCEL_SENTINEL;

    #[cfg(windows)]
    let mut cmd = {
        let mut command = Command::new("cmd.exe");
        command.args(["/d", "/c", "START-HERE.bat"]);
        command
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut command = Command::new("/bin/sh");
        command.arg("setup.sh");
        command
    };
    if let Some(storage) = crate::paths::model_storage_root() {
        cmd.arg("--data-dir").arg(storage.join("Strata-data"));
    }
    cmd.args([
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
    let mut child = spawn_owned(&mut cmd).context("start Strata setup")?;

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

#[cfg(not(any(windows, target_os = "linux")))]
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

/// The folder and run config of the Strata that starts.
fn active_location(root: &Path) -> (PathBuf, PathBuf) {
    match adopted_strata() {
        Some(found) => (found.dir, found.config),
        None => {
            let dir = strata_dir(root);
            let config = strata_config(&dir);
            (dir, config)
        }
    }
}

fn launch_strata(root: &Path) -> Result<Child> {
    use std::process::{Command, Stdio};

    let (dir, config) = active_location(root);
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
    spawn_owned(&mut cmd).context("start the Strata server")
}

/// Start Strata and wait until it answers. Stops the managed llama.cpp first,
/// because both engines need the GPU. On any failure the half-started server
/// is stopped; restoring the previous engine is the caller's decision.
pub fn start_strata(root: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress)) -> StrataStart {
    if !strata_installed(root) {
        return StrataStart::NotInstalled;
    }
    let (_, config) = active_location(root);
    let same_install = STRATA_CHILD_CONFIG
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        == Some(&config);
    if same_install && strata_running() && strata_answers() {
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
    *STRATA_CHILD_CONFIG
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some(config);
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

/// Use a Strata the user started on :8090 instead of starting one: frees
/// the built-in engine (both need the GPU), then checks that it answers.
/// Callers hold the lifecycle lock.
#[must_use]
pub fn use_running_strata(root: &Path) -> StrataStart {
    if !strata_answers() {
        return if strata_reachable() {
            StrataStart::PortBusy
        } else {
            StrataStart::FailedToStart
        };
    }
    // Owner-aware: a foreign :8080 listener stays; only ours is stopped.
    let _ = free_llama_port(root);
    log::info!("strata: using the server already running on :{STRATA_PORT}");
    StrataStart::Ready
}

/// Point the local provider at Strata after a confirmed start.
pub fn apply_strata_choice(cfg: &mut crate::config::Config, model_id: &str) {
    cfg.ai_local_base_url = STRATA_BASE_URL.to_string();
    cfg.ai_local_model = model_id.to_string();
    cfg.ai_local_prep_model.clear();
    cfg.ai_local_quality = false;
    // Preserve an explicitly enabled encoder; merely downloading it never
    // declares image capability or changes the screenshot route.
    cfg.ai_local_vision &= strata_vision_available(&super::default_root());
    if !cfg.ai_local_vision && cfg.vision_provider == "same" {
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
        assert!(!own_strata_installed(&root));
        std::fs::create_dir_all(strata_python(&dir).parent().unwrap()).unwrap();
        std::fs::write(strata_python(&dir), b"").unwrap();
        assert!(!own_strata_installed(&root));
        std::fs::write(strata_config(&dir), b"{}").unwrap();
        assert!(own_strata_installed(&root));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_model_configs_in_a_strata_folder() {
        let dir = std::env::temp_dir().join(format!("suflyor-found-strata-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("serve")).unwrap();
        std::fs::write(dir.join("serve").join("server.py"), b"").unwrap();
        std::fs::write(
            dir.join("strata-iq3_s.json"),
            br#"{"exe":"strata.exe","args":[],"model_name":"swift-1.5"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("strata-iq3_s.shared-settings.json"),
            br#"{"exe":"x","args":[]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("strata-notes.json"), b"{}").unwrap();
        // No venv yet: not usable.
        assert!(strata_in_folder(&dir).is_empty());
        std::fs::create_dir_all(strata_python(&dir).parent().unwrap()).unwrap();
        std::fs::write(strata_python(&dir), b"").unwrap();
        let found = strata_in_folder(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].model, "swift-1.5");
        assert_eq!(found[0].config, dir.join("strata-iq3_s.json"));
        std::fs::write(
            dir.join("strata-iq2_xs.json"),
            "\u{feff}{\"exe\":\"strata.exe\",\"args\":[]}",
        )
        .unwrap();
        assert!(strata_in_folder(&dir).iter().any(|f| f.model == "IQ2_XS"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_files_may_start_with_a_bom() {
        let v = parse_json_file("\u{feff}{\"installs\":[\"C:\\\\S\"]}");
        assert_eq!(v["installs"][0], "C:\\S");
        assert!(parse_json_file("not json").is_null());
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
        let no_gpu = StrataHardware {
            ram_gib: Some(16),
            vram_gib: None,
        };
        assert_eq!(
            no_gpu.describe_short(false),
            "This PC: 16 GB RAM, no NVIDIA graphics card"
        );
        assert_eq!(
            no_gpu.describe_short(true),
            "Этот ПК: 16 ГБ RAM, нет видеокарты NVIDIA"
        );
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
