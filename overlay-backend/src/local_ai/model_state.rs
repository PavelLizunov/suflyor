use std::path::{Path, PathBuf};

use super::{
    cached_pinned_file_matches, custom_choice_alias, detected_hardware_model_profile,
    file_has_expected_size, file_len, is_managed_llama_endpoint, legacy_gguf_complete,
    quality_model_verified, valid_custom_choice_path, valid_custom_gguf_path,
    HardwareModelProfile, LocalContextPreset, ManagedLlamaChoice, ManagedModel, GEMMA26_FILE,
    GEMMA26_MIN_BUILD, GEMMA26_SHA256, GEMMA26_SIZE, GEMMA4UV_MIN_BUILD, GEMMA_FILE, GEMMA_SIZE, GIB,
    GIGAAM_MODEL_SIZE, LEGACY_GEMMA_FILE, LEGACY_GEMMA_SIZE, LLAMA_BASE_URL, MMPROJ26_FILE,
    MMPROJ26_SIZE, MMPROJ_FILE, MMPROJ_SIZE,
};

/// Select the local provider and repair a bundled managed endpoint before any
/// request can use its persisted model fields. Custom local servers retain
/// their configured model and prep-model values.
pub fn select_local_provider(cfg: &mut crate::config::Config, root: &Path) -> bool {
    let provider_changed = cfg.ai_provider != "local";
    cfg.ai_provider = "local".to_string();
    let model_state_changed = repair_managed_model_state(cfg, root);
    provider_changed || model_state_changed
}

/// Default install root: `%USERPROFILE%\suflyor-local-ai`.
#[must_use]
pub fn default_root() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("suflyor-local-ai")
}

/// Persist a successfully started managed or user-selected model.
pub fn apply_llama_choice(
    cfg: &mut crate::config::Config,
    root: &Path,
    choice: &ManagedLlamaChoice,
) {
    cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();
    cfg.ai_local_quality = false;
    cfg.ai_local_prep_model.clear();
    if let (Some(path), Some(alias)) = (
        valid_custom_choice_path(choice),
        custom_choice_alias(choice),
    ) {
        cfg.ai_local_custom_gguf = path.to_string_lossy().into_owned();
        cfg.ai_local_model = alias;
        cfg.ai_local_vision = false;
        if vision_routes_to_managed_llama(cfg) {
            cfg.vision_provider = "off".to_string();
        }
    } else {
        cfg.ai_local_custom_gguf.clear();
        cfg.ai_local_model = choice.model.file_name().to_string();
        cfg.ai_local_quality = choice.model.is_quality();
        let vision_capable = managed_model_vision_capable(root, choice.model);
        cfg.ai_local_vision &= vision_capable;
        if !vision_capable && vision_routes_to_managed_llama(cfg) {
            cfg.vision_provider = "off".to_string();
        }
    }
}

/// Friendly, compact label for a local model basename.
#[must_use]
pub fn local_model_label(basename: &str) -> String {
    let l = basename.to_ascii_lowercase();
    if l.contains("26b") {
        "Gemma 26B-A4B".to_string()
    } else if l.contains("12b") {
        "Gemma 12B".to_string()
    } else if l.contains("e4b") || l.contains("e2b") || l.contains("4b") {
        "Gemma 4B".to_string()
    } else if l.contains("gemma") {
        "Gemma".to_string()
    } else {
        basename
            .trim_end_matches(".gguf")
            .trim_end_matches(".bin")
            .split(['-', '.', '/', ' ', ':'])
            .find(|s| !s.is_empty())
            .unwrap_or("—")
            .to_string()
    }
}

/// Basename of the candidate profile Settings should display. This is stat-only
/// and safe on the UI thread; the worker-side launcher independently resolves
/// the exact verified GGUF through [`selected_llama_gguf`].
#[must_use]
pub fn active_local_model_name(root: &Path, requested: ManagedModel) -> String {
    effective_managed_model(root, requested)
        .file_name()
        .to_string()
}

pub(super) fn fallback_model_name(root: &Path) -> String {
    fallback_llama_gguf(&root.join("llama.cpp"))
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| GEMMA_FILE.to_string())
}

/// Absolute path the optional 26B primary GGUF lives at (whether or not it
/// has been downloaded yet) under an install `root`.
#[must_use]
pub fn quality_gguf_path(root: &Path) -> PathBuf {
    root.join("llama.cpp").join(GEMMA26_FILE)
}

