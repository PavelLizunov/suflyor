//! Download-on-demand installer for the speaker-diarization models (D1).
//!
//! Like [`crate::tts_install`] (voices) and [`crate::ocr_install`] (Tesseract),
//! the models are NOT bundled in the app installer — the user installs them with
//! a button in Settings. Two models are needed TOGETHER: a pyannote segmentation
//! model (a `.tar.bz2` extracted to a dir) + a WeSpeaker speaker-embedding model
//! (a raw `.onnx`). Both from the sherpa-onnx GitHub releases (the same source the
//! TTS voices come from). SHA-256 pins make each download verify-before-use.
//!
//! The `suflyor-tts diarize` subcommand loads them from [`seg_model_path`] /
//! [`emb_model_path`] under `%APPDATA%\suflyor\diar\`. Download/verify/extract are
//! the shared [`crate::download`] helpers.
//!
//! Crash safety (suflyor H1): a partial/corrupt install must never look ready.
//! The archive model is extracted into a SIBLING STAGING dir (`.staging.<top>`)
//! and renamed onto the live tree only once its required file is present; the raw
//! model is renamed from its verified temp download. Only after the COMPLETE set
//! is on disk is the completion sentinel (`installed.json`, this build's verified
//! pins) written — atomically, as the last step. [`models_installed`] requires the
//! sentinel AND every marker, so a hard kill at ANY point (download, extract,
//! between the swap and the sentinel) leaves the install visibly NOT installed:
//! the Settings/transcript install button stays up and the next click re-runs the
//! whole install (stale staging/temp artifacts are swept first).

use crate::download::{curl_download, extract_tar_bz2, verify_sha256};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALL_BUSY: AtomicBool = AtomicBool::new(false);

struct InstallGuard;

impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALL_BUSY.store(false, Ordering::Release);
    }
}

fn try_acquire_install() -> Result<InstallGuard> {
    if INSTALL_BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        bail!("установка моделей уже выполняется");
    }
    Ok(InstallGuard)
}

/// A downloadable diarization model.
struct DiarModel {
    /// Friendly label for the Settings progress messages.
    label: &'static str,
    /// Download URL (sherpa-onnx GitHub release asset).
    url: &'static str,
    /// SHA-256 of the downloaded file, verified before use.
    sha256: &'static str,
    /// The asset filename (also the temp download name).
    filename: &'static str,
    /// `true` = a `.tar.bz2` to extract into `diar/`; `false` = a raw file to place.
    archive: bool,
    /// Path under `diar/` that must exist once this model is installed.
    marker: &'static str,
}

/// The two models the diarizer needs (both required). Segmentation is a `.tar.bz2`
/// (extracts to a dir with `model.onnx`); embedding is a raw `.onnx`.
const DIAR_MODELS: &[DiarModel] = &[
    DiarModel {
        label: "Сегментация речи",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
        sha256: "24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488",
        filename: "sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
        archive: true,
        marker: "sherpa-onnx-pyannote-segmentation-3-0/model.onnx",
    },
    DiarModel {
        label: "Голосовые эмбеддинги",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34.onnx",
        sha256: "5ef208a9da1453335308a6b6f4e6dfbd7e183a38b604de0a57664f45d257fe94",
        filename: "wespeaker_en_voxceleb_resnet34.onnx",
        archive: false,
        marker: "wespeaker_en_voxceleb_resnet34.onnx",
    },
];

/// The completion sentinel's file name inside `diar/`. Its presence (valid +
/// matching the current pins) is what turns a set of files on disk into an
/// "installed" set — a partial tree can never forge it.
const SENTINEL_FILE: &str = "installed.json";
/// Bump if the sentinel's MEANING changes (a layout the old content can't vouch
/// for). A mismatch is "no sentinel" → the install button stays up and the next
/// run re-installs + rewrites it.
const SENTINEL_VERSION: u32 = 1;

