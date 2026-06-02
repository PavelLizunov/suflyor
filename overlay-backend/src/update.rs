//! In-app auto-update against the project's GitHub Releases.
//!
//! Flow (driven by the Settings → Updates tab):
//!   1. `check_latest(current)` → GET `/releases/latest`, parse the tag +
//!      the installer asset URL, and report whether it's newer.
//!   2. `download_installer(url)` → stream the asset to `%TEMP%`, but ONLY
//!      from a github.com / githubusercontent.com host (defence against a
//!      tampered API response pointing elsewhere).
//!   3. `run_installer(path)` → launch the NSIS installer; the caller then
//!      quits the app so the installer can overwrite the running binary.
//!
//! The repo + asset name are HARD-CODED — there is no user-supplied URL,
//! so the download target can't be redirected by config or UI input.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// `owner/repo` of the official release channel.
const REPO: &str = "PavelLizunov/suflyor";
/// Installer asset filename produced by `scripts/build-slint-release.ps1`.
const INSTALLER_ASSET: &str = "suflyor-slint-setup.exe";

/// Result of an update check.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// Latest release version, tag with any leading `v` stripped (e.g. `0.2.1`).
    pub latest_version: String,
    /// True when `latest_version` is strictly newer than the current build.
    pub newer: bool,
    /// Direct download URL of the installer asset (empty if none attached).
    pub download_url: String,
    /// Human-facing release page (for the "open in browser" fallback).
    pub release_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    /// GitHub API asset digest, e.g. "sha256:abcd…". Absent on older releases.
    #[serde(default)]
    digest: Option<String>,
}

/// Parse a dotted version into `(major, minor, patch, release_rank)`,
/// ignoring a leading `v`. `release_rank` is 1 for a final release and 0
/// when a `-pre`/`-rc` suffix is present, so `0.2.0` correctly ranks above
/// `0.2.0-pre` (a pre-release precedes its release). Non-numeric parts
/// become 0.
fn parse_ver(s: &str) -> (u64, u64, u64, u8) {
    let s = s.trim().trim_start_matches('v');
    let has_pre = s.contains('-');
    let core = s.split('-').next().unwrap_or(s);
    let mut it = core.split('.').map(|x| x.parse::<u64>().unwrap_or(0));
    (
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
        u8::from(!has_pre),
    )
}

/// True if `a` is a strictly newer version than `b`.
#[must_use]
pub fn version_gt(a: &str, b: &str) -> bool {
    parse_ver(a) > parse_ver(b)
}

/// Select the canonical installer asset's download URL from a release's assets.
/// Requires the EXACT `INSTALLER_ASSET` name — there is deliberately NO "any
/// *.exe" fallback, so a renamed/stray/attacker-attached executable in the
/// release can never be chosen as the installer that `run_installer` spawns.
/// Returns an empty string when the named asset is absent (the UI then offers
/// the "open release page" path instead of auto-running anything).
fn pick_installer_url(assets: &[GhAsset]) -> String {
    assets
        .iter()
        .find(|a| a.name == INSTALLER_ASSET)
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_default()
}