/// Fast, stat-only 26B presence check for Settings and component rows. Exact
/// SHA-256 validation is intentionally deferred to [`quality_model_verified`],
/// which is called only from worker-side launch/switch paths. Opening Settings
/// must never stream the 10.5-GB model from disk.
#[must_use]
pub fn quality_model_present(root: &Path) -> bool {
    file_has_expected_size(&quality_gguf_path(root), GEMMA26_SIZE)
}

#[must_use]
pub fn legacy_model_present(root: &Path) -> bool {
    legacy_gguf_complete(&root.join("llama.cpp").join(LEGACY_GEMMA_FILE))
}

#[must_use]
pub fn fallback_model_present(root: &Path) -> bool {
    file_has_expected_size(&root.join("llama.cpp").join(GEMMA_FILE), GEMMA_SIZE)
}

#[must_use]
pub fn managed_model_present(root: &Path, model: ManagedModel) -> bool {
    match model {
        ManagedModel::Legacy4B => legacy_model_present(root),
        ManagedModel::Fallback12B => fallback_model_present(root),
        ManagedModel::Primary26B => quality_model_present(root),
    }
}

/// Resolve a requested model using only file metadata, safe for Settings.
#[must_use]
pub fn effective_managed_model(root: &Path, requested: ManagedModel) -> ManagedModel {
    if managed_model_present(root, requested) {
        requested
    } else if fallback_model_present(root) {
        ManagedModel::Fallback12B
    } else if legacy_model_present(root) {
        ManagedModel::Legacy4B
    } else {
        ManagedModel::Fallback12B
    }
}

pub(super) fn effective_verified_managed_model(
    root: &Path,
    requested: ManagedModel,
) -> ManagedModel {
    if requested == ManagedModel::Primary26B && quality_model_verified(root) {
        return requested;
    }
    if requested != ManagedModel::Primary26B && managed_model_present(root, requested) {
        return requested;
    }
    if fallback_model_present(root) {
        ManagedModel::Fallback12B
    } else if legacy_model_present(root) {
        ManagedModel::Legacy4B
    } else {
        ManagedModel::Fallback12B
    }
}

/// Resolve a persisted primary preference to the candidate profile displayed by
/// Settings. This is intentionally stat-only; worker launch paths use
/// [`effective_verified_local_quality`] before loading the primary.
#[must_use]
pub fn effective_local_quality(root: &Path, requested_quality: bool) -> bool {
    requested_quality && quality_model_present(root)
}

/// Repair persisted bundled-model state without touching custom local servers.
/// Returns `true` when the caller must save the config. This is used at boot and
/// when switching back to Suflyor's endpoint so a vanished/partial primary
/// cannot leave a stale model or prep-model id in requests. Same-size integrity
/// failures are repaired by [`repair_managed_model_state_after_verification`].
pub fn repair_managed_model_state(cfg: &mut crate::config::Config, root: &Path) -> bool {
    if !is_managed_llama_endpoint(&cfg.ai_local_base_url) {
        return false;
    }
    let custom_was_set = !cfg.ai_local_custom_gguf.trim().is_empty();
    if let Some(changed) = repair_valid_custom_model_state(cfg, root) {
        return changed;
    }
    cfg.ai_local_custom_gguf.clear();
    let requested = ManagedModel::from_config(&cfg.ai_local_model, cfg.ai_local_quality);
    custom_was_set
        | repair_managed_model_state_for_model(cfg, root, effective_managed_model(root, requested))
}

/// Worker-only version of [`repair_managed_model_state`]. It is called after a
/// launch attempt, so persistence records the 12B fallback if the exact 26B
/// SHA-256 review rejected a same-size replacement.
pub fn repair_managed_model_state_after_verification(
    cfg: &mut crate::config::Config,
    root: &Path,
) -> bool {
    if !is_managed_llama_endpoint(&cfg.ai_local_base_url) {
        return false;
    }
    let custom_was_set = !cfg.ai_local_custom_gguf.trim().is_empty();
    if let Some(changed) = repair_valid_custom_model_state(cfg, root) {
        return changed;
    }
    cfg.ai_local_custom_gguf.clear();
    let requested = ManagedModel::from_config(&cfg.ai_local_model, cfg.ai_local_quality);
    custom_was_set
        | repair_managed_model_state_for_model(
            cfg,
            root,
            effective_verified_managed_model(root, requested),
        )
}