/// Prefix of the per-model staging dirs (extract target + swap source). A hard
/// kill mid-extract leaves ONLY this dir dirty — [`clean_stale`] wipes it on the
/// next run, and [`models_installed`] never looks inside it.
const STAGING_PREFIX: &str = ".staging.";
/// Suffix of the in-progress download temp files (also swept by [`clean_stale`]).
const DOWNLOAD_SUFFIX: &str = ".download";

/// Coarse progress for the Settings UI (a step label per model — the packs are
/// small and the steps quick, so no byte bar).
pub enum DiarProgress {
    AlreadyInstalled(String),
    Downloading(String),
    Verifying(String),
    Unpacking(String),
    ModelFailed(String),
    AllInstalled,
}

/// The completion sentinel: the layout version + the verified download pin of
/// every committed model. Written ONLY after the complete set landed; read to
/// tell a committed set from a partial/torn one (and a stale one — pins changed
/// in a later build — from a current one).
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct Sentinel {
    version: u32,
    /// Asset filename → the SHA-256 pin of the download that produced the tree.
    models: std::collections::BTreeMap<String, String>,
}

/// The sentinel this build expects over a committed set (pure — from the pins).
fn expected_sentinel() -> Sentinel {
    Sentinel {
        version: SENTINEL_VERSION,
        models: DIAR_MODELS
            .iter()
            .map(|m| (m.filename.to_string(), m.sha256.to_string()))
            .collect(),
    }
}

/// `%APPDATA%\suflyor\diar` — where the models install (and where the sidecar
/// loads them from).
#[must_use]
pub fn diar_dir() -> Option<PathBuf> {
    crate::paths::data_root().map(|d| d.join("diar"))
}

/// Absolute path to the pyannote segmentation model (`None` if APPDATA unset).
#[must_use]
pub fn seg_model_path() -> Option<PathBuf> {
    diar_dir().map(|d| d.join(DIAR_MODELS[0].marker))
}

/// Absolute path to the WeSpeaker embedding model (`None` if APPDATA unset).
#[must_use]
pub fn emb_model_path() -> Option<PathBuf> {
    diar_dir().map(|d| d.join(DIAR_MODELS[1].marker))
}

/// True only when the COMPLETE committed set is on disk: a valid sentinel (this
/// build's version + pins) AND both models' markers. Diarization needs both, so
/// a partial install is "not installed"; a hard-killed install (markers without
/// a sentinel, or a stale sentinel) is ALSO "not installed" — which keeps the
/// install button up so the next click re-runs the install. Drives "Install" vs
/// "Installed" in Settings and gates the «Определить говорящих» button.
#[must_use]
pub fn models_installed() -> bool {
    let Some(root) = diar_dir() else {
        return false;
    };
    models_installed_in(&root)
}

/// Path-level [`models_installed`] (test seam — operates on an explicit root).
fn models_installed_in(root: &Path) -> bool {
    sentinel_valid(root) && DIAR_MODELS.iter().all(|m| root.join(m.marker).is_file())
}

/// True when the sentinel exists, parses, and vouches for exactly this build's
/// pinned set. Missing/garbage/wrong-version/stale-pin → false.
fn sentinel_valid(root: &Path) -> bool {
    let Ok(bytes) = std::fs::read(root.join(SENTINEL_FILE)) else {
        return false;
    };
    let Ok(sentinel) = serde_json::from_slice::<Sentinel>(&bytes) else {
        return false;
    };
    sentinel == expected_sentinel()
}

/// Download + verify + (extract) both models. Blocking — the caller runs it on a
/// worker thread (mirrors `tts_install::install_voices`). `cancel` is polled
/// between models; `on` receives step messages. Fails unless BOTH end up
/// installed (the diarizer can't run with only one).
///
/// # Errors
/// If APPDATA is unset, the dir can't be created, the sentinel can't be written,
/// or (after the loop) not both models are present.
pub fn install_models(cancel: &AtomicBool, on: &dyn Fn(DiarProgress)) -> Result<()> {
    // Both Settings and the transcript can start this operation. Serialize at
    // the filesystem boundary so they cannot race on shared staging files.
    let _guard = try_acquire_install()?;
    let root = diar_dir().context("APPDATA not set — no diar dir")?;
    install_models_in(&root, cancel, on)
}

