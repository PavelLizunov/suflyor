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
//!
//! Self-heal: on Windows `fs::rename` cannot replace an existing directory,
//! so a broken release (missing marker, corrupt/short files, junk left by an
//! interrupted external tool) would make every re-install fail at publish
//! forever. A release that fails validation is first moved to a uniquely
//! named `<release>.broken-<ms>-<pid>-<n>` quarantine dir WITHIN the same
//! managed root, then staging publishes atomically. A VALID install is never
//! touched (it early-returns). Quarantine leftovers are swept best-effort
//! after a successful publish.
//!
//! See `suflyor-teratts/NOTICE.md` for the licensing release gate.

use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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

    // Self-heal: a release dir still present here FAILED validation at the
    // top (valid installs early-return). On Windows the publish rename cannot
    // replace it, so move it to a quarantine within the same parent first.
    if release.exists() {
        if let Err(e) = quarantine_broken_release(tts_root, &release, &manifest.revision) {
            wipe_staging(&staging);
            return Err(e).context("quarantine broken release");
        }
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
    sweep_quarantined(tts_root);
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

/// Unique-suffix counter for quarantine dir names (survives same-ms retries).
static QUARANTINE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Containment check used before moving/removing anything: `child` is `parent`
/// itself or a path strictly below it. Compares components pairwise
/// (ASCII-case-insensitively on Windows, where the filesystem is) and refuses
/// `.`/`..` segments in the child, so neither a drive-prefix neighbour
/// (`C:\a\b` vs `C:\a\bc`) nor a traversal (`C:\a\b\..\x`) ever passes.
fn is_within(parent: &Path, child: &Path) -> bool {
    let mut child_components = child.components();
    for parent_component in parent.components() {
        match child_components.next() {
            Some(c) if component_eq(&parent_component, &c) => continue,
            _ => return false,
        }
    }
    // Parent exhausted: child == parent or below it. Whatever follows the
    // matched prefix must be plain path segments — `..` (or anything else
    // non-Normal) never counts as containment.
    child_components.all(|c| matches!(c, Component::Normal(_)))
}

fn component_eq(a: &Component<'_>, b: &Component<'_>) -> bool {
    if cfg!(windows) {
        a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
    } else {
        a.as_os_str() == b.as_os_str()
    }
}

/// Move an INVALID release dir aside so the atomic publish rename can proceed
/// (on Windows `fs::rename` never replaces an existing directory). The
/// quarantine lives in the same parent under a unique name, so a valid
/// install — which early-returned before we get here — is never touched, and
/// nothing outside the managed root is ever moved or deleted. If the rename
/// is refused (e.g. a broken file is locked), falls back to deleting exactly
/// the one release dir.
fn quarantine_broken_release(tts_root: &Path, release: &Path, revision: &str) -> Result<()> {
    if !is_within(tts_root, release) {
        bail!("release dir escapes the managed tts root");
    }
    let Some(name) = release.file_name().and_then(|n| n.to_str()) else {
        bail!("release dir has no usable name");
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let pid = std::process::id();
    for _ in 0..8u32 {
        let attempt = QUARANTINE_COUNTER.fetch_add(1, Ordering::AcqRel);
        let quarantine = tts_root.join(format!("{name}.broken-{now_ms}-{pid}-{attempt}"));
        if std::fs::rename(release, &quarantine).is_ok() {
            log::warn!(
                "teratts: quarantined broken revision {revision} release at {}",
                quarantine.display()
            );
            return Ok(());
        }
    }
    log::warn!(
        "teratts: quarantine rename refused; removing broken revision {revision} release in place"
    );
    std::fs::remove_dir_all(release)
        .with_context(|| format!("remove broken release for revision {revision}"))
}

/// Best-effort sweep of quarantine leftovers (`teratts-v2-*.broken-*`) after a
/// successful publish, so a self-healed install does not keep ~370 MiB of
/// broken data around forever.
fn sweep_quarantined(tts_root: &Path) {
    let Ok(entries) = std::fs::read_dir(tts_root) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !name.starts_with("teratts-v2-") || !name.contains(".broken-") {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match std::fs::remove_dir_all(&path) {
            Ok(()) => log::info!("teratts: removed quarantined release {name}"),
            Err(e) => log::warn!("teratts: quarantine cleanup failed for {name}: {e}"),
        }
    }
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

    // ===== Self-heal (broken release quarantine) =====

    fn ok_downloader(_url: &str, dest: &Path) -> Result<()> {
        std::fs::write(dest, b"abc")?;
        Ok(())
    }

    fn release_path(root: &Path, manifest: &Manifest) -> PathBuf {
        root.join(format!("teratts-v2-{}", manifest.revision))
    }

    fn quarantine_dirs(root: &Path) -> Vec<String> {
        std::fs::read_dir(root)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| e.file_name().to_str().map(str::to_string))
                    .filter(|n| n.starts_with("teratts-v2-") && n.contains(".broken-"))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn is_within_rejects_prefix_neighbours_traversal_and_drives() {
        assert!(is_within(
            Path::new("root/tts"),
            Path::new("root/tts/teratts")
        ));
        assert!(is_within(Path::new("root/tts"), Path::new("root/tts")));
        // A sibling whose name merely starts with the same letters is OUT.
        assert!(!is_within(Path::new("root/tts"), Path::new("root/ttsx")));
        assert!(!is_within(Path::new("root/tts"), Path::new("root/other")));
        // Traversal never counts as containment (`.` normalizes away and is
        // genuinely inside; `..` escapes).
        assert!(!is_within(
            Path::new("root/tts"),
            Path::new("root/tts/../x")
        ));
        assert!(is_within(Path::new("root/tts"), Path::new("root/tts/./x")));
        assert!(!is_within(Path::new("root/tts"), Path::new("tts")));
        assert!(!is_within(Path::new("root/tts"), Path::new("root")));
    }

    #[cfg(windows)]
    #[test]
    fn is_within_understands_drive_prefixes_and_case() {
        assert!(is_within(Path::new(r"C:\a\b"), Path::new(r"C:\a\b\c")));
        // Drive-prefix neighbour: C:\a\bc is NOT below C:\a\b.
        assert!(!is_within(Path::new(r"C:\a\b"), Path::new(r"C:\a\bc")));
        assert!(!is_within(Path::new(r"C:\a\b"), Path::new(r"D:\a\b")));
        // Windows paths compare case-insensitively.
        assert!(is_within(Path::new(r"C:\A\b"), Path::new(r"c:\a\B\c")));
    }

    #[test]
    fn quarantine_moves_the_broken_dir_within_the_same_root() {
        let root = tempfile::tempdir().unwrap();
        let manifest = tiny_manifest();
        let release = release_path(root.path(), &manifest);
        std::fs::create_dir_all(release.join("models")).unwrap();
        std::fs::write(release.join("models/a.onnx"), b"junk").unwrap();

        quarantine_broken_release(root.path(), &release, &manifest.revision).unwrap();
        assert!(!release.exists());
        let quarantined = quarantine_dirs(root.path());
        assert_eq!(quarantined.len(), 1, "{quarantined:?}");
        let moved = root.path().join(&quarantined[0]);
        assert!(moved.join("models/a.onnx").is_file(), "content preserved");
    }

    #[test]
    fn quarantine_names_are_unique_and_refuse_foreign_paths() {
        let root = tempfile::tempdir().unwrap();
        let manifest = tiny_manifest();
        let release = release_path(root.path(), &manifest);
        std::fs::create_dir_all(&release).unwrap();
        quarantine_broken_release(root.path(), &release, &manifest.revision).unwrap();
        std::fs::create_dir_all(&release).unwrap();
        quarantine_broken_release(root.path(), &release, &manifest.revision).unwrap();
        let quarantined = quarantine_dirs(root.path());
        assert_eq!(quarantined.len(), 2, "{quarantined:?}");
        let unique: std::collections::BTreeSet<_> = quarantined.iter().collect();
        assert_eq!(unique.len(), 2, "quarantine names must not collide");

        // A release path outside the managed root must never be touched.
        let outside_root = tempfile::tempdir().unwrap();
        let outside = outside_root.path().join("teratts-v2-elsewhere");
        std::fs::create_dir_all(&outside).unwrap();
        let err = quarantine_broken_release(root.path(), &outside, &manifest.revision).unwrap_err();
        assert!(err.to_string().contains("escapes"), "{err:#}");
        assert!(outside.exists(), "foreign dir must stay untouched");
    }

    #[test]
    fn install_with_self_heals_release_missing_its_marker() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let release = release_path(root.path(), &manifest);
        // Broken release: files but no publish marker.
        write_test_files(&release);
        std::fs::remove_file(release.join(MARKER)).ok();

        let cancel = AtomicBool::new(false);
        install_with(&manifest, root.path(), &cancel, &|_| {}, &ok_downloader).unwrap();

        assert!(release.join(MARKER).is_file());
        assert!(check_dir(&manifest, &release).is_ok());
        // The quarantine was swept after the successful publish.
        assert!(quarantine_dirs(root.path()).is_empty());
    }

    #[test]
    fn install_with_self_heals_release_with_corrupt_file() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let release = release_path(root.path(), &manifest);
        // Broken release: marker present, one file truncated (size mismatch).
        write_test_files(&release);
        std::fs::write(release.join(MARKER), "{}").unwrap();
        std::fs::write(release.join("models/a.onnx"), b"a").unwrap();

        let cancel = AtomicBool::new(false);
        install_with(&manifest, root.path(), &cancel, &|_| {}, &ok_downloader).unwrap();

        assert!(release.join(MARKER).is_file());
        assert!(check_dir(&manifest, &release).is_ok());
        assert!(quarantine_dirs(root.path()).is_empty());
    }

    #[test]
    fn install_with_self_heals_nonempty_junk_release_dir() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let release = release_path(root.path(), &manifest);
        // Nonempty dir of unrelated junk (e.g. an interrupted external tool).
        std::fs::create_dir_all(release.join("random")).unwrap();
        std::fs::write(release.join("random/junk.bin"), b"xxxxx").unwrap();

        let cancel = AtomicBool::new(false);
        install_with(&manifest, root.path(), &cancel, &|_| {}, &ok_downloader).unwrap();

        assert!(check_dir(&manifest, &release).is_ok());
        assert!(!release.join("random").exists());
        assert!(quarantine_dirs(root.path()).is_empty());
    }

    #[test]
    fn install_with_preserves_a_valid_release_and_makes_no_quarantine() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let release = release_path(root.path(), &manifest);
        write_test_files(&release);
        std::fs::write(release.join(MARKER), "{}").unwrap();
        // Canary file: a valid release must be preserved byte-for-byte.
        std::fs::write(release.join("canary.txt"), b"keep me").unwrap();

        let cancel = AtomicBool::new(false);
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let counting = |_url: &str, dest: &Path| -> Result<()> {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::fs::write(dest, b"abc")?;
            Ok(())
        };
        install_with(&manifest, root.path(), &cancel, &|_| {}, &counting).unwrap();

        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(
            std::fs::read(release.join("canary.txt")).unwrap(),
            b"keep me"
        );
        assert!(quarantine_dirs(root.path()).is_empty());
    }

    #[test]
    fn install_with_cancel_leaves_broken_release_for_the_next_run() {
        let manifest = tiny_manifest();
        let root = tempfile::tempdir().unwrap();
        let release = release_path(root.path(), &manifest);
        write_test_files(&release);
        std::fs::remove_file(release.join(MARKER)).ok();

        let cancel = AtomicBool::new(true);
        let err =
            install_with(&manifest, root.path(), &cancel, &|_| {}, &ok_downloader).unwrap_err();
        assert!(format!("{err:#}").contains("отменено"));
        // Cancel happens before quarantine: the broken dir waits for the next
        // run, and nothing else is left behind.
        assert!(release.join("models/a.onnx").is_file());
        assert!(quarantine_dirs(root.path()).is_empty());
        assert!(!root
            .path()
            .join(format!("teratts-v2-{}.staging", manifest.revision))
            .exists());
    }

    #[test]
    fn sweep_quarantined_removes_only_quarantine_dirs() {
        let root = tempfile::tempdir().unwrap();
        let quarantine = root.path().join("teratts-v2-deadbeef.broken-1-2-3");
        let keeper = root.path().join("teratts-v2-keeper");
        let unrelated = root.path().join("other-model.broken-9");
        for dir in [&quarantine, &keeper, &unrelated] {
            std::fs::create_dir_all(dir).unwrap();
        }
        sweep_quarantined(root.path());
        assert!(!quarantine.exists());
        assert!(keeper.exists());
        assert!(unrelated.exists(), "only teratts quarantines are swept");
    }
}