/// Whether the configured local text endpoint may safely receive an image
/// attachment. Managed profiles are checked from their actual selected model
/// and matching projector state instead of trusting the persisted UI flag.
#[must_use]
pub fn local_vision_available(cfg: &crate::config::Config, root: &Path) -> bool {
    !is_managed_llama_endpoint(&cfg.ai_local_base_url)
        || (cfg.ai_local_custom_gguf.trim().is_empty()
            && managed_model_vision_capable(
                root,
                effective_managed_model(
                    root,
                    ManagedModel::from_config(&cfg.ai_local_model, cfg.ai_local_quality),
                ),
            ))
}

#[must_use]
pub fn local_vision_enabled(cfg: &crate::config::Config, root: &Path) -> bool {
    cfg.ai_local_vision && local_vision_available(cfg, root)
}

/// Apply the local-model Vision toggle and keep F8's route in sync.
pub fn set_local_vision(cfg: &mut crate::config::Config, root: &Path, enabled: bool) {
    cfg.ai_local_vision = enabled;
    if enabled {
        cfg.vision_provider = "same".to_string();
    } else if cfg.vision_provider == "same" {
        cfg.vision_provider = "off".to_string();
    }
    repair_managed_model_state(cfg, root);
}

pub(super) fn repair_valid_custom_model_state(
    cfg: &mut crate::config::Config,
    root: &Path,
) -> Option<bool> {
    let path = valid_custom_gguf_path(&cfg.ai_local_custom_gguf)?;
    let alias = path.file_name()?.to_string_lossy().into_owned();
    let changed = cfg.ai_local_base_url != LLAMA_BASE_URL
        || cfg.ai_local_quality
        || cfg.ai_local_model != alias
        || !cfg.ai_local_prep_model.is_empty()
        || cfg.ai_local_vision
        || (vision_routes_to_managed_llama(cfg) && cfg.vision_provider != "off");
    let choice = ManagedLlamaChoice::for_custom(
        path,
        LocalContextPreset::from_config(&cfg.ai_local_context),
    );
    apply_llama_choice(cfg, root, &choice);
    Some(changed)
}

pub(super) fn repair_managed_model_state_for_model(
    cfg: &mut crate::config::Config,
    root: &Path,
    model: ManagedModel,
) -> bool {
    let model_name = model.file_name().to_string();
    let quality = model.is_quality();
    let vision_capable = managed_model_vision_capable(root, model);
    let local_vision = cfg.ai_local_vision && vision_capable;
    let vision_provider = if !vision_capable && vision_routes_to_managed_llama(cfg) {
        "off".to_string()
    } else {
        cfg.vision_provider.clone()
    };
    let changed = cfg.ai_local_base_url != LLAMA_BASE_URL
        || cfg.ai_local_quality != quality
        || cfg.ai_local_model != model_name
        || !cfg.ai_local_prep_model.is_empty()
        || cfg.ai_local_vision != local_vision
        || cfg.vision_provider != vision_provider;
    // The managed server is launched on 127.0.0.1. Canonicalise legacy
    // localhost/[::1] spellings so persisted requests use the same listener.
    cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();
    cfg.ai_local_quality = quality;
    cfg.ai_local_model = model_name;
    cfg.ai_local_prep_model.clear();
    cfg.ai_local_vision = local_vision;
    cfg.vision_provider = vision_provider;
    changed
}

/// True when either the current 12B fallback or the legacy 4B fallback is
/// complete. The latter remains launchable only to preserve an existing install
/// until the user runs the new installer.
#[must_use]
pub fn base_model_present(root: &Path) -> bool {
    let llama_dir = root.join("llama.cpp");
    file_has_expected_size(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE)
        || legacy_gguf_complete(&llama_dir.join(LEGACY_GEMMA_FILE))
}

/// The conventional GigaAM model directory under the local-AI root
/// (`<root>/gigaam-v3`) — the SAME location the installer writes to. The
/// readiness API uses this when `config.stt_gigaam_dir` is unset, so it agrees
/// with where a fresh install lands (single source of truth for the path).
#[must_use]
pub fn gigaam_default_dir(root: &Path) -> PathBuf {
    root.join("gigaam-v3")
}

/// True when a complete GigaAM model lives in `dir` (`model.int8.onnx` present
/// at the pinned size). Mirrors the installer's own "needs download?" size check
/// so the readiness API can't disagree with it; a truncated file reads as absent.
#[must_use]
pub fn gigaam_model_present(dir: &Path) -> bool {
    file_len(&dir.join("model.int8.onnx")) >= GIGAAM_MODEL_SIZE
}

