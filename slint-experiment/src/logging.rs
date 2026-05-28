//! File logging for the overlay-host binary.
//!
//! Release builds are compiled with `windows_subsystem = "windows"`
//! (no console window — see the crate-attribute in `bin/overlay_host.rs`),
//! so `eprintln!` output goes nowhere. To keep diagnostics available for
//! testers, `init()` opens a log file next to `config.json`
//! (`%APPDATA%\overlay-mvp\overlay-host.log`), installs a panic hook that
//! records crashes there, and exposes `line()` to append timestamped
//! entries. Entries are ALSO mirrored to stderr, so debug builds (which
//! keep their console) still print to the terminal as before.
//!
//! NEVER log secrets (API keys / bearer tokens). Call sites log presence
//! booleans, not values.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

/// `%APPDATA%\overlay-mvp\overlay-host.log` — same dir as `config.json`.
fn log_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("overlay-mvp").join("overlay-host.log"))
}

/// Open the log file (append), rotate it if large, and install a panic
/// hook. Idempotent-ish: safe to call once at startup. Silent on any
/// filesystem error — logging must never take the app down.
pub fn init() {
    let Some(path) = log_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Rotate when the log passes ~2 MiB so it can't grow without bound.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 2 * 1024 * 1024 {
            let _ = std::fs::rename(&path, path.with_extension("log.old"));
        }
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = LOG_FILE.set(Mutex::new(file));
    }

    // Crashes are the single most valuable thing to capture for a tester
    // build with no console. Chain to the previous hook so debug-build
    // behaviour (backtrace to stderr) is preserved.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        line(&format!("PANIC: {info}"));
        prev(info);
    }));

    line(&format!(
        "=== suflyor overlay-host v{} start ===",
        env!("CARGO_PKG_VERSION")
    ));
}

/// Append one timestamped line to the log file (if open) and mirror it to
/// stderr. The timestamp is UTC `HH:MM:SS` — enough to correlate events
/// without pulling in a date/time crate.
pub fn line(msg: &str) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stamp = format!(
        "{:02}:{:02}:{:02}Z",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    );
    if let Some(lock) = LOG_FILE.get() {
        if let Ok(mut f) = lock.lock() {
            let _ = writeln!(f, "[{stamp}] {msg}");
            let _ = f.flush();
        }
    }
    // Visible in debug builds (which keep a console); a no-op in release.
    eprintln!("{msg}");
}
