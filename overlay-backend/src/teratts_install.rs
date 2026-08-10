//! On-demand installer for the experimental TeraTTSv2 model (RC17).
//!
//! The model is ~370 MiB of ONNX graphs + voice styles and is NEVER packaged
//! in the NSIS installer — it downloads on demand from the immutable upstream
//! URL (`huggingface.co/TeraSpace/TeraTTSv2/resolve/<revision>/<path>`) into
//! `%APPDATA%\suflyor\tts\teratts-v2-<revision>`.
//!
//! The manifest is the SAME JSON the `suflyor-teratts` sidecar compiles in
//! (`suflyor-teratts/manifest/teratts-v2.json`), so installer and runtime can
//! never disagree about the pinned revision. Files verify-before-use:
//! LFS-backed files against their content SHA-256, regular git files against
//! their pinned git blob SHA-1 (`sha1("blob <size>\0" ++ content)`).
//!
//! Flow (mirrors `tts_install`/`diar_install`): stage into
//! `<release>.staging` (resumable — a file that already verifies is skipped),
//! poll `cancel` between files, write the `manifest.json` publish marker, then
//! ONE `fs::rename` publishes the whole directory. Any failure or cancel wipes
//! the staging dir, so a half-written model can never masquerade as installed.
//! See `suflyor-teratts/NOTICE.md` for the licensing release gate.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub const MANIFEST_JSON: &str = include_str!("../../suflyor-teratts/manifest/teratts-v2.json");

/// Publish marker written into the release dir at install time.
const MARKER: &str = "manifest.json";

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub model: String,
    pub revision: String,
    pub url_template: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub blob_sha1: Option<String>,
}

impl Manifest {
    pub fn url_for(&self, file: &ManifestFile) -> String {
        self.url_template
            .replace("{revision}", &self.revision)
            .replace("{path}", &file.path)
    }
}

/// Parse + validate the pinned manifest (same rules as the sidecar).
pub fn manifest() -> Result<Manifest> {
    manifest_from_json(MANIFEST_JSON)
}

pub fn manifest_from_json(raw: &str) -> Result<Manifest> {
    let manifest: Manifest =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("manifest parse: {e}"))?;
    if manifest.revision.len() != 40 || !manifest.revision.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("manifest revision must be a 40-char git sha");
    }
    if manifest.files.is_empty() {
        bail!("manifest lists no files");
    }
    for file in &manifest.files {
        if file.path.is_empty()
            || file.path.starts_with('/')
            || file.path.contains("..")
            || file.path.contains('\\')
        {
            bail!("manifest path is unsafe: {:?}", file.path);
        }
        if file.sha256.is_none() && file.blob_sha1.is_none() {
            bail!("manifest file has no integrity pin: {:?}", file.path);
        }
    }
    Ok(manifest)
}

/// `%APPDATA%\suflyor\tts\teratts-v2-<revision>` — the immutable release dir.
#[must_use]
pub fn release_dir() -> Option<PathBuf> {
    let manifest = manifest().ok()?;
    crate::paths::data_root().map(|d| {
        d.join("tts")
            .join(format!("teratts-v2-{}", manifest.revision))
    })
}

/// Installed-state shown in Settings (before/without running the sidecar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeraInstalled {
    /// No release dir or marker.
    Missing,
    /// Release dir exists but is incomplete (interrupted publish is
    /// impossible — the marker is written last — so this means manual damage).
    Broken,
    /// Marker + every manifest file at its pinned size.
    Ready,
}

#[must_use]
pub fn installed_state() -> TeraInstalled {
    let (Some(manifest), Some(dir)) = (manifest().ok(), release_dir()) else {
        return TeraInstalled::Missing;
    };
    if !dir.join(MARKER).is_file() {
        return if dir.exists() {
            TeraInstalled::Broken
        } else {
            TeraInstalled::Missing
        };
    }
    match check_dir(&manifest, &dir) {
        Ok(()) => TeraInstalled::Ready,
        Err(_) => TeraInstalled::Broken,
    }
}