#[must_use]
pub fn quality_vision_present(root: &Path) -> bool {
    file_has_expected_size(&root.join("llama.cpp").join(MMPROJ26_FILE), MMPROJ26_SIZE)
}

#[must_use]
pub fn quality_vision_supported(root: &Path) -> bool {
    llama_build_supports_26b(&root.join("llama.cpp"))
}

/// Resource text for the selected endpoint/model. The only numbers shown are
/// the owner-approved hardware matrix and exact disk sizes. Vision memory is
/// intentionally explicit as unknown for both bundled models.
#[must_use]
pub fn local_model_resource_warning(root: &Path, base_url: &str, model_id: &str) -> String {
    if !is_managed_llama_endpoint(base_url) {
        return "[!] Требования к памяти выбранной внешней модели неизвестны.".to_string();
    }
    let lower = model_id.to_ascii_lowercase();
    if lower.contains("26b-a4b") {
        let profile = detected_hardware_model_profile(false);
        let matrix = match profile {
            HardwareModelProfile::Primary26Vram8 => "профиль 8 ГБ VRAM / 32+ ГБ RAM",
            HardwareModelProfile::Primary26Vram12 => "профиль 12 ГБ VRAM / 24+ ГБ RAM",
            HardwareModelProfile::Primary26Vram16 => "профиль 16 ГБ VRAM / 32+ ГБ RAM",
            HardwareModelProfile::Unknown | HardwareModelProfile::Fallback12B => {
                "профиль железа не подтверждён"
            }
        };
        format!(
            "[!] Gemma 26B-A4B: {:.1} GiB на диске; {matrix}. Память для vision: неизвестно.",
            GEMMA26_SIZE as f64 / GIB as f64
        )
    } else if lower.contains("12b") {
        format!(
            "[!] Gemma 12B QAT fallback: {:.1} GiB на диске. Матрица 8 ГБ VRAM / 16-31 ГБ RAM подтверждена владельцем. Память для vision: неизвестно.",
            GEMMA_SIZE as f64 / GIB as f64
        )
    } else if lower.contains("e4b") || lower.contains("4b") {
        format!(
            "[!] Legacy Gemma 4B: {:.1} GiB на диске. Память для vision: неизвестно.",
            LEGACY_GEMMA_SIZE as f64 / GIB as f64
        )
    } else if model_id.trim().is_empty() && base_model_present(root) {
        local_model_resource_warning(root, base_url, &fallback_model_name(root))
    } else {
        "[!] Требования к памяти выбранной локальной модели неизвестны.".to_string()
    }
}

/// Pick which llama GGUF to load: the 26B only when requested and complete;
/// otherwise the always-installed 12B fallback.
/// Centralised so `ensure_servers` and `install`'s launch agree. Does the disk
/// check then defers the choice to the pure [`pick_llama_gguf`] (unit-tested
/// without materialising a 6 GB file).
pub(super) fn selected_llama_gguf(llama_dir: &Path, model: ManagedModel) -> PathBuf {
    match model {
        ManagedModel::Primary26B => {
            // Selection is a worker-only launch boundary. The exact pinned hash
            // is rechecked here (or served from the matching metadata cache).
            let present = cached_pinned_file_matches(
                &llama_dir.join(GEMMA26_FILE),
                GEMMA26_SIZE,
                GEMMA26_SHA256,
            );
            pick_llama_gguf(llama_dir, model, present)
        }
        ManagedModel::Legacy4B => pick_llama_gguf(
            llama_dir,
            model,
            legacy_gguf_complete(&llama_dir.join(LEGACY_GEMMA_FILE)),
        ),
        ManagedModel::Fallback12B => pick_llama_gguf(
            llama_dir,
            model,
            file_has_expected_size(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE),
        ),
    }
}

/// Prefer the current 12B fallback, but keep the previous 4B artifact
/// launchable during an in-place upgrade that has not downloaded 12B yet.
pub(super) fn fallback_llama_gguf(llama_dir: &Path) -> PathBuf {
    complete_fallback_llama_gguf(llama_dir).unwrap_or_else(|| llama_dir.join(GEMMA_FILE))
}

/// Complete fallback GGUF available for a launch-time model-load check. The
/// current 12B is preferred, while a complete legacy 4B remains supported until
/// the user installs 12B. `None` means a binary-only verification is the best
/// safe check available.
pub(super) fn complete_fallback_llama_gguf(llama_dir: &Path) -> Option<PathBuf> {
    let current = llama_dir.join(GEMMA_FILE);
    if file_has_expected_size(&current, GEMMA_SIZE) {
        Some(current)
    } else {
        let legacy = llama_dir.join(LEGACY_GEMMA_FILE);
        legacy_gguf_complete(&legacy).then_some(legacy)
    }
}

