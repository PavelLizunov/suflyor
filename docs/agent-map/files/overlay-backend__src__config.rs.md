---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a9ff4ad99048"
source_path: "overlay-backend/src/config.rs"
batch_id: "B09"
total_lines: 2074
symbols_count: 72
review_state: validated
---

# File Map: `overlay-backend/src/config.rs`

- **Batch:** B09
- **Physical Lines:** 2074
- **Coverage:** 2074/2074 lines (100%)

## Types & Structures (9)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `SttBackendCfg` | L1075 | pub |
| struct | `Config` | L31 | pub |
| struct | `means` | L426 | private |
| struct | `Snippet` | L502 | pub |
| struct | `ContextProfile` | L512 | pub |
| struct | `ReadinessItem` | L716 | pub |
| struct | `ReadinessReport` | L727 | pub |
| struct | `PreviewGroup` | L1805 | pub |
| struct | `ServerSettingsPreview` | L1845 | pub |

## Symbols & Routines (72)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `ui_is_ru` | L525 | `fn ui_is_ru(&self) -> bool` |
| function | `active_profile_index` | L532 | `fn active_profile_index(&self) -> Option<usize>` |
| function | `select_profile` | L539 | `fn select_profile(&mut self, idx: usize) -> ()` |
| function | `add_profile` | L557 | `fn add_profile(&mut self, name: &str) -> Option<usize>` |
| function | `rename_active_profile` | L573 | `fn rename_active_profile(&mut self, new_name: &str) -> bool` |
| function | `delete_active_profile` | L589 | `fn delete_active_profile(&mut self) -> ()` |
| function | `save_active_context` | L609 | `fn save_active_context(&mut self, text: &str) -> ()` |
| function | `defaults` | L618 | `fn defaults() -> Self` |
| function | `ai_endpoint` | L746 | `fn ai_endpoint(&self, prep: bool) -> AiEndpoint` |
| function | `ai_endpoint_cloud` | L845 | `fn ai_endpoint_cloud(&self) -> AiEndpoint` |
| function | `same_text_model_accepts_images_declared` | L866 | `fn same_text_model_accepts_images_declared(&self) -> bool` |
| function | `vision_endpoint` | L879 | `fn vision_endpoint(&self) -> Option<AiEndpoint>` |
| function | `readiness` | L957 | `fn readiness(&self) -> ReadinessReport` |
| function | `stt_backend` | L1092 | `fn stt_backend(&self) -> SttBackendCfg` |
| function | `stt_is_local` | L1112 | `fn stt_is_local(&self) -> bool` |
| function | `default_ai_provider` | L1117 | `fn default_ai_provider() -> String` |
| function | `default_ai_mlx_model` | L1121 | `fn default_ai_mlx_model() -> String` |
| function | `default_vision_mlx_model` | L1125 | `fn default_vision_mlx_model() -> String` |
| function | `default_openai_base_url` | L1129 | `fn default_openai_base_url() -> String` |
| function | `default_openai_model` | L1133 | `fn default_openai_model() -> String` |
| function | `default_anthropic_base_url` | L1137 | `fn default_anthropic_base_url() -> String` |
| function | `default_anthropic_model` | L1141 | `fn default_anthropic_model() -> String` |
| function | `protected_provider_secret` | L1145 | `fn protected_provider_secret(slot: SecretSlot) -> String` |
| function | `default_vision_provider` | L1153 | `fn default_vision_provider() -> String` |
| function | `default_ai_local_base_url` | L1157 | `fn default_ai_local_base_url() -> String` |
| function | `default_ai_local_context` | L1163 | `fn default_ai_local_context() -> String` |
| function | `default_stt_provider` | L1167 | `fn default_stt_provider() -> String` |
| function | `default_stt_gigaam_dir` | L1175 | `fn default_stt_gigaam_dir() -> String` |
| function | `default_stt_gigaam_gpu` | L1181 | `fn default_stt_gigaam_gpu() -> bool` |
| function | `default_stt_whisper_url` | L1185 | `fn default_stt_whisper_url() -> String` |
| function | `default_stt_whisper_model` | L1189 | `fn default_stt_whisper_model() -> String` |
| function | `default_tile_body_opacity` | L1193 | `fn default_tile_body_opacity() -> f32` |
| function | `default_post_meeting_debrief_enabled` | L1197 | `fn default_post_meeting_debrief_enabled() -> bool` |
| function | `default_record_audio_enabled` | L1201 | `fn default_record_audio_enabled() -> bool` |
| function | `default_record_retention_sessions` | L1208 | `fn default_record_retention_sessions() -> u32` |
| function | `default_journal_retention_sessions` | L1212 | `fn default_journal_retention_sessions() -> u32` |
| function | `default_journal_max_total_mb` | L1216 | `fn default_journal_max_total_mb() -> u32` |
| function | `default_record_max_total_mb` | L1220 | `fn default_record_max_total_mb() -> u32` |
| function | `default_session_archive_enabled` | L1224 | `fn default_session_archive_enabled() -> bool` |
| function | `default_max_session_cost_usd` | L1228 | `fn default_max_session_cost_usd() -> f64` |
| function | `default_detector_skip_mic` | L1242 | `fn default_detector_skip_mic() -> bool` |
| function | `default_ui_language` | L1246 | `fn default_ui_language() -> String` |
| function | `default_hermes_bridge_port` | L1253 | `fn default_hermes_bridge_port() -> u16` |
| function | `default_hermes_bridge_host` | L1259 | `fn default_hermes_bridge_host() -> String` |
| function | `default_hermes_api_url` | L1264 | `fn default_hermes_api_url() -> String` |
| function | `default_tile_font_size` | L1270 | `fn default_tile_font_size() -> u32` |
| function | `default_trigger_keywords` | L1287 | `fn default_trigger_keywords() -> String` |
| function | `config_path` | L1353 | `fn config_path() -> Result<PathBuf>` |
| function | `parse_config_bytes` | L1366 | `fn parse_config_bytes(raw: &[u8]) -> Result<Config>` |
| function | `preserve_corrupt_config` | L1390 | `fn preserve_corrupt_config(path: &std::path::Path) -> ()` |
| function | `load` | L1402 | `fn load() -> Config` |
| function | `migrate_legacy_tts_default` | L1505 | `fn migrate_legacy_tts_default(cfg: &mut Config) -> bool` |
| function | `migrate_legacy_vision_same` | L1515 | `fn migrate_legacy_vision_same(cfg: &mut Config) -> bool` |
| function | `migrate_macos_gigaam_cpu_default` | L1532 | `fn migrate_macos_gigaam_cpu_default(cfg: &mut Config) -> bool` |
| function | `migrate_macos_gigaam_default` | L1540 | `fn migrate_macos_gigaam_default(cfg: &mut Config, managed_ready: bool) -> bool` |
| function | `save_to_path` | L1561 | `fn save_to_path(path: &std::path::Path, cfg: &Config) -> Result<()>` |
| function | `save` | L1622 | `fn save(cfg: &Config) -> Result<()>` |
| function | `secret_redacted` | L1632 | `fn secret_redacted(cfg: &Config) -> Config` |
| function | `export_to` | L1649 | `fn export_to(path: &std::path::Path, cfg: &Config) -> Result<()>` |
| function | `portable_config_bytes` | L1655 | `fn portable_config_bytes(cfg: &Config) -> Result<Vec<u8>>` |
| function | `import_from` | L1669 | `fn import_from(path: &std::path::Path, local_deep_lock: bool) -> Result<Config>` |
| function | `preserve_local_import_state` | L1677 | `fn preserve_local_import_state(mut cfg: Config, local_deep_lock: bool) -> Config` |
| function | `merge_server_settings` | L1696 | `fn merge_server_settings(current: &Config, imported: Config) -> Config` |
| function | `import_server_settings_from` | L1765 | `fn import_server_settings_from(path: &std::path::Path, current: &Config) -> Result<Config>` |
| function | `export_server_settings_to` | L1789 | `fn export_server_settings_to(path: &std::path::Path, cfg: &Config) -> Result<()>` |
| function | `changed` | L1831 | `fn changed(&self) -> bool` |
| function | `mask_host` | L1868 | `fn mask_host(url: &str) -> String` |
| function | `preview_server_settings` | L1940 | `fn preview_server_settings(current: &Config, imported: &Config) -> ServerSettingsPreview` |
| function | `preview_server_settings_from` | L2035 | `fn preview_server_settings_from(path: &std::path::Path, current: &Config,) -> Result<(ServerSettingsPreview, Config)>` |
| function | `apply_server_settings` | L2052 | `fn apply_server_settings(current: &Config, imported: Config) -> Config` |
| function | `shared` | L2062 | `fn shared() -> SharedConfig` |
| function | `shared_from` | L2069 | `fn shared_from(cfg: Config) -> SharedConfig` |

## Configuration Access

- Configuration read at L1371: `// config.json interleaves live secrets (ai_bearer / groq_api_key /`
- Configuration read at L1519: `let replacement = match cfg.ai_provider.as_str() {`
- Configuration read at L1533: `if !cfg!(target_os = "macos") || cfg.config_version >= 2 || !cfg.stt_gigaam_gpu `
- Configuration read at L1942: `let ai_profile = |cfg: &Config| match cfg.ai_provider.as_str() {`
- Configuration read at L1944: `cfg.openai_base_url.clone(),`
- Configuration read at L1945: `cfg.openai_model.clone(),`
- Configuration read at L1957: `cfg.ai_base_url.clone(),`
- Configuration read at L1958: `cfg.ai_model.clone(),`
- Configuration read at L1959: `present(&cfg.ai_bearer),`
