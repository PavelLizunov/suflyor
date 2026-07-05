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

use crate::download::{curl_download, extract_tar_bz2, verify_sha256};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

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

/// True only when BOTH models are on disk (diarization needs both, so a partial
/// install is "not installed" — drives "Install" vs "Installed" in Settings and
/// gates the «Определить говорящих» button).
#[must_use]
pub fn models_installed() -> bool {
    let Some(root) = diar_dir() else {
        return false;
    };
    DIAR_MODELS.iter().all(|m| root.join(m.marker).is_file())
}

/// Download + verify + (extract) both models. Blocking — the caller runs it on a
/// worker thread (mirrors `tts_install::install_voices`). `cancel` is polled
/// between models; `on` receives step messages. Fails unless BOTH end up
/// installed (the diarizer can't run with only one).
///
/// # Errors
/// If APPDATA is unset, the dir can't be created, a download/verify/extract fails,
/// or (after the loop) not both models are present.
pub fn install_models(cancel: &AtomicBool, on: &dyn Fn(DiarProgress)) -> Result<()> {
    let root = diar_dir().context("APPDATA not set — no diar dir")?;
    std::fs::create_dir_all(&root).with_context(|| format!("create {}", root.display()))?;

    let mut failed: Vec<&str> = Vec::new();
    for m in DIAR_MODELS {
        if cancel.load(Ordering::Acquire) {
            bail!("отменено");
        }
        if root.join(m.marker).is_file() {
            on(DiarProgress::AlreadyInstalled(m.label.to_string()));
            continue;
        }
        match install_one(m, &root, on) {
            Ok(()) => {}
            Err(e) => {
                log::warn!("diar model '{}' failed: {e:#}", m.filename);
                failed.push(m.label);
                on(DiarProgress::ModelFailed(m.label.to_string()));
            }
        }
    }

    if !models_installed() {
        if failed.is_empty() {
            bail!("модели установлены не полностью");
        }
        bail!("не удалось установить: {}", failed.join(", "));
    }
    on(DiarProgress::AllInstalled);
    Ok(())
}

/// Download + verify + place ONE model. On failure the partial output (extracted
/// dir or placed file) is removed so a half-written model can't masquerade as
/// installed.
fn install_one(m: &DiarModel, root: &Path, on: &dyn Fn(DiarProgress)) -> Result<()> {
    on(DiarProgress::Downloading(m.label.to_string()));
    let tmp = root.join(format!("{}.download", m.filename));
    let _ = std::fs::remove_file(&tmp);
    curl_download(m.url, &tmp).with_context(|| format!("download {}", m.label))?;

    on(DiarProgress::Verifying(m.label.to_string()));
    verify_sha256(&tmp, m.sha256, m.label)?;

    if m.archive {
        on(DiarProgress::Unpacking(m.label.to_string()));
        if let Err(e) = extract_tar_bz2(&tmp, root) {
            let _ = std::fs::remove_file(&tmp);
            let _ = std::fs::remove_dir_all(root.join(marker_top(m)));
            return Err(e).with_context(|| format!("extract {}", m.label));
        }
        let _ = std::fs::remove_file(&tmp);
    } else {
        // Raw file: atomically move the verified download onto its final name.
        // Drop the temp on failure so a rename error doesn't leak the ~26 MB file.
        let dest = root.join(m.filename);
        std::fs::rename(&tmp, &dest)
            .inspect_err(|_| {
                let _ = std::fs::remove_file(&tmp);
            })
            .with_context(|| format!("place {}", m.label))?;
    }

    if !root.join(m.marker).is_file() {
        // Wipe a partial install so it can't look installed.
        if m.archive {
            let _ = std::fs::remove_dir_all(root.join(marker_top(m)));
        } else {
            let _ = std::fs::remove_file(root.join(m.filename));
        }
        bail!("{} установлен не полностью", m.label);
    }
    Ok(())
}

/// The top-level path component of a model's marker — the dir an archive extracts
/// into (e.g. `sherpa-onnx-pyannote-segmentation-3-0`), for cleanup of a partial
/// extraction. For a raw model (marker = the file) this is the file name itself.
fn marker_top(m: &DiarModel) -> &str {
    m.marker.split(['/', '\\']).next().unwrap_or(m.marker)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