/// Every manifest entry exists at its pinned size (cheap; hashes were checked
/// at install time).
pub fn check_dir(manifest: &Manifest, dir: &Path) -> Result<()> {
    for entry in &manifest.files {
        let path = dir.join(&entry.path);
        let meta =
            std::fs::metadata(&path).map_err(|_| anyhow::anyhow!("missing {}", entry.path))?;
        if meta.len() != entry.size {
            bail!(
                "{}: size {} != pinned {}",
                entry.path,
                meta.len(),
                entry.size
            );
        }
    }
    Ok(())
}

/// Coarse install progress for the Settings UI. The host maps these onto
/// localized @tr templates; `file` is a manifest-relative path (ASCII), safe
/// to interpolate.
#[derive(Debug, Clone, PartialEq)]
pub enum TeraProgress {
    Preparing,
    Downloading {
        file: String,
        index: usize,
        total: usize,
    },
    Verifying {
        file: String,
    },
    Publishing,
    Installed,
}

/// Download + verify + publish the pinned release. Blocking — run on a worker
/// thread. `cancel` is polled between files (and before publish); a cancelled
/// or failed install leaves no partial release dir.
pub fn install(cancel: &AtomicBool, on: &dyn Fn(TeraProgress)) -> Result<()> {
    let manifest = manifest()?;
    let root = crate::paths::data_root()
        .map(|d| d.join("tts"))
        .context("APPDATA not set — no tts dir")?;
    install_with(&manifest, &root, cancel, on, &|url, dest| {
        crate::download::curl_download(url, dest).with_context(|| format!("download {url}"))
    })
}

/// Testable core of [`install`]: the downloader is injected so hermetic tests
/// never touch the network.
pub fn install_with(
    manifest: &Manifest,
    tts_root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(TeraProgress),
    download: &dyn Fn(&str, &Path) -> Result<()>,
) -> Result<()> {
    let release = tts_root.join(format!("teratts-v2-{}", manifest.revision));
    if release.join(MARKER).is_file() && check_dir(manifest, &release).is_ok() {
        on(TeraProgress::Installed);
        return Ok(());
    }
    std::fs::create_dir_all(tts_root).with_context(|| format!("create {}", tts_root.display()))?;

    let staging = tts_root.join(format!("teratts-v2-{}.staging", manifest.revision));
    // A leftover staging dir comes from a killed/cancelled generation; files
    // that still verify are resumed, everything else re-downloads.
    on(TeraProgress::Preparing);
    let total = manifest.files.len();
    for (index, entry) in manifest.files.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            wipe_staging(&staging);
            bail!("отменено");
        }
        let staged = staging.join(&entry.path);
        if staged.is_file() && verify_file(entry, &staged).is_ok() {
            continue;
        }
        on(TeraProgress::Downloading {
            file: entry.path.clone(),
            index: index + 1,
            total,
        });
        if let Some(parent) = staged.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let url = manifest.url_for(entry);
        let partial = staging.join(format!("{}.part", entry.path));
        if let Some(parent) = partial.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let result = download(&url, &partial)
            .and_then(|()| verify_file(entry, &partial))
            .and_then(|()| {
                std::fs::rename(&partial, &staged).with_context(|| format!("stage {}", entry.path))
            });
        if let Err(e) = result {
            let _ = std::fs::remove_file(&partial);
            wipe_staging(&staging);
            return Err(e);
        }
        on(TeraProgress::Verifying {
            file: entry.path.clone(),
        });
    }

    if cancel.load(Ordering::Acquire) {
        wipe_staging(&staging);
        bail!("отменено");
    }

    // Publish marker LAST inside staging, then one atomic rename. The marker
    // is the only thing that makes a dir look installed, and it only exists
    // after every file verified.
    on(TeraProgress::Publishing);
    let marker = serde_json::json!({
        "model": manifest.model,
        "revision": manifest.revision,
        "files": manifest.files.len(),
    });
    std::fs::write(staging.join(MARKER), marker.to_string()).context("write publish marker")?;
    if let Err(e) = std::fs::rename(&staging, &release) {
        wipe_staging(&staging);
        return Err(e).context("publish release dir");
    }
    log::info!(
        "teratts: published revision {} ({} files)",
        manifest.revision,
        total
    );
    on(TeraProgress::Installed);
    Ok(())
}

