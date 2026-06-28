//! Download-on-demand installer for the read-aloud neural voices.
//!
//! The voices are large (~63 MB each) so they are NOT bundled in the app
//! installer; the user installs them with a button in Settings → «Озвучка»,
//! exactly like the local-AI model installer ([`crate::local_ai`]). The flow
//! mirrors that module: `curl.exe` download → SHA-256 verify (pinned below) →
//! extract with the system `bsdtar` (`%SystemRoot%\System32\tar.exe`, Win10
//! 1803+; libarchive decompresses bz2 in-process) into `%APPDATA%\suflyor\tts\`.
//! We pin the System32 path so PATH order can't decide which `tar` runs.
//!
//! Sources are the sherpa-onnx `tts-models` GitHub release (the same Piper
//! voices the app already loads). SHA-256 pins make the download verify-before-
//! use (the security review's requirement — never trust a network artifact).

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

/// A downloadable voice pack.
pub struct VoicePack {
    /// On-disk directory name (also the sherpa archive stem) — stable id.
    pub id: &'static str,
    /// Friendly label for progress messages.
    pub label: &'static str,
    /// `.tar.bz2` download URL (sherpa-onnx tts-models release).
    pub url: &'static str,
    /// SHA-256 of the `.tar.bz2`, verified before extraction.
    pub sha256: &'static str,
}

/// The voices offered by the installer button. Both are Piper RU (the app's
/// existing engine); Irina = female, Ruslan = male.
pub const VOICE_PACKS: &[VoicePack] = &[
    VoicePack {
        id: "vits-piper-ru_RU-irina-medium",
        label: "Ирина (ж)",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-ru_RU-irina-medium.tar.bz2",
        sha256: "1fc0f54e5e084fe287c07909f2f6e0ba6d857864cf800e3ab80286a4e8233008",
    },
    VoicePack {
        id: "vits-piper-ru_RU-ruslan-medium",
        label: "Руслан (м)",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-ru_RU-ruslan-medium.tar.bz2",
        sha256: "0690b1cad01f86e8db9ba988af24898bdc1af774e23cb2e46b9c730269b6fd83",
    },
];

/// Coarse progress for the Settings UI (no byte bar — the packs are small and
/// the steps are quick; a step label is enough and robust).
pub enum VoiceProgress {
    AlreadyInstalled(String),
    Downloading(String),
    Verifying(String),
    Unpacking(String),
    PackFailed(String),
    AllInstalled,
    PartiallyInstalled(String),
}

/// `%APPDATA%\suflyor\tts` — where voices install. Same dir the sidecar scans.
#[must_use]
pub fn tts_dir() -> Option<PathBuf> {
    crate::paths::data_root().map(|d| d.join("tts"))
}

/// True when a given pack is already installed (its dir has a model + tokens).
fn pack_installed(dir: &Path) -> bool {
    if !dir.join("tokens.txt").is_file() {
        return false;
    }
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("onnx"))
        })
        .unwrap_or(false)
}

/// True when at least one voice is installed (drives "Install" vs "Installed"
/// in the Settings panel).
#[must_use]
pub fn any_voice_installed() -> bool {
    let Some(root) = tts_dir() else {
        return false;
    };
    VOICE_PACKS.iter().any(|p| pack_installed(&root.join(p.id)))
}

/// Download + verify + extract every not-yet-installed voice pack. Blocking —
/// the caller runs it on a worker thread (mirrors `local_ai::install`). `cancel`
/// is polled between packs; `on` receives step messages for the UI.
pub fn install_voices(cancel: &AtomicBool, on: &dyn Fn(VoiceProgress)) -> Result<()> {
    let root = tts_dir().context("APPDATA not set — no voices dir")?;
    std::fs::create_dir_all(&root).with_context(|| format!("create {}", root.display()))?;

    // Install packs INDEPENDENTLY: one pack failing (a transient CDN reset on the
    // 2nd voice) must NOT discard a voice that already installed — otherwise the
    // UI reports total failure while a usable voice sits on disk (the tester's
    // exact case). Succeed if AT LEAST ONE voice ends up installed.
    let mut ok = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    for pack in VOICE_PACKS {
        if cancel.load(Ordering::Acquire) {
            bail!("отменено");
        }
        let dir = root.join(pack.id);
        if pack_installed(&dir) {
            on(VoiceProgress::AlreadyInstalled(pack.label.to_string()));
            ok += 1;
            continue;
        }
        match install_one(pack, &root, &dir, on) {
            Ok(()) => ok += 1,
            Err(e) => {
                log::warn!("voice pack '{}' failed: {e:#}", pack.id);
                failed.push(pack.label);
                on(VoiceProgress::PackFailed(pack.label.to_string()));
            }
        }
    }

    if ok == 0 {
        bail!("не удалось установить ни одного голоса");
    }
    if failed.is_empty() {
        on(VoiceProgress::AllInstalled);
    } else {
        on(VoiceProgress::PartiallyInstalled(failed.join(", ")));
    }
    Ok(())
}

