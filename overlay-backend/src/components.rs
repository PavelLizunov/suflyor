//! Read-only "what's installed" readiness API for the on-demand components the
//! user installs from Settings (llama.cpp engine, local LLM weights, STT, neural
//! voices, OCR).
//!
//! The onboarding **«Компоненты»** hub and the first-run readiness dashboard
//! render this. Each component reuses the SAME `*_install` / `*_present` checks
//! the install buttons already use, so the hub can never disagree with what the
//! app actually sees as installed.
//!
//! Cheap: stat-only (no model loads), safe to call on Settings open / at
//! startup. The labelling logic is split into small pure helpers so it is
//! unit-testable without materialising multi-GB model files.

use crate::config::Config;

/// One installable component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// llama.cpp runtime — the engine that serves the local model.
    Engine,
    /// Local LLM weights — Gemma 4B base (+ optional 12B "smarter").
    LocalModel,
    /// Local speech-to-text — GigaAM.
    Stt,
    /// Read-aloud neural voices — the TTS sidecar.
    Voices,
    /// On-screen text recognition — Tesseract OCR.
    Ocr,
}

/// Readiness of one component.
#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub kind: ComponentKind,
    /// True when the component is installed + usable right now.
    pub installed: bool,
    /// Short, locale-neutral detail when installed (version / tier / engine),
    /// e.g. `"b9637"`, `"Gemma 4B + 12B"`, `"Tesseract"`. Empty when not
    /// installed (the UI shows its own «⬇ установить» from `installed`).
    pub detail: String,
}

/// Snapshot of every component's readiness. Reads the filesystem under
/// [`crate::local_ai::default_root`] + the config's STT dir.
#[must_use]
pub fn status(cfg: &Config) -> Vec<ComponentStatus> {
    let root = crate::local_ai::default_root();

    // Engine (llama.cpp build number).
    let engine_build = crate::local_ai::installed_engine_build(&root);
    let engine = ComponentStatus {
        kind: ComponentKind::Engine,
        installed: engine_build.is_some(),
        detail: engine_detail(engine_build),
    };

    // Local model: 4B base is the minimum; 12B is optional on top.
    let base = crate::local_ai::base_model_present(&root);
    let quality = crate::local_ai::quality_model_present(&root);
    let local_model = ComponentStatus {
        kind: ComponentKind::LocalModel,
        installed: base,
        detail: local_model_detail(base, quality),
    };

    // STT (GigaAM): installed when the model file exists at the pinned size in
    // the configured (or default) dir. Reuses local_ai's path + presence check
    // (single source of truth with the installer) — NOT validate_gigaam_dir,
    // which loads the model.
    let gigaam_dir = if cfg.stt_gigaam_dir.trim().is_empty() {
        crate::local_ai::gigaam_default_dir(&root)
    } else {
        std::path::PathBuf::from(cfg.stt_gigaam_dir.trim())
    };
    let stt_installed = crate::local_ai::gigaam_model_present(&gigaam_dir);
    let stt = ComponentStatus {
        kind: ComponentKind::Stt,
        installed: stt_installed,
        detail: if stt_installed {
            "GigaAM".into()
        } else {
            String::new()
        },
    };

    // Voices (TTS sidecar) + OCR (Tesseract) — their own installed checks.
    let voices_installed = crate::tts_install::any_voice_installed();
    let voices = ComponentStatus {
        kind: ComponentKind::Voices,
        installed: voices_installed,
        detail: if voices_installed {
            "Neural (Piper)".into()
        } else {
            String::new()
        },
    };

    let ocr_installed = crate::ocr::is_available();
    let ocr = ComponentStatus {
        kind: ComponentKind::Ocr,
        installed: ocr_installed,
        detail: if ocr_installed {
            "Tesseract".into()
        } else {
            String::new()
        },
    };

    vec![engine, local_model, stt, voices, ocr]
}

/// Pure: engine detail from the optional build number.
fn engine_detail(build: Option<u32>) -> String {
    build.map(|b| format!("b{b}")).unwrap_or_default()
}

/// Pure: local-model detail from the 4B / 12B presence flags.
fn local_model_detail(base: bool, quality: bool) -> String {
    match (base, quality) {
        (false, _) => String::new(),
        (true, true) => "Gemma 4B + 12B".to_string(),
        (true, false) => "Gemma 4B".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_detail_formats_build_or_empty() {
        assert_eq!(engine_detail(Some(9637)), "b9637");
        assert_eq!(engine_detail(None), "");
    }

    #[test]
    fn local_model_detail_covers_tiers() {
        assert_eq!(local_model_detail(false, false), "");
        assert_eq!(local_model_detail(false, true), ""); // 12B without 4B → still not usable
        assert_eq!(local_model_detail(true, false), "Gemma 4B");
        assert_eq!(local_model_detail(true, true), "Gemma 4B + 12B");
    }

    #[test]
    fn status_returns_every_component_once() {
        let cfg = Config::defaults();
        let s = status(&cfg);
        assert_eq!(s.len(), 5);
        for k in [
            ComponentKind::Engine,
            ComponentKind::LocalModel,
            ComponentKind::Stt,
            ComponentKind::Voices,
            ComponentKind::Ocr,
        ] {
            assert_eq!(s.iter().filter(|c| c.kind == k).count(), 1, "{k:?} once");
        }
        // Not-installed components carry an empty detail (UI owns the label).
        for c in &s {
            if !c.installed {
                assert!(c.detail.is_empty(), "{:?} empty when absent", c.kind);
            }
        }
    }
}