/// Path-level [`install_models`] (test seam — operates on an explicit root, so
/// the staged-commit/sentinel logic is testable without APPDATA; the download
/// path itself needs the network and stays covered by the pin tests).
fn install_models_in(root: &Path, cancel: &AtomicBool, on: &dyn Fn(DiarProgress)) -> Result<()> {
    std::fs::create_dir_all(root).with_context(|| format!("create {}", root.display()))?;
    // Deterministic recovery: wipe artifacts a hard kill could have left (staging
    // dirs mid-extract, temp downloads) before deciding what's already installed.
    clean_stale(root);

    // Already complete + current → nothing to fetch (the button is normally hidden
    // in this state; this is the defensive no-op + the network-free test seam).
    if models_installed_in(root) {
        for m in DIAR_MODELS {
            on(DiarProgress::AlreadyInstalled(m.label.to_string()));
        }
        on(DiarProgress::AllInstalled);
        return Ok(());
    }

    // Keep readiness false for the whole repair/reinstall. A still-valid sentinel
    // plus an old marker could otherwise briefly make the set look complete after
    // the first model lands but before the second one is replaced.
    invalidate_sentinel(root)?;

    // Install each model, continuing past a failure so the UI can name EVERY model
    // that failed (Settings shows a per-model «Не скачалось»). A failed model
    // leaves nothing partial on the live names (install_one sweeps its staging +
    // temp), so the post-loop validation is what decides success.
    let mut failed: Vec<&str> = Vec::new();
    for m in DIAR_MODELS {
        if cancel.load(Ordering::Acquire) {
            bail!("отменено");
        }
        if let Err(e) = install_one(m, root, on) {
            log::warn!("diar model '{}' failed: {e:#}", m.filename);
            failed.push(m.label);
            on(DiarProgress::ModelFailed(m.label.to_string()));
        }
    }

    if !failed.is_empty() {
        bail!("не удалось установить: {}", failed.join(", "));
    }
    // Validate the required model files BEFORE activation. Any model missing → the
    // set is NOT committed (no sentinel), so it can never look installed.
    if !DIAR_MODELS.iter().all(|m| root.join(m.marker).is_file()) {
        bail!("модели установлены не полностью");
    }
    // The complete set is on disk — commit the sentinel that makes it "installed".
    // This is the ONLY writer, so a valid sentinel always vouches for a complete,
    // current set.
    write_sentinel(root).context("commit diar install sentinel")?;
    on(DiarProgress::AllInstalled);
    Ok(())
}

fn invalidate_sentinel(root: &Path) -> Result<()> {
    let path = root.join(SENTINEL_FILE);
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("invalidate {}", path.display()))?;
    }
    Ok(())
}