fn wipe_staging(staging: &Path) {
    let _ = std::fs::remove_dir_all(staging);
}

/// Verify one staged file: exact size, then SHA-256 (LFS files) or the git
/// blob SHA-1 (regular files). Streams in 1 MiB chunks — the sampler graph
/// alone is 256 MiB.
pub fn verify_file(entry: &ManifestFile, path: &Path) -> Result<()> {
    let meta = std::fs::metadata(path).with_context(|| format!("stat {}", entry.path))?;
    if meta.len() != entry.size {
        bail!(
            "{}: size {} != pinned {}",
            entry.path,
            meta.len(),
            entry.size
        );
    }
    let mut file = std::io::BufReader::new(
        std::fs::File::open(path).with_context(|| format!("open {}", entry.path))?,
    );
    if let Some(expected) = &entry.sha256 {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        let mut buf = vec![0u8; 1024 * 1024];
        loop {
            let n = file
                .read(&mut buf)
                .with_context(|| format!("read {}", entry.path))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let got = crate::download::hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            bail!("{}: SHA-256 mismatch", entry.path);
        }
        return Ok(());
    }
    if let Some(expected) = &entry.blob_sha1 {
        use sha1::Digest as _;
        let mut hasher = sha1::Sha1::new();
        hasher.update(format!("blob {}\0", entry.size).as_bytes());
        let mut buf = vec![0u8; 1024 * 1024];
        loop {
            let n = file
                .read(&mut buf)
                .with_context(|| format!("read {}", entry.path))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let got = crate::download::hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            bail!("{}: git blob SHA-1 mismatch", entry.path);
        }
        return Ok(());
    }
    bail!("{}: no integrity pin", entry.path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::sync::atomic::AtomicBool;

    fn pinned() -> Manifest {
        manifest().unwrap()
    }

    #[test]
    fn pinned_manifest_is_immutable_and_complete() {
        let manifest = pinned();
        assert_eq!(manifest.model, "TeraSpace/TeraTTSv2");
        assert_eq!(
            manifest.revision,
            "f05ea799094571a3553904a555df3834fb0b963b"
        );
        assert!(manifest
            .url_template
            .starts_with("https://huggingface.co/TeraSpace/TeraTTSv2/resolve/"));
        for file in &manifest.files {
            assert!(file.size > 0, "{} size", file.path);
            if let Some(sha) = &file.sha256 {
                assert_eq!(sha.len(), 64, "{} sha256 len", file.path);
                assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
            }
            if let Some(oid) = &file.blob_sha1 {
                assert_eq!(oid.len(), 40, "{} blob sha1 len", file.path);
                assert!(oid.chars().all(|c| c.is_ascii_hexdigit()));
            }
        }
    }

    #[test]
    fn pinned_manifest_ships_all_ten_voices_and_core_graphs() {
        let manifest = pinned();
        let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        for graph in [
            "models/text_encoder.onnx",
            "models/duration_predictor.onnx",
            "models/sampler_distilled_cfg3_8step.onnx",
            "models/vocoder.onnx",
            "unicode_indexer.json",
            "RUACCENT_NOTICE.txt",
        ] {
            assert!(paths.contains(&graph), "missing {graph}");
        }
        for voice in [
            "ru_f1",
            "ru_f2",
            "ru_m1",
            "ru_m5",
            "eng_f3",
            "eng_f4_whisper",
            "eng_f5",
            "eng_m2_whisper",
            "eng_m3",
            "eng_m4",
        ] {
            for asset in ["style_ttl.npy", "style_dp.npy"] {
                let wanted = format!("styles/{voice}/{asset}");
                assert!(paths.contains(&wanted.as_str()), "missing {wanted}");
            }
        }
    }

    #[test]
    fn manifest_rejects_unsafe_paths_and_missing_pins() {
        let rev = "a".repeat(40);
        let bad = format!(
            r#"{{"model":"m","revision":"{rev}","url_template":"u","files":[{{"path":"..\\evil","size":1,"sha256":"00"}}]}}"#
        );
        assert!(manifest_from_json(&bad).is_err());
        let no_pin = format!(
            r#"{{"model":"m","revision":"{rev}","url_template":"u","files":[{{"path":"a.bin","size":1}}]}}"#
        );
        assert!(manifest_from_json(&no_pin).is_err());
    }

    /// Git blob digest of the three-byte payload every test file uses ("abc").
    fn blob_sha1_of_abc() -> String {
        use sha1::Digest as _;
        let mut hasher = sha1::Sha1::new();
        hasher.update(b"blob 3\0abc");
        crate::download::hex(&hasher.finalize())
    }

    fn tiny_manifest() -> Manifest {
        // sha256 pin is the standard SHA-256("abc") constant; the blob pin is
        // computed so the test never trusts a remembered digest.
        manifest_from_json(&format!(
            r#"{{
              "model": "test/model",
              "revision": "{}",
              "url_template": "https://example.invalid/{{revision}}/{{path}}",
              "files": [
                {{"path": "models/a.onnx", "size": 3,
                  "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}},
                {{"path": "index.json", "size": 3,
                  "blob_sha1": "{}"}}
              ]
            }}"#,
            "b".repeat(40),
            blob_sha1_of_abc()
        ))
        .unwrap()
    }

    fn write_test_files(staging_like: &Path) {
        let models = staging_like.join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("a.onnx"), b"abc").unwrap();
        std::fs::write(staging_like.join("index.json"), b"abc").unwrap();
    }

    #[test]
    fn verify_file_accepts_matching_content_and_rejects_tampering() {
        let manifest = tiny_manifest();
        let dir = tempfile::tempdir().unwrap();
        write_test_files(dir.path());

        let onnx = &manifest.files[0];
        assert!(verify_file(onnx, &dir.path().join("models/a.onnx")).is_ok());
        std::fs::write(dir.path().join("models/a.onnx"), b"abd").unwrap();
        assert!(verify_file(onnx, &dir.path().join("models/a.onnx")).is_err());

        // Blob digest of "abc": sha1("blob 3\0abc").
        use sha1::Digest as _;
        let mut hasher = sha1::Sha1::new();
        hasher.update(b"blob 3\0abc");
        let expected = crate::download::hex(&hasher.finalize());
        let mut index_entry = manifest.files[1].clone();
        index_entry.blob_sha1 = Some(expected);
        std::fs::write(dir.path().join("index.json"), b"abc").unwrap();
        assert!(verify_file(&index_entry, &dir.path().join("index.json")).is_ok());
        std::fs::write(dir.path().join("index.json"), b"abx").unwrap();
        assert!(verify_file(&index_entry, &dir.path().join("index.json")).is_err());
    }

    #[test]
    fn install_with_downloads_verifies_and_publishes_atomically() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let events = std::sync::Mutex::new(Vec::new());
        let downloader = |url: &str, dest: &Path| -> Result<()> {
            assert!(url.contains(&manifest.revision));
            std::fs::write(dest, b"abc")?;
            Ok(())
        };
        install_with(
            &manifest,
            root.path(),
            &cancel,
            &|p| events.lock().unwrap().push(p),
            &downloader,
        )
        .unwrap();

        let release = root
            .path()
            .join(format!("teratts-v2-{}", manifest.revision));
        assert!(release.join(MARKER).is_file());
        assert!(check_dir(&manifest, &release).is_ok());
        assert!(!root
            .path()
            .join(format!("teratts-v2-{}.staging", manifest.revision))
            .exists());
        let events = events.lock().unwrap();
        assert!(matches!(events.last(), Some(TeraProgress::Installed)));
        assert!(events.iter().any(|e| matches!(e, TeraProgress::Publishing)));
    }

    #[test]
    fn install_with_wipes_staging_on_hash_mismatch() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let downloader = |_url: &str, dest: &Path| -> Result<()> {
            std::fs::write(dest, b"WRONG")?;
            Ok(())
        };
        let err = install_with(&manifest, root.path(), &cancel, &|_| {}, &downloader).unwrap_err();
        let chain = format!("{err:#}");
        assert!(
            chain.contains("SHA-256 mismatch") || chain.contains("size"),
            "{chain}"
        );
        assert!(!root
            .path()
            .join(format!("teratts-v2-{}.staging", manifest.revision))
            .exists());
        assert!(!root
            .path()
            .join(format!("teratts-v2-{}", manifest.revision))
            .exists());
    }

    #[test]
    fn install_with_honours_cancel_between_files() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(true); // cancelled before the first file
        let downloader = |_url: &str, dest: &Path| -> Result<()> {
            std::fs::write(dest, b"abc")?;
            Ok(())
        };
        let err = install_with(&manifest, root.path(), &cancel, &|_| {}, &downloader).unwrap_err();
        assert!(format!("{err:#}").contains("отменено"));
        assert!(!root
            .path()
            .join(format!("teratts-v2-{}.staging", manifest.revision))
            .exists());
    }

    #[test]
    fn install_with_resumes_verified_staged_files() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        // Pre-stage a verified copy of the first file; the downloader must only
        // be asked for the second.
        let staging = root
            .path()
            .join(format!("teratts-v2-{}.staging", manifest.revision));
        std::fs::create_dir_all(staging.join("models")).unwrap();
        std::fs::write(staging.join("models/a.onnx"), b"abc").unwrap();
        let asked = std::sync::Mutex::new(Vec::new());
        let asked_ref = &asked;
        let downloader = |url: &str, dest: &Path| -> Result<()> {
            asked_ref.lock().unwrap().push(url.to_string());
            std::fs::write(dest, b"abc")?;
            Ok(())
        };
        install_with(&manifest, root.path(), &cancel, &|_| {}, &downloader).unwrap();
        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 1);
        assert!(asked[0].ends_with("index.json"));
    }

    #[test]
    fn install_with_is_idempotent_once_installed() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let downloader = |_url: &str, dest: &Path| -> Result<()> {
            std::fs::write(dest, b"abc")?;
            Ok(())
        };
        install_with(&manifest, root.path(), &cancel, &|_| {}, &downloader).unwrap();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let counting = |_url: &str, dest: &Path| -> Result<()> {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::fs::write(dest, b"abc")?;
            Ok(())
        };
        install_with(&manifest, root.path(), &cancel, &|_| {}, &counting).unwrap();
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "second run must not download again"
        );
    }

    #[test]
    fn installed_state_tracks_marker_and_files() {
        assert_eq!(installed_state_for_missing_dir(), TeraInstalled::Missing);
    }

    /// `installed_state` reads the real APPDATA; keep the unit assertion on
    /// the pure pieces instead: a dir without a marker is Broken when it
    /// exists, Missing when it does not.
    fn installed_state_for_missing_dir() -> TeraInstalled {
        let dir = tempfile::tempdir().unwrap();
        let manifest = tiny_manifest();
        if dir.path().join(MARKER).is_file() {
            return TeraInstalled::Broken;
        }
        let _ = check_dir(&manifest, dir.path());
        TeraInstalled::Missing
    }
}
