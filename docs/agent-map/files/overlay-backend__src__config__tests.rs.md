---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4eda19a34a8e"
source_path: "overlay-backend/src/config/tests.rs"
batch_id: "B09"
total_lines: 1911
symbols_count: 82
review_state: validated
---

# File Map: `overlay-backend/src/config/tests.rs`

- **Batch:** B09
- **Physical Lines:** 1911
- **Coverage:** 1911/1911 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `should` | L469 | private |

## Symbols & Routines (82)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `merge_server_settings_takes_servers_keeps_locals` | L11 | `fn merge_server_settings_takes_servers_keeps_locals() -> ()` |
| function | `imported_with_secret_tokens` | L137 | `fn imported_with_secret_tokens() -> Config` |
| function | `export_server_settings_writes_servers_only_no_locals` | L166 | `fn export_server_settings_writes_servers_only_no_locals() -> ()` |
| function | `preview_server_settings_is_redacted_and_diffs` | L237 | `fn preview_server_settings_is_redacted_and_diffs() -> ()` |
| function | `mask_host_blanks_host_keeps_scheme_port_path` | L315 | `fn mask_host_blanks_host_keeps_scheme_port_path() -> ()` |
| function | `apply_server_settings_keeps_local_gigaam_dir` | L331 | `fn apply_server_settings_keeps_local_gigaam_dir() -> ()` |
| function | `readiness_reflects_active_providers` | L350 | `fn readiness_reflects_active_providers() -> ()` |
| function | `config_save_load_roundtrip` | L434 | `fn config_save_load_roundtrip() -> ()` |
| function | `config_partial_json_uses_serde_defaults` | L472 | `fn config_partial_json_uses_serde_defaults() -> ()` |
| function | `config_defaults_stamp_current_schema_version` | L485 | `fn config_defaults_stamp_current_schema_version() -> ()` |
| function | `config_pre_versioning_json_reads_as_zero` | L493 | `fn config_pre_versioning_json_reads_as_zero() -> ()` |
| function | `preserve_corrupt_config_renames_aside_keeping_bytes` | L503 | `fn preserve_corrupt_config_renames_aside_keeping_bytes() -> ()` |
| function | `parse_error_never_echoes_the_offending_token` | L533 | `fn parse_error_never_echoes_the_offending_token() -> ()` |
| function | `config_with_utf8_bom_is_stripped_not_reset` | L558 | `fn config_with_utf8_bom_is_stripped_not_reset() -> ()` |
| function | `config_empty_object_yields_all_defaults` | L576 | `fn config_empty_object_yields_all_defaults() -> ()` |
| function | `secret_redacted_blanks_every_secret_keeps_the_rest` | L587 | `fn secret_redacted_blanks_every_secret_keeps_the_rest() -> ()` |
| function | `v015_retention_fields_default_to_pre_v015_behaviour` | L642 | `fn v015_retention_fields_default_to_pre_v015_behaviour() -> ()` |
| function | `ai_endpoint_defaults_to_cloud` | L666 | `fn ai_endpoint_defaults_to_cloud() -> ()` |
| function | `direct_provider_profiles_are_additive_and_use_native_protocols` | L682 | `fn direct_provider_profiles_are_additive_and_use_native_protocols() -> ()` |
| function | `codex_subscription_profile_round_trips_selected_model_without_secrets` | L714 | `fn codex_subscription_profile_round_trips_selected_model_without_secrets() -> ()` |
| function | `cloud_escalation_preserves_legacy_bridge_but_keeps_direct_protocol` | L767 | `fn cloud_escalation_preserves_legacy_bridge_but_keeps_direct_protocol() -> ()` |
| function | `direct_profile_preview_exposes_no_protected_key_field` | L785 | `fn direct_profile_preview_exposes_no_protected_key_field() -> ()` |
| function | `server_settings_merge_preserves_direct_provider_profile_without_secrets` | L805 | `fn server_settings_merge_preserves_direct_provider_profile_without_secrets() -> ()` |
| function | `codex_model_is_in_redacted_preview_and_server_settings_merge` | L820 | `fn codex_model_is_in_redacted_preview_and_server_settings_merge() -> ()` |
| function | `legacy_server_import_does_not_erase_explicit_codex_models` | L844 | `fn legacy_server_import_does_not_erase_explicit_codex_models() -> ()` |
| function | `fresh_and_untouched_legacy_tts_select_tera_ru_f1_only` | L858 | `fn fresh_and_untouched_legacy_tts_select_tera_ru_f1_only() -> ()` |
| function | `vision_endpoint_off_is_none` | L878 | `fn vision_endpoint_off_is_none() -> ()` |
| function | `vision_endpoint_same_honors_external_local_declaration` | L885 | `fn vision_endpoint_same_honors_external_local_declaration() -> ()` |
| function | `vision_endpoint_same_rejects_unknown_text_provider` | L904 | `fn vision_endpoint_same_rejects_unknown_text_provider() -> ()` |
| function | `vision_endpoint_same_does_not_claim_implicit_codex_vision` | L912 | `fn vision_endpoint_same_does_not_claim_implicit_codex_vision() -> ()` |
| function | `vision_endpoint_same_uses_only_catalog_confirmed_codex_model` | L921 | `fn vision_endpoint_same_uses_only_catalog_confirmed_codex_model() -> ()` |
| function | `legacy_same_provider_migrates_without_unverified_image_claims` | L940 | `fn legacy_same_provider_migrates_without_unverified_image_claims() -> ()` |
| function | `vision_endpoint_direct_providers_reuse_their_provider_profiles` | L960 | `fn vision_endpoint_direct_providers_reuse_their_provider_profiles() -> ()` |
| function | `vision_endpoint_codex_requires_an_explicit_image_capable_selection` | L978 | `fn vision_endpoint_codex_requires_an_explicit_image_capable_selection() -> ()` |
| function | `vision_endpoint_cloud_falls_back_to_text_bridge_and_sonnet` | L1001 | `fn vision_endpoint_cloud_falls_back_to_text_bridge_and_sonnet() -> ()` |
| function | `vision_endpoint_cloud_uses_explicit_fields_when_set` | L1021 | `fn vision_endpoint_cloud_uses_explicit_fields_when_set() -> ()` |
| function | `vision_endpoint_local_falls_back_to_text_local` | L1041 | `fn vision_endpoint_local_falls_back_to_text_local() -> ()` |
| function | `vision_endpoint_default_provider_is_cloud` | L1053 | `fn vision_endpoint_default_provider_is_cloud() -> ()` |
| function | `mlx_defaults_are_additive_and_fail_closed_until_owned_runtime_is_ready` | L1060 | `fn mlx_defaults_are_additive_and_fail_closed_until_owned_runtime_is_ready() -> ()` |
| function | `server_merge_copies_only_persisted_mlx_model_choices` | L1099 | `fn server_merge_copies_only_persisted_mlx_model_choices() -> ()` |
| function | `ai_endpoint_local_uses_local_fields_and_prep_fallback` | L1121 | `fn ai_endpoint_local_uses_local_fields_and_prep_fallback() -> ()` |
| function | `ai_endpoint_cloud_always_uses_cloud_bridge_and_prep_model` | L1143 | `fn ai_endpoint_cloud_always_uses_cloud_bridge_and_prep_model() -> ()` |
| function | `stt_backend_uses_the_platform_default` | L1170 | `fn stt_backend_uses_the_platform_default() -> ()` |
| function | `unknown_stt_provider_falls_back_to_cloud` | L1188 | `fn unknown_stt_provider_falls_back_to_cloud() -> ()` |
| function | `stt_backend_gigaam_uses_dir_and_is_local` | L1196 | `fn stt_backend_gigaam_uses_dir_and_is_local() -> ()` |
| function | `stt_backend_whisper_uses_url_bearer_model_and_is_local` | L1208 | `fn stt_backend_whisper_uses_url_bearer_model_and_is_local() -> ()` |
| function | `stt_provider_defaults_from_partial_json` | L1230 | `fn stt_provider_defaults_from_partial_json() -> ()` |
| function | `macos_migrates_the_old_coreml_default_once` | L1253 | `fn macos_migrates_the_old_coreml_default_once() -> ()` |
| function | `macos_migrates_an_empty_saved_gigaam_dir_to_the_managed_path` | L1270 | `fn macos_migrates_an_empty_saved_gigaam_dir_to_the_managed_path() -> ()` |
| function | `macos_promotes_an_unconfigured_cloud_stt_when_managed_gigaam_is_ready` | L1286 | `fn macos_promotes_an_unconfigured_cloud_stt_when_managed_gigaam_is_ready() -> ()` |
| function | `macos_replaces_a_retired_stt_provider_when_managed_gigaam_is_ready` | L1298 | `fn macos_replaces_a_retired_stt_provider_when_managed_gigaam_is_ready() -> ()` |
| function | `config_missing_provider_fields_default_cloud` | L1309 | `fn config_missing_provider_fields_default_cloud() -> ()` |
| function | `new_v002_field_defaults` | L1323 | `fn new_v002_field_defaults() -> ()` |
| function | `pre_v002_config_gets_correct_field_defaults_via_serde` | L1354 | `fn pre_v002_config_gets_correct_field_defaults_via_serde() -> ()` |
| function | `explicit_positive_cost_cap_preserved` | L1381 | `fn explicit_positive_cost_cap_preserved() -> ()` |
| function | `explicit_zero_cost_cap_preserved` | L1394 | `fn explicit_zero_cost_cap_preserved() -> ()` |
| function | `defaults_use_models_present_in_pricing_table` | L1405 | `fn defaults_use_models_present_in_pricing_table() -> ()` |
| function | `trigger_keywords_default_includes_core_devops_terms` | L1428 | `fn trigger_keywords_default_includes_core_devops_terms() -> ()` |
| function | `stealth_defaults_off_for_safer_first_run` | L1449 | `fn stealth_defaults_off_for_safer_first_run() -> ()` |
| function | `auto_tiles_default_on` | L1456 | `fn auto_tiles_default_on() -> ()` |
| function | `malformed_json_parse_errors_caught_gracefully` | L1464 | `fn malformed_json_parse_errors_caught_gracefully() -> ()` |
| function | `wrong_field_type_errors_dont_coerce` | L1477 | `fn wrong_field_type_errors_dont_coerce() -> ()` |
| function | `default_snippets_cover_breadth` | L1488 | `fn default_snippets_cover_breadth() -> ()` |
| function | `default_trigger_keywords_breadth` | L1529 | `fn default_trigger_keywords_breadth() -> ()` |
| function | `default_snippets_present_and_keys_unique` | L1542 | `fn default_snippets_present_and_keys_unique() -> ()` |
| function | `default_snippets_have_content` | L1562 | `fn default_snippets_have_content() -> ()` |
| function | `snippet_serialisation_roundtrip` | L1580 | `fn snippet_serialisation_roundtrip() -> ()` |
| function | `context_profile_serialisation_roundtrip` | L1595 | `fn context_profile_serialisation_roundtrip() -> ()` |
| function | `profile_lifecycle_add_select_rename_delete` | L1613 | `fn profile_lifecycle_add_select_rename_delete() -> ()` |
| function | `add_profile_does_not_clone_active_profile_context` | L1661 | `fn add_profile_does_not_clone_active_profile_context() -> ()` |
| function | `ui_language_is_independent_from_ai_response_language` | L1678 | `fn ui_language_is_independent_from_ai_response_language() -> ()` |
| function | `mask_host_bracketed_ipv6_without_port_is_fully_masked` | L1693 | `fn mask_host_bracketed_ipv6_without_port_is_fully_masked() -> ()` |
| function | `mask_host_keeps_real_ports_and_dns_ipv4` | L1700 | `fn mask_host_keeps_real_ports_and_dns_ipv4() -> ()` |
| function | `mask_host_strips_userinfo_and_redacts_credentials` | L1720 | `fn mask_host_strips_userinfo_and_redacts_credentials() -> ()` |
| function | `mask_host_handles_query_and_fragment_boundaries` | L1741 | `fn mask_host_handles_query_and_fragment_boundaries() -> ()` |
| function | `deep_lock_defaults_false_and_survives_serde_roundtrip` | L1758 | `fn deep_lock_defaults_false_and_survives_serde_roundtrip() -> ()` |
| function | `server_settings_transfer_never_carries_deep_lock` | L1780 | `fn server_settings_transfer_never_carries_deep_lock() -> ()` |
| function | `apply_server_settings_keeps_local_deep_lock` | L1821 | `fn apply_server_settings_keeps_local_deep_lock() -> ()` |
| function | `full_profile_import_preserves_local_deep_lock` | L1832 | `fn full_profile_import_preserves_local_deep_lock() -> ()` |
| function | `config_persists_no_tray_hidden_state` | L1852 | `fn config_persists_no_tray_hidden_state() -> ()` |
| function | `legacy_config_loads_and_startup_stays_visible` | L1874 | `fn legacy_config_loads_and_startup_stays_visible() -> ()` |
| function | `config_save_sets_unix_mode_0600` | L1893 | `fn config_save_sets_unix_mode_0600() -> ()` |