/// Download + verify + place ONE model. The archive model is extracted into a
/// staging sibling and swapped onto the live tree only when complete; the raw
/// model is renamed from its verified temp download. On failure nothing partial
/// reaches the live names (staging + temps are wiped), so a half-written model
/// can't masquerade as installed.
fn install_one(m: &DiarModel, root: &Path, on: &dyn Fn(DiarProgress)) -> Result<()> {
    on(DiarProgress::Downloading(m.label.to_string()));
    let tmp = root.join(format!("{}{DOWNLOAD_SUFFIX}", m.filename));
    let _ = std::fs::remove_file(&tmp);
    curl_download(m.url, &tmp).with_context(|| format!("download {}", m.label))?;

    on(DiarProgress::Verifying(m.label.to_string()));
    // A failed verify leaves the `.download` temp, which clean_stale sweeps on the
    // next run (it is never read by the readiness check).
    verify_sha256(&tmp, m.sha256, m.label)?;

    if m.archive {
        on(DiarProgress::Unpacking(m.label.to_string()));
        // Stage the extraction OFF the live tree: a kill mid-extract leaves only
        // the staging dir dirty (swept by clean_stale), never a partial live tree.
        let stage = root.join(format!("{STAGING_PREFIX}{}", marker_top(m)));
        let _ = std::fs::remove_dir_all(&stage);
        std::fs::create_dir_all(&stage)
            .with_context(|| format!("create staging for {}", m.label))?;
        if let Err(e) = extract_tar_bz2(&tmp, &stage) {
            let _ = std::fs::remove_file(&tmp);
            let _ = std::fs::remove_dir_all(&stage);
            return Err(e).with_context(|| format!("extract {}", m.label));
        }
        let _ = std::fs::remove_file(&tmp);
        commit_staged(m, root, &stage)
    } else {
        // Windows rename does not replace an existing file. Remove the old live
        // file first; readiness remains false until the final sentinel is written.
        let dest = root.join(m.filename);
        if dest.exists() {
            std::fs::remove_file(&dest).with_context(|| format!("replace {}", m.label))?;
        }
        std::fs::rename(&tmp, &dest)
            .inspect_err(|_| {
                let _ = std::fs::remove_file(&tmp);
            })
            .with_context(|| format!("place {}", m.label))
    }
}

/// Swap a staged extraction onto the live tree: validate the required marker
/// INSIDE the staging dir first (a partial extract never commits), clear any
/// old/partial live dir, rename the staged top dir over it (atomic same-volume
/// rename), and wipe the staging shell. A kill between the clear and the rename
/// leaves the model simply ABSENT (not installed → the button re-runs), never
/// partial.
fn commit_staged(m: &DiarModel, root: &Path, stage: &Path) -> Result<()> {
    if !stage.join(m.marker).is_file() {
        // Wipe the partial staging so it can't look installed.
        let _ = std::fs::remove_dir_all(stage);
        bail!("{}: модель установлена не полностью", m.label);
    }
    let staged_top = stage.join(marker_top(m));
    let dest = root.join(marker_top(m));
    if dest.exists() {
        std::fs::remove_dir_all(&dest).with_context(|| format!("replace {}", m.label))?;
    }
    std::fs::rename(&staged_top, &dest)
        .inspect_err(|_| {
            let _ = std::fs::remove_dir_all(stage);
        })
        .with_context(|| format!("place {}", m.label))?;
    let _ = std::fs::remove_dir_all(stage); // the now-empty staging shell
    Ok(())
}

/// Write the completion sentinel for the current pinned set. Called only once
/// the complete set is on disk. Windows cannot rename over an existing file, so
/// an old sentinel is removed first; a crash in that gap is safely "not ready".
fn write_sentinel(root: &Path) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(&expected_sentinel()).context("encode sentinel")?;
    let final_path = root.join(SENTINEL_FILE);
    let tmp = root.join(format!("{SENTINEL_FILE}.tmp"));
    std::fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    if final_path.exists() {
        std::fs::remove_file(&final_path)
            .with_context(|| format!("replace {}", final_path.display()))?;
    }
    std::fs::rename(&tmp, &final_path)
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
        .with_context(|| format!("commit {}", final_path.display()))?;
    Ok(())
}

