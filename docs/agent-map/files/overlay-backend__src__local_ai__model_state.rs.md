---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_bc57ae5659e2"
source_path: "overlay-backend/src/local_ai/model_state.rs"
batch_id: "B06"
total_lines: 526
symbols_count: 39
review_state: validated
---

# File Map: `overlay-backend/src/local_ai/model_state.rs`

- **Batch:** B06
- **Physical Lines:** 526
- **Coverage:** 526/526 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (39)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `select_local_provider` | L16 | `fn select_local_provider(cfg: &mut crate::config::Config, root: &Path) -> bool` |
| function | `default_root` | L25 | `fn default_root() -> PathBuf` |
| function | `apply_llama_choice` | L31 | `fn apply_llama_choice(cfg: &mut crate::config::Config, root: &Path, choice: &ManagedLlamaChoice,) -> ()` |
| function | `local_model_label` | L63 | `fn local_model_label(basename: &str) -> String` |
| function | `active_local_model_name` | L88 | `fn active_local_model_name(root: &Path, requested: ManagedModel) -> String` |
| function | `fallback_model_name` | L94 | `fn fallback_model_name(root: &Path) -> String` |
| function | `quality_gguf_path` | L104 | `fn quality_gguf_path(root: &Path) -> PathBuf` |
| function | `quality_model_present` | L113 | `fn quality_model_present(root: &Path) -> bool` |
| function | `legacy_model_present` | L118 | `fn legacy_model_present(root: &Path) -> bool` |
| function | `fallback_model_present` | L123 | `fn fallback_model_present(root: &Path) -> bool` |
| function | `managed_model_present` | L128 | `fn managed_model_present(root: &Path, model: ManagedModel) -> bool` |
| function | `effective_managed_model` | L138 | `fn effective_managed_model(root: &Path, requested: ManagedModel) -> ManagedModel` |
| function | `effective_verified_managed_model` | L150 | `fn effective_verified_managed_model(root: &Path, requested: ManagedModel,) -> ManagedModel` |
| function | `effective_local_quality` | L173 | `fn effective_local_quality(root: &Path, requested_quality: bool) -> bool` |
| function | `repair_managed_model_state` | L182 | `fn repair_managed_model_state(cfg: &mut crate::config::Config, root: &Path) -> bool` |
| function | `repair_managed_model_state_after_verification` | L199 | `fn repair_managed_model_state_after_verification(cfg: &mut crate::config::Config, root: &Path,) -> bool` |
| function | `local_vision_available` | L224 | `fn local_vision_available(cfg: &crate::config::Config, root: &Path) -> bool` |
| function | `local_vision_enabled` | L237 | `fn local_vision_enabled(cfg: &crate::config::Config, root: &Path) -> bool` |
| function | `set_local_vision` | L242 | `fn set_local_vision(cfg: &mut crate::config::Config, root: &Path, enabled: bool) -> ()` |
| function | `repair_valid_custom_model_state` | L252 | `fn repair_valid_custom_model_state(cfg: &mut crate::config::Config, root: &Path,) -> Option<bool>` |
| function | `repair_managed_model_state_for_model` | L272 | `fn repair_managed_model_state_for_model(cfg: &mut crate::config::Config, root: &Path, model: ManagedModel,) -> bool` |
| function | `base_model_present` | L307 | `fn base_model_present(root: &Path) -> bool` |
| function | `gigaam_default_dir` | L318 | `fn gigaam_default_dir(root: &Path) -> PathBuf` |
| function | `gigaam_model_present` | L326 | `fn gigaam_model_present(dir: &Path) -> bool` |
| function | `quality_vision_present` | L331 | `fn quality_vision_present(root: &Path) -> bool` |
| function | `quality_vision_supported` | L336 | `fn quality_vision_supported(root: &Path) -> bool` |
| function | `local_model_resource_warning` | L344 | `fn local_model_resource_warning(root: &Path, base_url: &str, model_id: &str) -> String` |
| function | `selected_llama_gguf` | L385 | `fn selected_llama_gguf(llama_dir: &Path, model: ManagedModel) -> PathBuf` |
| function | `fallback_llama_gguf` | L412 | `fn fallback_llama_gguf(llama_dir: &Path) -> PathBuf` |
| function | `complete_fallback_llama_gguf` | L420 | `fn complete_fallback_llama_gguf(llama_dir: &Path) -> Option<PathBuf>` |
| function | `pick_llama_gguf` | L431 | `fn pick_llama_gguf(llama_dir: &Path, model: ManagedModel, target_present: bool,) -> PathBuf` |
| function | `installed_llama_build` | L447 | `fn installed_llama_build(llama_dir: &Path) -> Option<u32>` |
| function | `parse_build_tag` | L454 | `fn parse_build_tag(tag: &str) -> Option<u32>` |
| function | `write_build_stamp` | L462 | `fn write_build_stamp(llama_dir: &Path, tag: &str) -> ()` |
| function | `llama_build_supports_gemma4uv` | L471 | `fn llama_build_supports_gemma4uv(llama_dir: &Path) -> bool` |
| function | `llama_build_supports_26b` | L475 | `fn llama_build_supports_26b(llama_dir: &Path) -> bool` |
| function | `mmproj_for_model` | L480 | `fn mmproj_for_model(llama_dir: &Path, gguf: &Path) -> Option<PathBuf>` |
| function | `managed_model_vision_capable` | L501 | `fn managed_model_vision_capable(root: &Path, model: ManagedModel) -> bool` |
| function | `vision_routes_to_managed_llama` | L513 | `fn vision_routes_to_managed_llama(cfg: &crate::config::Config) -> bool` |

