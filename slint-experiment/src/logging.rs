//! File logging for the overlay-host binary.
//!
//! Release builds are compiled with `windows_subsystem = "windows"` (no
//! console window — see the crate-attribute in `bin/overlay_host.rs`), so
//! `eprintln!` would normally go nowhere. To keep diagnostics available for
//! testers, `init()` opens a log file next to `config.json`
//! (`%APPDATA%\overlay-mvp\overlay-host.log`), installs a panic hook that
//! records crashes there, and — IN RELEASE — redirects the process stderr to
//! that file via `SetStdHandle`, so EVERY `eprintln!` across the binary (~170
//! of them) is captured (v0.9.3 — before this, the log held only `line()`
//! entries + panics, leaving testers with near-empty logs). Debug builds keep
//! their console (stderr untouched); `line()` writes its timestamped entries to
//! the file directly there.
//!
//! NEVER log secrets (API keys / bearer tokens). Call sites log presence
//! booleans, not values.

use std::fs::OpenOptions;
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
        // RELEASE is built `windows_subsystem="windows"` → NO console, so the
        // process stderr is invalid and EVERY `eprintln!` (~170 of them across
        // the binary) is silently discarded — testers ended up with an almost
        // empty log. Redirect stderr to the log file so all of them land in
        // `overlay-host.log`. DEBUG keeps its console untouched (the terminal
        // still prints), and `line()` writes its own entries via `LOG_FILE`.
        // The file is kept alive in `LOG_FILE` below, so the handle backing
        // stderr stays valid for the whole process; a failure here is non-fatal
        // (logging must never take the app down).
        #[cfg(all(windows, not(debug_assertions)))]
        {
            use std::os::windows::io::AsRawHandle;
            use windows::Win32::Foundation::HANDLE;
            use windows::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE};
            unsafe {
                let _ = SetStdHandle(STD_ERROR_HANDLE, HANDLE(file.as_raw_handle()));
            }
        }
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
    // DEBUG: stderr goes to the console, NOT the file — so write the entry to
    // `LOG_FILE` directly here. RELEASE: stderr is redirected to the log file
    // (see `init`), so the `eprintln!` below already lands there; a direct
    // write would duplicate every `line()` entry.
    #[cfg(debug_assertions)]
    if let Some(lock) = LOG_FILE.get() {
        if let Ok(mut f) = lock.lock() {
            use std::io::Write;
            let _ = writeln!(f, "[{stamp}] {msg}");
            let _ = f.flush();
        }
    }
    // Console in debug; the redirected log file in release. Timestamped so a
    // raw `eprintln!` elsewhere and a `line()` entry interleave readably.
    eprintln!("[{stamp}] {msg}");
}