/// Download + verify + extract ONE pack into its dir. On any failure the partial
/// dir is wiped so a half-written pack can't masquerade as installed.
fn install_one(
    pack: &VoicePack,
    root: &Path,
    dir: &Path,
    on: &dyn Fn(VoiceProgress),
) -> Result<()> {
    on(VoiceProgress::Downloading(pack.label.to_string()));
    let tarball = root.join(format!("{}.download.tar.bz2", pack.id));
    let _ = std::fs::remove_file(&tarball);
    curl_download(pack.url, &tarball).with_context(|| format!("download {}", pack.label))?;

    on(VoiceProgress::Verifying(pack.label.to_string()));
    verify_sha256(&tarball, pack.sha256, pack.label)?;

    on(VoiceProgress::Unpacking(pack.label.to_string()));
    if let Err(e) = extract_tar_bz2(&tarball, root) {
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(&tarball);
        return Err(e).with_context(|| format!("extract {}", pack.label));
    }
    let _ = std::fs::remove_file(&tarball);
    if !pack_installed(dir) {
        let _ = std::fs::remove_dir_all(dir);
        bail!("{} установлен не полностью", pack.label);
    }
    Ok(())
}

/// Download `url` → `dest` via `curl.exe`. RETRIES transient failures — the
/// GitHub release CDN resets open-ended GETs (`curl: (35) Connection was reset`),
/// so `--retry … --retry-all-errors` is essential (the same resilience
/// `local_ai`'s downloader relies on). Follows redirects, fails on HTTP error,
/// no console window. Blocking.
fn curl_download(url: &str, dest: &Path) -> Result<()> {
    let status = no_window(Command::new("curl.exe").args([
        "-L",
        "--fail",
        "--silent",
        "--show-error",
        "--retry",
        "5",
        "--retry-delay",
        "2",
        "--retry-all-errors",
        "--connect-timeout",
        "30",
        "-o",
    ]))
    .arg(dest)
    .arg(url)
    .status()
    .context("spawn curl.exe")?;
    if !status.success() {
        let _ = std::fs::remove_file(dest);
        bail!("curl exited with {status}");
    }
    Ok(())
}

/// Extract a `.tar.bz2` into `dest_dir` using the system `bsdtar`
/// (`%SystemRoot%\System32\tar.exe`, libarchive — decompresses bz2 in-process).
fn extract_tar_bz2(tarball: &Path, dest_dir: &Path) -> Result<()> {
    let tar = system_bsdtar();
    let status = no_window(&mut Command::new(tar))
        .arg("-xf")
        .arg(tarball)
        .arg("-C")
        .arg(dest_dir)
        .status()
        .context("spawn bsdtar")?;
    if !status.success() {
        bail!("bsdtar exited with {status}");
    }
    Ok(())
}

/// Full path to the libarchive `tar.exe` under System32 (Win10 1803+), so PATH
/// order can't substitute a different `tar`. Falls back to a bare `tar.exe`.
fn system_bsdtar() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("tar.exe"))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("tar.exe"))
}

/// Verify a file's SHA-256 against `expected_hex`; delete + error on mismatch.
fn verify_sha256(path: &Path, expected_hex: &str, label: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).with_context(|| format!("read {label} to verify"))?;
    let got = hex(&Sha256::digest(&bytes));
    if !got.eq_ignore_ascii_case(expected_hex) {
        let _ = std::fs::remove_file(path);
        bail!(
            "{label}: SHA-256 не совпал — файл повреждён или подменён, удалён; повторите установку"
        );
    }
    log::info!("{label} sha256 verified");
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(windows))]
fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_have_valid_pins() {
        // Each pack must have a sherpa tts-models URL ending in .tar.bz2 and a
        // 64-hex-char SHA-256 — a botched edit (wrong url/sha) would break the
        // verify-before-use guarantee silently.
        for p in VOICE_PACKS {
            assert!(p.url.starts_with("https://github.com/k2-fsa/sherpa-onnx/"));
            assert!(p.url.ends_with(".tar.bz2"));
            assert_eq!(p.sha256.len(), 64, "{} sha must be 64 hex chars", p.id);
            assert!(p.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn hex_is_lowercase_and_padded() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }
}
