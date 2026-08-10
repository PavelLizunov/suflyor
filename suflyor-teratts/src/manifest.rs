//! Pinned TeraTTSv2 release manifest (runtime side).
//!
//! The manifest JSON is compiled into both this sidecar and the installer in
//! overlay-backend (`include_str!` of the same file), so the download side and
//! the runtime side can never disagree about the pinned revision. The
//! installer owns hash verification at download time; the sidecar only needs
//! the revision, paths, and sizes to decide "installed and loadable".
//!
//! The JSON keeps per-file `sha256` / `blob_sha1` pins; this parser reads
//! them solely to refuse manifests that lost their integrity pins (serde
//! ignores any other unknown keys, like `model`/`url_template`).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::Deserialize;

pub const MANIFEST_JSON: &str = include_str!("../manifest/teratts-v2.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub revision: String,
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
    pub fn pinned() -> Result<Manifest> {
        Self::from_json(MANIFEST_JSON)
    }

    pub fn from_json(raw: &str) -> Result<Manifest> {
        let manifest: Manifest =
            serde_json::from_str(raw).map_err(|e| anyhow!("manifest parse: {e}"))?;
        if manifest.revision.len() != 40
            || !manifest.revision.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(anyhow!("manifest revision must be a 40-char git sha"));
        }
        if manifest.files.is_empty() {
            return Err(anyhow!("manifest lists no files"));
        }
        for file in &manifest.files {
            if file.path.is_empty()
                || file.path.starts_with('/')
                || file.path.contains("..")
                || file.path.contains('\\')
            {
                return Err(anyhow!("manifest path is unsafe: {:?}", file.path));
            }
            if file.sha256.is_none() && file.blob_sha1.is_none() {
                return Err(anyhow!(
                    "manifest file has no integrity pin: {:?}",
                    file.path
                ));
            }
        }
        Ok(manifest)
    }

    /// `%APPDATA%\suflyor\tts\teratts-v2-<revision>` — the immutable release
    /// directory; one revision, one directory, never mutated after publish.
    pub fn release_dir(&self, tts_root: &Path) -> PathBuf {
        tts_root.join(format!("teratts-v2-{}", self.revision))
    }
}

/// Cheap installed-state check: every manifest entry exists with the pinned
/// size and the publish marker is present. (Full hash verification happens at
/// install time; hashing ~370 MiB on every startup is not worth it.)
pub fn check_installed(manifest: &Manifest, release_dir: &Path) -> Result<()> {
    if !release_dir.join("manifest.json").is_file() {
        return Err(anyhow!("publish marker missing"));
    }
    for entry in &manifest.files {
        let path = release_dir.join(&entry.path);
        let meta = std::fs::metadata(&path).map_err(|_| anyhow!("missing {}", entry.path))?;
        if meta.len() != entry.size {
            return Err(anyhow!(
                "{}: size {} != pinned {}",
                entry.path,
                meta.len(),
                entry.size
            ));
        }
    }
    Ok(())
}

/// Voices shipped by the release, discovered from `styles/<voice>/`.
pub fn installed_voices(release_dir: &Path) -> Vec<String> {
    let mut voices = Vec::new();
    let styles = release_dir.join("styles");
    let Ok(entries) = std::fs::read_dir(&styles) else {
        return voices;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("style_ttl.npy").is_file() || !path.join("style_dp.npy").is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            voices.push(name.to_string());
        }
    }
    voices.sort();
    voices
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn pinned_manifest_parses_and_is_immutable() {
        let manifest = Manifest::pinned().unwrap();
        assert_eq!(
            manifest.revision,
            "f05ea799094571a3553904a555df3834fb0b963b"
        );
        assert_eq!(manifest.files.len(), 27);
        let total: u64 = manifest.files.iter().map(|f| f.size).sum();
        // Core TeraTTSv2 contract without the RUAccent NN subtree: ~370 MiB.
        assert!((380_000_000..400_000_000).contains(&total), "total {total}");
        // Every entry keeps its integrity pin end-to-end.
        for file in &manifest.files {
            assert!(
                file.sha256.is_some() || file.blob_sha1.is_some(),
                "{} lost its pin",
                file.path
            );
        }
    }

    #[test]
    fn pinned_manifest_covers_all_ten_voices() {
        let manifest = Manifest::pinned().unwrap();
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
                assert!(
                    manifest.files.iter().any(|f| f.path == wanted),
                    "manifest misses {wanted}"
                );
            }
        }
    }

    #[test]
    fn release_dir_names_the_revision() {
        let manifest = Manifest::pinned().unwrap();
        let dir = manifest.release_dir(Path::new("C:/appdata/suflyor/tts"));
        assert!(dir.ends_with(format!("teratts-v2-{}", manifest.revision)));
    }

    #[test]
    fn rejects_unsafe_or_unpinned_manifests() {
        let rev = "a".repeat(40);
        let bad_path = format!(
            r#"{{"revision":"{rev}","files":[{{"path":"../evil","size":1,"sha256":"00"}}]}}"#
        );
        assert!(Manifest::from_json(&bad_path).is_err());
        let no_pin = format!(r#"{{"revision":"{rev}","files":[{{"path":"a.bin","size":1}}]}}"#);
        assert!(Manifest::from_json(&no_pin).is_err());
        let short_rev = r#"{"revision":"abc","files":[{"path":"a.bin","size":1,"sha256":"00"}]}"#;
        assert!(Manifest::from_json(short_rev).is_err());
    }

    #[test]
    fn installed_check_requires_marker_and_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = Manifest {
            revision: "a".repeat(40),
            files: vec![ManifestFile {
                path: "styles/v/style_dp.npy".into(),
                size: 4,
                sha256: Some("00".into()),
                blob_sha1: None,
            }],
        };
        assert!(check_installed(&manifest, dir.path()).is_err());
        std::fs::write(dir.path().join("manifest.json"), "{}").unwrap();
        assert!(check_installed(&manifest, dir.path()).is_err());
        let nested = dir.path().join("styles/v");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("style_dp.npy"), b"abcd").unwrap();
        assert!(check_installed(&manifest, dir.path()).is_ok());
        std::fs::write(nested.join("style_dp.npy"), b"abc").unwrap();
        assert!(check_installed(&manifest, dir.path()).is_err());
    }

    #[test]
    fn voice_discovery_needs_both_style_files() {
        let dir = tempfile::tempdir().unwrap();
        let complete = dir.path().join("styles/ru_f1");
        let partial = dir.path().join("styles/ru_m1");
        std::fs::create_dir_all(&complete).unwrap();
        std::fs::create_dir_all(&partial).unwrap();
        std::fs::write(complete.join("style_ttl.npy"), b"x").unwrap();
        std::fs::write(complete.join("style_dp.npy"), b"x").unwrap();
        std::fs::write(partial.join("style_ttl.npy"), b"x").unwrap();
        assert_eq!(installed_voices(dir.path()), vec!["ru_f1".to_string()]);
    }
}