## Configuration Access

- Configuration read at L179: `cfg.system_audio_device = Some("My Loopback".into());`
- Configuration read at L476: `assert_eq!(cfg.ai_model, "claude-old");`
- Configuration read at L478: `assert_eq!(cfg.ai_bearer, "");`
- Configuration read at L579: `assert_eq!(cfg.ai_bearer, "");`
- Configuration read at L580: `assert_eq!(cfg.ai_model, "");`
- Configuration read at L581: `assert_eq!(cfg.ai_local_context, "auto");`
- Configuration read at L692: `cfg.ai_provider = "openai".into();`
- Configuration read at L693: `cfg.openai_base_url = "https://openai.example/v1".into();`
- Configuration read at L694: `cfg.openai_model = "gpt-test".into();`
- Configuration read at L695: `let openai = cfg.ai_endpoint(false);`
- Configuration read at L700: `cfg.ai_provider = "anthropic".into();`
- Configuration read at L703: `let anthropic = cfg.ai_endpoint(false);`
- Configuration read at L716: `cfg.ai_provider = "codex".into();`
- Configuration read at L720: `let endpoint = cfg.ai_endpoint(false);`
- Configuration read at L740: `let cloud_endpoint = cfg.ai_endpoint_cloud();`
- Configuration read at L769: `cfg.ai_provider = "local".into();`
- Configuration read at L770: `cfg.ai_base_url = "https://bridge.example/v1".into();`
- Configuration read at L771: `cfg.ai_bearer = "bridge-secret".into();`
- Configuration read at L773: `let legacy = cfg.ai_endpoint_cloud();`
- Configuration read at L777: `cfg.ai_provider = "openai".into();`
- Configuration read at L778: `cfg.openai_model = "gpt-test".into();`
- Configuration read at L779: `let direct = cfg.ai_endpoint_cloud();`
- Configuration read at L1063: `assert_eq!(cfg.ai_provider, "cloud");`
- Configuration read at L1064: `assert_eq!(cfg.ai_mlx_model, crate::mlx_install::DEFAULT_TEXT_MODEL);`
- Configuration read at L1070: `cfg.ai_provider = "mlx".into();`
- Configuration read at L1071: `let text = cfg.ai_endpoint(false);`
- Configuration read at L1089: `cfg.ai_mlx_model = crate::mlx_install::GEMMA4_MODEL.into();`
- Configuration read at L1242: `assert_eq!(cfg.stt_is_local(), cfg!(target_os = "macos"));`
- Configuration read at L1244: `assert!(!cfg.ai_local_thinking);`
- Configuration read at L1246: `assert_eq!(cfg.stt_gigaam_gpu, !cfg!(target_os = "macos"));`
- Configuration read at L1313: `assert_eq!(cfg.ai_provider, "cloud");`
- Configuration read at L1314: `assert_eq!(cfg.ai_local_base_url, "http://127.0.0.1:8080/v1");`
- Configuration read at L1315: `assert!(!cfg.ai_endpoint(false).is_local);`