## Configuration Access

- Configuration read at L17: `let provider_changed = cfg.ai_provider != "local";`
- Configuration read at L18: `cfg.ai_provider = "local".to_string();`
- Configuration read at L36: `cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();`
- Configuration read at L37: `cfg.ai_local_quality = false;`
- Configuration read at L38: `cfg.ai_local_prep_model.clear();`
- Configuration read at L43: `cfg.ai_local_custom_gguf = path.to_string_lossy().into_owned();`
- Configuration read at L44: `cfg.ai_local_model = alias;`
- Configuration read at L45: `cfg.ai_local_vision = false;`
- Configuration read at L50: `cfg.ai_local_custom_gguf.clear();`
- Configuration read at L51: `cfg.ai_local_model = choice.model.file_name().to_string();`
- Configuration read at L52: `cfg.ai_local_quality = choice.model.is_quality();`
- Configuration read at L54: `cfg.ai_local_vision &= vision_capable;`
- Configuration read at L183: `if !is_managed_llama_endpoint(&cfg.ai_local_base_url) {`
- Configuration read at L186: `let custom_was_set = !cfg.ai_local_custom_gguf.trim().is_empty();`
- Configuration read at L190: `cfg.ai_local_custom_gguf.clear();`
- Configuration read at L191: `let requested = ManagedModel::from_config(&cfg.ai_local_model, cfg.ai_local_qual`
- Configuration read at L203: `if !is_managed_llama_endpoint(&cfg.ai_local_base_url) {`
- Configuration read at L206: `let custom_was_set = !cfg.ai_local_custom_gguf.trim().is_empty();`
- Configuration read at L210: `cfg.ai_local_custom_gguf.clear();`
- Configuration read at L211: `let requested = ManagedModel::from_config(&cfg.ai_local_model, cfg.ai_local_qual`
- Configuration read at L225: `!is_managed_llama_endpoint(&cfg.ai_local_base_url)`
- Configuration read at L226: `|| (cfg.ai_local_custom_gguf.trim().is_empty()`
- Configuration read at L231: `ManagedModel::from_config(&cfg.ai_local_model, cfg.ai_local_quality),`
- Configuration read at L238: `cfg.ai_local_vision && local_vision_available(cfg, root)`
- Configuration read at L243: `cfg.ai_local_vision = enabled;`
- Configuration read at L256: `let path = valid_custom_gguf_path(&cfg.ai_local_custom_gguf)?;`
- Configuration read at L258: `let changed = cfg.ai_local_base_url != LLAMA_BASE_URL`
- Configuration read at L259: `|| cfg.ai_local_quality`
- Configuration read at L260: `|| cfg.ai_local_model != alias`
- Configuration read at L261: `|| !cfg.ai_local_prep_model.is_empty()`
- Configuration read at L262: `|| cfg.ai_local_vision`
- Configuration read at L266: `LocalContextPreset::from_config(&cfg.ai_local_context),`
- Configuration read at L280: `let local_vision = cfg.ai_local_vision && vision_capable;`
- Configuration read at L286: `let changed = cfg.ai_local_base_url != LLAMA_BASE_URL`
- Configuration read at L287: `|| cfg.ai_local_quality != quality`
- Configuration read at L288: `|| cfg.ai_local_model != model_name`
- Configuration read at L289: `|| !cfg.ai_local_prep_model.is_empty()`
- Configuration read at L290: `|| cfg.ai_local_vision != local_vision`
- Configuration read at L294: `cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();`
- Configuration read at L295: `cfg.ai_local_quality = quality;`
- Configuration read at L296: `cfg.ai_local_model = model_name;`
- Configuration read at L297: `cfg.ai_local_prep_model.clear();`
- Configuration read at L298: `cfg.ai_local_vision = local_vision;`
- Configuration read at L518: `&cfg.ai_local_base_url`