/// Remove artifacts a hard kill can leave behind: per-model staging dirs and
/// temp downloads. Best-effort — a locked file just stays until the next run;
/// neither kind is ever read by the readiness check, so a leftover is inert.
fn clean_stale(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.starts_with(STAGING_PREFIX) {
            let _ = std::fs::remove_dir_all(entry.path());
        } else if name.ends_with(DOWNLOAD_SUFFIX) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// The top-level path component of a model's marker — the dir an archive extracts
/// into (e.g. `sherpa-onnx-pyannote-segmentation-3-0`). For a raw model (marker =
/// the file) this is the file name itself.
fn marker_top(m: &DiarModel) -> &str {
    m.marker.split(['/', '\\']).next().unwrap_or(m.marker)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// Force the segmentation marker on disk (a fake extracted tree).
    fn force_seg_marker(root: &Path) {
        let dir = root.join("sherpa-onnx-pyannote-segmentation-3-0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("model.onnx"), b"fake-onnx").unwrap();
    }

    /// Force the embedding marker on disk (a fake placed file).
    fn force_emb_marker(root: &Path) {
        std::fs::write(root.join("wespeaker_en_voxceleb_resnet34.onnx"), b"fake").unwrap();
    }

    fn force_both_markers(root: &Path) {
        force_seg_marker(root);
        force_emb_marker(root);
    }

    #[test]
    fn pins_and_layout_are_valid() {
        for m in DIAR_MODELS {
            assert!(
                m.url.starts_with("https://github.com/k2-fsa/sherpa-onnx/"),
                "{}",
                m.label
            );
            assert!(
                m.url.ends_with(m.filename),
                "url must end with the filename: {}",
                m.label
            );
            assert_eq!(m.sha256.len(), 64, "{} sha must be 64 hex", m.label);
            assert!(
                m.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                "{}",
                m.label
            );
        }
        // Diarization needs BOTH: exactly one archive (seg) + one raw onnx (emb).
        assert_eq!(DIAR_MODELS.iter().filter(|m| m.archive).count(), 1);
        assert_eq!(DIAR_MODELS.iter().filter(|m| !m.archive).count(), 1);
        // The public path getters index [0]=seg, [1]=emb — pin that layout.
        assert!(DIAR_MODELS[0].marker.ends_with("/model.onnx"));
        assert_eq!(DIAR_MODELS[1].marker, DIAR_MODELS[1].filename);
    }

    #[test]
    fn marker_top_is_the_extract_dir_or_the_file() {
        // archive marker "dir/model.onnx" → "dir"; raw marker "x.onnx" → "x.onnx".
        assert_eq!(
            marker_top(&DIAR_MODELS[0]),
            "sherpa-onnx-pyannote-segmentation-3-0"
        );
        assert_eq!(
            marker_top(&DIAR_MODELS[1]),
            "wespeaker_en_voxceleb_resnet34.onnx"
        );
    }

    #[test]
    fn empty_root_is_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!models_installed_in(tmp.path()));
    }

    #[test]
    fn markers_without_sentinel_are_not_installed() {
        // The H1 crash shape: a kill after the files landed but before the
        // sentinel commit. Must NOT report installed — the button stays up so the
        // click re-runs the install.
        let tmp = tempfile::tempdir().unwrap();
        force_both_markers(tmp.path());
        assert!(!models_installed_in(tmp.path()));
    }

    #[test]
    fn staged_partial_tree_never_reports_installed() {
        // A kill mid-extract: the marker exists ONLY inside the staging dir.
        let tmp = tempfile::tempdir().unwrap();
        let stage = tmp.path().join(format!(
            "{STAGING_PREFIX}sherpa-onnx-pyannote-segmentation-3-0"
        ));
        std::fs::create_dir_all(stage.join("sherpa-onnx-pyannote-segmentation-3-0")).unwrap();
        std::fs::write(
            stage.join("sherpa-onnx-pyannote-segmentation-3-0/model.onnx"),
            b"partial",
        )
        .unwrap();
        force_emb_marker(tmp.path());
        assert!(!models_installed_in(tmp.path()));
        // clean_stale wipes the staging shell; the live tree is untouched.
        clean_stale(tmp.path());
        assert!(!stage.exists());
        assert!(tmp
            .path()
            .join("wespeaker_en_voxceleb_resnet34.onnx")
            .is_file());
    }

    #[test]
    fn valid_sentinel_with_full_set_is_installed() {
        let tmp = tempfile::tempdir().unwrap();
        force_both_markers(tmp.path());
        write_sentinel(tmp.path()).unwrap();
        assert!(models_installed_in(tmp.path()));
    }

    #[test]
    fn valid_sentinel_with_a_missing_marker_is_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        force_seg_marker(tmp.path());
        write_sentinel(tmp.path()).unwrap(); // sentinel claims both…
        assert!(!models_installed_in(tmp.path())); // …but the emb file is gone
    }

    #[test]
    fn reinstall_invalidates_old_sentinel_until_final_commit() {
        let tmp = tempfile::tempdir().unwrap();
        force_both_markers(tmp.path());
        write_sentinel(tmp.path()).unwrap();
        std::fs::remove_file(tmp.path().join(DIAR_MODELS[0].marker)).unwrap();

        invalidate_sentinel(tmp.path()).unwrap();
        force_seg_marker(tmp.path());

        assert!(
            !models_installed_in(tmp.path()),
            "restoring one marker mid-install must not reactivate the old sentinel"
        );
    }

    #[test]
    fn stale_or_garbage_sentinel_is_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        force_both_markers(tmp.path());
        // Wrong version → stale.
        let stale = Sentinel {
            version: SENTINEL_VERSION + 1,
            models: expected_sentinel().models,
        };
        std::fs::write(
            tmp.path().join(SENTINEL_FILE),
            serde_json::to_vec(&stale).unwrap(),
        )
        .unwrap();
        assert!(!models_installed_in(tmp.path()));
        // A mismatched pin (the build updated a model) → stale.
        let mut wrong = expected_sentinel();
        wrong
            .models
            .insert(DIAR_MODELS[1].filename.to_string(), "0".repeat(64));
        std::fs::write(
            tmp.path().join(SENTINEL_FILE),
            serde_json::to_vec(&wrong).unwrap(),
        )
        .unwrap();
        assert!(!models_installed_in(tmp.path()));
        // Garbage bytes → unparseable → not installed.
        std::fs::write(tmp.path().join(SENTINEL_FILE), b"not json").unwrap();
        assert!(!models_installed_in(tmp.path()));
    }

    #[test]
    fn already_installed_is_a_no_op() {
        // A complete + current set short-circuits BEFORE any download is attempted
        // (no network is reachable here, so a fetch would fail the test). This is
        // the orchestration seam that needs no APPDATA and no network.
        let tmp = tempfile::tempdir().unwrap();
        force_both_markers(tmp.path());
        write_sentinel(tmp.path()).unwrap();
        let cancel = AtomicBool::new(false);
        let already = std::cell::Cell::new(0);
        let done = std::cell::Cell::new(false);
        install_models_in(tmp.path(), &cancel, &|p| match p {
            DiarProgress::AlreadyInstalled(_) => already.set(already.get() + 1),
            DiarProgress::AllInstalled => done.set(true),
            DiarProgress::ModelFailed(l) => panic!("unexpected failure: {l}"),
            _ => panic!("unexpected download step"),
        })
        .unwrap();
        assert_eq!(
            already.get(),
            DIAR_MODELS.len(),
            "every model reused, none fetched"
        );
        assert!(done.get());
        assert!(models_installed_in(tmp.path()));
    }

    #[test]
    fn install_guard_allows_only_one_writer() {
        let first = try_acquire_install().unwrap();
        assert!(try_acquire_install().is_err());
        drop(first);
        assert!(try_acquire_install().is_ok());
    }

    #[test]
    fn commit_staged_swaps_a_complete_tree_over_an_old_one() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let m = &DIAR_MODELS[0];
        // A stale/partial live dir from a previous crashed install.
        let live = root.join("sherpa-onnx-pyannote-segmentation-3-0");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::write(live.join("torn.onnx"), b"partial junk").unwrap();
        // The staged extraction: complete (marker present) + a sibling file.
        let stage = root.join(format!(
            "{STAGING_PREFIX}sherpa-onnx-pyannote-segmentation-3-0"
        ));
        let staged_top = stage.join("sherpa-onnx-pyannote-segmentation-3-0");
        std::fs::create_dir_all(&staged_top).unwrap();
        std::fs::write(staged_top.join("model.onnx"), b"the real model").unwrap();
        std::fs::write(staged_top.join("tokens.txt"), b"tokens").unwrap();

        commit_staged(m, root, &stage).unwrap();

        assert!(root.join(m.marker).is_file(), "marker committed");
        assert_eq!(
            std::fs::read(root.join("sherpa-onnx-pyannote-segmentation-3-0/tokens.txt")).unwrap(),
            b"tokens",
            "the whole staged tree landed"
        );
        assert!(
            !root
                .join("sherpa-onnx-pyannote-segmentation-3-0/torn.onnx")
                .exists(),
            "the old partial tree is gone"
        );
        assert!(!stage.exists(), "staging shell cleaned up");
    }

    #[test]
    fn commit_staged_rejects_a_partial_extraction() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let m = &DIAR_MODELS[0];
        // Staged WITHOUT the marker (a kill mid-extract).
        let stage = root.join(format!(
            "{STAGING_PREFIX}sherpa-onnx-pyannote-segmentation-3-0"
        ));
        std::fs::create_dir_all(stage.join("sherpa-onnx-pyannote-segmentation-3-0")).unwrap();
        std::fs::write(
            stage.join("sherpa-onnx-pyannote-segmentation-3-0/tokens.txt"),
            b"tokens",
        )
        .unwrap();
        // A prior live tree that must survive the rejected commit.
        let live = root.join("sherpa-onnx-pyannote-segmentation-3-0");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::write(live.join("model.onnx"), b"previous").unwrap();

        assert!(commit_staged(m, root, &stage).is_err());
        assert!(!stage.exists(), "partial staging wiped");
        assert_eq!(
            std::fs::read(root.join("sherpa-onnx-pyannote-segmentation-3-0/model.onnx")).unwrap(),
            b"previous",
            "the live tree is untouched by a rejected commit"
        );
    }

    #[test]
    fn clean_stale_sweeps_staging_and_temp_downloads_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        force_both_markers(root);
        std::fs::create_dir_all(root.join(format!("{STAGING_PREFIX}x"))).unwrap();
        std::fs::write(root.join("something.tar.bz2.download"), b"temp").unwrap();
        std::fs::write(root.join(SENTINEL_FILE), b"{}").unwrap();

        clean_stale(root);

        assert!(!root.join(format!("{STAGING_PREFIX}x")).exists());
        assert!(!root.join("something.tar.bz2.download").exists());
        // Real files survive.
        assert!(root
            .join("sherpa-onnx-pyannote-segmentation-3-0/model.onnx")
            .is_file());
        assert!(root.join("wespeaker_en_voxceleb_resnet34.onnx").is_file());
        assert!(root.join(SENTINEL_FILE).is_file());
    }

    #[test]
    fn sentinel_roundtrip_is_atomic_and_verifies() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!sentinel_valid(tmp.path()));
        write_sentinel(tmp.path()).unwrap();
        assert!(sentinel_valid(tmp.path()));
        assert!(!tmp.path().join(format!("{SENTINEL_FILE}.tmp")).exists());
        // Tampered pin → invalid.
        let mut tampered = expected_sentinel();
        tampered
            .models
            .insert(DIAR_MODELS[0].filename.to_string(), "f".repeat(64));
        std::fs::write(
            tmp.path().join(SENTINEL_FILE),
            serde_json::to_vec(&tampered).unwrap(),
        )
        .unwrap();
        assert!(!sentinel_valid(tmp.path()));
    }
}
