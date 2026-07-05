//! Shared model-download helpers for the on-demand engine installers
//! (`tts_install`, `ocr_install`, `diar_install`). Each installer used to carry a
//! byte-identical private copy of these; this module is the single home so a fix
//! (a curl flag, the SHA-verify guarantee, the pinned System32 `tar`) lands once.
//!
//! Flow they compose: `curl.exe` download (retries the GitHub CDN's resets) →
//! SHA-256 pin verify (never trust a network artifact) → optional `bsdtar` extract,
//! into `%APPDATA%\suflyor\…`. All blocking — callers run them on a worker thread.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Download `url` → `dest` via `curl.exe`. RETRIES transient failures — the GitHub
/// release CDN resets open-ended GETs (`curl: (35) Connection was reset`), so
/// `--retry … --retry-all-errors` is essential. Follows redirects, fails on HTTP
/// error, no console window. Blocking. Removes a partial file on failure.
pub(crate) fn curl_download(url: &str, dest: &Path) -> Result<()> {
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
pub(crate) fn extract_tar_bz2(tarball: &Path, dest_dir: &Path) -> Result<()> {
    let status = no_window(&mut Command::new(system_bsdtar()))
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
pub(crate) fn system_bsdtar() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("tar.exe"))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("tar.exe"))
}

/// Verify a file's SHA-256 against `expected_hex`; delete + error on mismatch.
pub(crate) fn verify_sha256(path: &Path, expected_hex: &str, label: &str) -> Result<()> {
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

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(windows)]
pub(crate) fn no_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(windows))]
pub(crate) fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_and_padded() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }
}