/// Pure model-choice rule (no I/O): an explicit model only when complete.
pub(super) fn pick_llama_gguf(
    llama_dir: &Path,
    model: ManagedModel,
    target_present: bool,
) -> PathBuf {
    if target_present {
        llama_dir.join(model.file_name())
    } else {
        fallback_llama_gguf(llama_dir)
    }
}

/// The installed llama.cpp release build number (the `bNNNN` tag), read from the
/// `.llama-build` stamp `install`/the engine-updater write next to the binaries.
/// `None` when the stamp is missing/unparseable (an old install) → treated as
/// too-old by the gemma4uv gate (so we stay safe, never crash).
pub(super) fn installed_llama_build(llama_dir: &Path) -> Option<u32> {
    parse_build_tag(&std::fs::read_to_string(llama_dir.join(".llama-build")).ok()?)
}

/// Parse a llama.cpp build tag (`b9626`, or a bare `9626`) into its number.
/// `None` for anything unparseable (an old/garbage stamp) → callers treat that
/// as "too old", staying on the safe side of the gemma4uv gate.
pub(super) fn parse_build_tag(tag: &str) -> Option<u32> {
    tag.trim().trim_start_matches('b').parse::<u32>().ok()
}

/// Record which llama.cpp build is installed (the `bNNNN` tag, e.g. `b9626`).
/// Best-effort: a write failure just leaves the gate conservative (12B vision
/// stays off until the next successful install/update). Trims to keep the stamp
/// a clean single token regardless of what the GitHub API returned.
pub(super) fn write_build_stamp(llama_dir: &Path, tag: &str) {
    let tag = tag.trim();
    if !tag.is_empty() {
        let _ = std::fs::write(llama_dir.join(".llama-build"), tag);
    }
}

/// True if the installed llama.cpp is new enough to load the 12B's "gemma4uv"
/// projector (build >= [`GEMMA4UV_MIN_BUILD`]). A missing/old stamp → false.
pub(super) fn llama_build_supports_gemma4uv(llama_dir: &Path) -> bool {
    installed_llama_build(llama_dir).is_some_and(|b| b >= GEMMA4UV_MIN_BUILD)
}

pub(super) fn llama_build_supports_26b(llama_dir: &Path) -> bool {
    installed_llama_build(llama_dir).is_some_and(|build| build >= GEMMA26_MIN_BUILD)
}

/// The matching vision projector to attach for `gguf`, if present and loadable.
pub(super) fn mmproj_for_model(llama_dir: &Path, gguf: &Path) -> Option<PathBuf> {
    let name = gguf.file_name().and_then(|n| n.to_str())?;
    let (file, size, supported) = match name {
        GEMMA_FILE => (
            MMPROJ_FILE,
            MMPROJ_SIZE,
            llama_build_supports_gemma4uv(llama_dir),
        ),
        GEMMA26_FILE => (
            MMPROJ26_FILE,
            MMPROJ26_SIZE,
            llama_build_supports_26b(llama_dir),
        ),
        _ => return None,
    };
    let proj = llama_dir.join(file);
    (supported && file_len(&proj) == size).then_some(proj)
}

/// Whether the effective managed profile can accept screenshots on the server
/// Suflyor launches.
pub(super) fn managed_model_vision_capable(root: &Path, model: ManagedModel) -> bool {
    if !managed_model_present(root, model) {
        return false;
    }
    let llama_dir = root.join("llama.cpp");
    mmproj_for_model(&llama_dir, &llama_dir.join(model.file_name())).is_some()
}

/// True when F8's configured route resolves back to Suflyor's managed text
/// server. Besides `same`, a `local` vision provider with an empty (or explicit
/// managed) URL inherits `ai_local_base_url` and is the same unsafe route for a
/// text-only profile.
pub(super) fn vision_routes_to_managed_llama(cfg: &crate::config::Config) -> bool {
    match cfg.vision_provider.as_str() {
        "same" => true,
        "local" => {
            let base_url = if cfg.vision_local_base_url.trim().is_empty() {
                &cfg.ai_local_base_url
            } else {
                &cfg.vision_local_base_url
            };
            is_managed_llama_endpoint(base_url)
        }
        _ => false,
    }
}