/// Query the latest release. `current_version` is the running build's
/// version (e.g. `env!("CARGO_PKG_VERSION")`).
///
/// GitHub requires a User-Agent header or it returns 403; we set one.
/// `/releases/latest` only returns non-prerelease releases, so a `-pre`
/// tagged release is correctly ignored.
pub async fn check_latest(current_version: &str) -> Result<UpdateInfo> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent(concat!("suflyor-updater/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;
    let rel: GhRelease = client
        .get(&url)
        .send()
        .await
        .context("GET releases/latest")?
        .error_for_status()
        .context("releases/latest returned an error status")?
        .json()
        .await
        .context("parse releases/latest JSON")?;

    let latest = rel.tag_name.trim_start_matches('v').to_string();
    let download_url = pick_installer_url(&rel.assets);

    Ok(UpdateInfo {
        newer: version_gt(&latest, current_version),
        latest_version: latest,
        download_url,
        release_url: rel.html_url,
    })
}

/// Allow-list of hosts the installer may be downloaded from. GitHub serves
/// release assets from github.com (which 302-redirects to the second host).
fn is_trusted_download(url: &str) -> bool {
    url.starts_with("https://github.com/")
        || url.starts_with("https://objects.githubusercontent.com/")
        || url.starts_with("https://release-assets.githubusercontent.com/")
}

/// Best-effort: re-query the latest release and return the expected SHA-256
/// (lowercase hex) for the asset at `url`, from GitHub's API `digest` field
/// ("sha256:…"). None when the API exposes no digest (older releases) — the
/// caller then relies on the size + PE-magic checks. Verifying against the API
/// digest closes the "truncated / CDN-tampered download executed" gap; it does
/// NOT defend a fully compromised GitHub release (that needs Authenticode + a
/// code-signing cert, which this project does not have).
async fn expected_sha256_for(client: &reqwest::Client, url: &str) -> Option<String> {
    let api = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let rel: GhRelease = client
        .get(&api)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let asset = rel.assets.iter().find(|a| a.browser_download_url == url)?;
    asset
        .digest
        .as_deref()?
        .strip_prefix("sha256:")
        .map(|h| h.to_lowercase())
}

/// Download the installer to `%TEMP%\suflyor-update\` and return its path.
/// Refuses any non-GitHub URL.
pub async fn download_installer(url: &str) -> Result<PathBuf> {
    if !is_trusted_download(url) {
        bail!("refusing to download installer from untrusted URL");
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!("suflyor-updater/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("build reqwest client")?;
    let bytes = client
        .get(url)
        .send()
        .await
        .context("GET installer asset")?
        .error_for_status()
        .context("installer download returned an error status")?
        .bytes()
        .await
        .context("read installer bytes")?;
    if bytes.len() < 100_000 {
        bail!(
            "downloaded installer is implausibly small ({} bytes)",
            bytes.len()
        );
    }
    // Reject anything that isn't a Windows PE binary (an HTML error page or a
    // truncated blob that slipped past the size floor).
    if bytes.len() < 2 || &bytes[..2] != b"MZ" {
        bail!("downloaded installer is not a Windows executable (bad header)");
    }
    // Verify SHA-256 against GitHub's API digest when present. Defends a
    // truncated download and a CDN-path tamper that doesn't also forge the
    // api.github.com response; full account-compromise still needs Authenticode.
    match expected_sha256_for(&client, url).await {
        Some(expected) => {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let got: String = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if got != expected {
                bail!(
                    "installer SHA-256 mismatch — refusing to run (expected {expected}, got {got})"
                );
            }
            log::info!("update: installer sha256 verified");
        }
        None => {
            log::warn!(
                "update: release metadata has no sha256 digest — size + PE header checked only"
            )
        }
    }
    let dir = std::env::temp_dir().join("suflyor-update");
    std::fs::create_dir_all(&dir).context("create temp update dir")?;
    let path = dir.join(INSTALLER_ASSET);
    std::fs::write(&path, &bytes).context("write installer to temp")?;
    Ok(path)
}

/// Launch the downloaded installer (detached). The caller MUST exit the app
/// right after so the installer can overwrite the running binary; the NSIS
/// installer's first page is interactive, giving the app time to quit.
pub fn run_installer(path: &Path) -> Result<()> {
    std::process::Command::new(path)
        .spawn()
        .context("launch installer")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(version_gt("0.2.1", "0.2.0"));
        assert!(version_gt("0.3.0", "0.2.9"));
        assert!(version_gt("1.0.0", "0.9.9"));
        assert!(!version_gt("0.2.0", "0.2.0"));
        assert!(!version_gt("0.2.0", "0.2.1"));
        // leading v + pre-release suffix are ignored on both sides
        assert!(version_gt("v0.2.0", "0.2.0-pre"));
        assert!(!version_gt("0.2.0-pre", "v0.2.0"));
    }

    #[test]
    fn untrusted_download_host_rejected() {
        assert!(is_trusted_download(
            "https://github.com/PavelLizunov/suflyor/releases/download/v0.2.0/suflyor-slint-setup.exe"
        ));
        assert!(!is_trusted_download("https://evil.example.com/x.exe"));
        assert!(!is_trusted_download("http://github.com/x.exe")); // not https
    }

    #[test]
    fn installer_asset_requires_exact_name() {
        let canonical =
            "https://github.com/PavelLizunov/suflyor/releases/download/v1/suflyor-slint-setup.exe";
        let assets = vec![
            GhAsset {
                name: "release-notes.txt".to_string(),
                browser_download_url: "https://x/notes.txt".to_string(),
                digest: None,
            },
            GhAsset {
                name: INSTALLER_ASSET.to_string(),
                browser_download_url: canonical.to_string(),
                digest: None,
            },
        ];
        assert_eq!(pick_installer_url(&assets), canonical);

        // A stray/renamed .exe must NOT be picked when the canonical asset is
        // absent — pre-fix, the `.or_else(... ends_with(".exe"))` fallback would
        // have returned this and run_installer would have spawned it.
        let stray = vec![GhAsset {
            name: "totally-not-the-installer.exe".to_string(),
            browser_download_url: "https://github.com/x/stray.exe".to_string(),
            digest: None,
        }];
        assert_eq!(pick_installer_url(&stray), "");

        assert_eq!(pick_installer_url(&[]), "");
    }
}
