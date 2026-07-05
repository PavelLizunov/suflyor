//! Download-on-demand installer for the OCR engine (Tesseract).
//!
//! Like [`crate::tts_install`] (voices) and [`crate::local_ai`] (models), the
//! ~53 MB engine is NOT bundled in the app installer — the user installs it with
//! a button in Settings → Vision. Flow: `curl.exe` download (with retries) →
//! SHA-256 verify (pinned) → extract with the system `bsdtar`
//! (`%SystemRoot%\System32\tar.exe`; libarchive handles `.tar.bz2`) into
//! `%APPDATA%\suflyor\` — the archive root is `tesseract/`, which is exactly
//! where [`crate::ocr::tesseract_root`] looks.
//!
//! The bundle is the UB-Mannheim Tesseract 5.4.0 runtime (Apache-2.0) + RU/EN
//! fast tessdata, hosted as a stable GitHub release asset. SHA-256 pinning makes
//! the download verify-before-use (the security review's requirement).
//!
//! (Download / verify / extract helpers are shared in [`crate::download`].)

use crate::download::{curl_download, extract_tar_bz2, verify_sha256};
use anyhow::{bail, Context, Result};
use std::path::Path;

/// Hosted OCR-engine bundle (`tesseract/` runtime + RU/EN fast tessdata).
const BUNDLE_URL: &str =
    "https://github.com/PavelLizunov/suflyor/releases/download/ocr-assets/tesseract-ocr-suflyor.tar.bz2";
/// SHA-256 of the bundle, verified before extraction.
const BUNDLE_SHA256: &str = "d753736e47d147cc7c42faf8948b33431d28524fd36f542aac11a25e0f07ce55";

/// Coarse progress for the Settings UI.
pub enum OcrProgress {
    AlreadyInstalled,
    Downloading,
    Verifying,
    Unpacking,
    Installed,
}

/// True when the OCR engine is installed (delegates to the resolver the OCR
/// path itself uses, so "installed" means "Shift+Alt+2 will use Tesseract").
#[must_use]
pub fn is_installed() -> bool {
    crate::ocr::is_available()
}

/// Download + verify + extract the OCR engine. Blocking — the caller runs it on
/// a worker thread (mirrors the voice + local-AI installers).
pub fn install(on: &dyn Fn(OcrProgress)) -> Result<()> {
    if is_installed() {
        on(OcrProgress::AlreadyInstalled);
        return Ok(());
    }
    let root = crate::paths::data_root().context("APPDATA not set — no data dir")?;
    std::fs::create_dir_all(&root).with_context(|| format!("create {}", root.display()))?;
    let dest = root.join("tesseract");

    on(OcrProgress::Downloading);
    let tarball = root.join("tesseract-ocr.download.tar.bz2");
    let _ = std::fs::remove_file(&tarball);
    curl_download(BUNDLE_URL, &tarball).context("download OCR engine")?;

    on(OcrProgress::Verifying);
    verify_sha256(&tarball, BUNDLE_SHA256, "OCR")?;

    on(OcrProgress::Unpacking);
    if let Err(e) = extract_tar_bz2(&tarball, &root) {
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&tarball);
        return Err(e).context("extract OCR engine");
    }
    let _ = std::fs::remove_file(&tarball);

    // Confirm the extraction actually produced a usable engine AT THE DEST.
    // We check `dest` directly rather than `is_installed()` because the latter
    // (via ocr::tesseract_root) prefers an `<exe_dir>\tesseract` copy — so it
    // could (a) miss a half-extracted %APPDATA% dir if some unrelated exe-dir
    // copy exists, or (b) pass even though THIS bundle's archive root wasn't
    // `tesseract/` and the files landed loose in %APPDATA%. Checking the dir we
    // wrote to keeps the confirm + cleanup honest regardless of the resolver's
    // search order or a future re-pin's layout.
    if !dest_has_engine(&dest) {
        let _ = std::fs::remove_dir_all(&dest);
        bail!("движок распознавания установлен не полностью");
    }
    on(OcrProgress::Installed);
    Ok(())
}

/// True when `dest` (the `%APPDATA%\…\tesseract` extraction target) actually
/// holds a usable engine: the Tesseract binary + at least the Russian tessdata.
/// Checked directly on the dir we extracted into, NOT via the `<exe_dir>`-first
/// resolver, so the post-install confirm + partial-install cleanup reason about
/// the exact bytes this install produced.
fn dest_has_engine(dest: &Path) -> bool {
    dest.join("tesseract.exe").is_file() && dest.join("tessdata").join("rus.traineddata").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_pin_is_valid() {
        assert!(BUNDLE_URL.starts_with("https://github.com/PavelLizunov/suflyor/"));
        assert!(BUNDLE_URL.ends_with(".tar.bz2"));
        assert_eq!(BUNDLE_SHA256.len(), 64);
        assert!(BUNDLE_SHA256.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn dest_has_engine_requires_binary_and_tessdata() -> std::io::Result<()> {
        let tmp = std::env::temp_dir().join(format!("ocr_dest_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("tessdata"))?;
        // Empty dir / partial extraction → not a usable engine.
        assert!(
            !dest_has_engine(&tmp),
            "empty dest must not count as installed"
        );
        std::fs::write(tmp.join("tesseract.exe"), b"x")?;
        assert!(
            !dest_has_engine(&tmp),
            "binary alone (no tessdata) is incomplete"
        );
        std::fs::write(tmp.join("tessdata").join("rus.traineddata"), b"x")?;
        assert!(dest_has_engine(&tmp), "binary + RU tessdata → usable");
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
