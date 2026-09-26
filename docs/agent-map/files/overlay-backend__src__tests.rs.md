---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9d3189bcb76e"
source_path: "overlay-backend/src/tests.rs"
batch_id: "B09"
total_lines: 1904
symbols_count: 82
review_state: validated
---

# File Map: `overlay-backend/src/tests.rs`

- **Batch:** B09
- **Physical Lines:** 1904
- **Coverage:** 1904/1904 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (82)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `asset` | L13 | `fn asset(name: &str) -> GhAsset` |
| function | `ready_switch_outcomes_include_fallback_and_rollback` | L22 | `fn ready_switch_outcomes_include_fallback_and_rollback() -> ()` |
| function | `long_running_child` | L36 | `fn long_running_child() -> Child` |
| function | `process_is_running` | L50 | `fn process_is_running(pid: u32) -> bool` |
| function | `failed_install_cleanup_terminates_server_children` | L73 | `fn failed_install_cleanup_terminates_server_children() -> ()` |
| function | `llama_readiness_requires_nonempty_text_message_content` | L88 | `fn llama_readiness_requires_nonempty_text_message_content() -> ()` |
| function | `make_complete` | L108 | `fn make_complete(path: &Path, size: u64) -> ()` |
| function | `local_result` | L113 | `fn local_result(quality: bool, vision: bool) -> LocalAiResult` |
| function | `owner_primary_coordinates_and_sha_are_exact` | L135 | `fn owner_primary_coordinates_and_sha_are_exact() -> ()` |
| function | `pick_llama_gguf_uses_explicit_model_only_when_present` | L167 | `fn pick_llama_gguf_uses_explicit_model_only_when_present() -> ()` |
| function | `quality_model_present_rejects_wrong_sizes` | L190 | `fn quality_model_present_rejects_wrong_sizes() -> ()` |
| function | `stat_only_presence_accepts_an_exact_size_fixture_without_hashing` | L202 | `fn stat_only_presence_accepts_an_exact_size_fixture_without_hashing() -> ()` |
| function | `fallback_presence_rejects_the_old_1472_byte_short_size` | L211 | `fn fallback_presence_rejects_the_old_1472_byte_short_size() -> ()` |
| function | `pinned_presence_rejects_same_size_corruption` | L223 | `fn pinned_presence_rejects_same_size_corruption() -> ()` |
| function | `rejected_same_size_primary_is_removed_for_redownload` | L235 | `fn rejected_same_size_primary_is_removed_for_redownload() -> ()` |
| function | `quality_gguf_path_is_under_llama_dir` | L250 | `fn quality_gguf_path_is_under_llama_dir() -> ()` |
| function | `persisted_primary_falls_back_to_12b_when_missing` | L259 | `fn persisted_primary_falls_back_to_12b_when_missing() -> ()` |
| function | `managed_primary_uses_only_its_matching_projector` | L291 | `fn managed_primary_uses_only_its_matching_projector() -> ()` |
| function | `managed_primary_repairs_same_vision_provider_after_selection` | L320 | `fn managed_primary_repairs_same_vision_provider_after_selection() -> ()` |
| function | `local_vision_toggle_enables_f8_route_for_ready_12b` | L339 | `fn local_vision_toggle_enables_f8_route_for_ready_12b() -> ()` |
| function | `custom_gguf_validation_and_repair_are_machine_local` | L370 | `fn custom_gguf_validation_and_repair_are_machine_local() -> ()` |
| function | `legacy_4b_install_survives_upgrade_until_12b_is_installed` | L408 | `fn legacy_4b_install_survives_upgrade_until_12b_is_installed() -> ()` |
| function | `engine_verification_uses_complete_legacy_4b_until_12b_exists` | L446 | `fn engine_verification_uses_complete_legacy_4b_until_12b_exists() -> ()` |
| function | `selecting_local_provider_repairs_managed_state_before_prep_requests` | L463 | `fn selecting_local_provider_repairs_managed_state_before_prep_requests() -> ()` |
| function | `local_model_label_distinguishes_fallback_and_primary` | L485 | `fn local_model_label_distinguishes_fallback_and_primary() -> ()` |
| function | `mmproj_attach_rules_are_model_specific` | L499 | `fn mmproj_attach_rules_are_model_specific() -> ()` |
| function | `primary_model_requires_the_tested_llama_build` | L521 | `fn primary_model_requires_the_tested_llama_build() -> ()` |
| function | `owner_hardware_matrix_is_minimum_ram` | L542 | `fn owner_hardware_matrix_is_minimum_ram() -> ()` |
| function | `tester_16vram_64ram_gets_primary26_and_full_context` | L578 | `fn tester_16vram_64ram_gets_primary26_and_full_context() -> ()` |
| function | `vram_release_uses_the_largest_dedicated_adapter_and_requires_a_drop` | L594 | `fn vram_release_uses_the_largest_dedicated_adapter_and_requires_a_drop() -> ()` |
| function | `only_confirmed_profiles_allow_manual_primary_selection` | L611 | `fn only_confirmed_profiles_allow_manual_primary_selection() -> ()` |
| function | `only_nvidia_discovery_enters_the_confirmed_matrix` | L620 | `fn only_nvidia_discovery_enters_the_confirmed_matrix() -> ()` |
| function | `normalization_snaps_near_nominal_readings_to_approved_tiers` | L641 | `fn normalization_snaps_near_nominal_readings_to_approved_tiers() -> ()` |
| function | `normalization_never_promotes_clearly_smaller_hardware` | L666 | `fn normalization_never_promotes_clearly_smaller_hardware() -> ()` |
| function | `igpu_ram_reservation_snaps_to_existing_profile` | L690 | `fn igpu_ram_reservation_snaps_to_existing_profile() -> ()` |
| function | `near_nominal_vram_snaps_to_existing_profile` | L719 | `fn near_nominal_vram_snaps_to_existing_profile() -> ()` |
| function | `strongest_adapter_vram_normalizes_correctly` | L741 | `fn strongest_adapter_vram_normalizes_correctly() -> ()` |
| function | `launcher_uses_fixed_context_with_the_confirmed_hardware_matrix` | L763 | `fn launcher_uses_fixed_context_with_the_confirmed_hardware_matrix() -> ()` |
| function | `context_presets_are_compact_clamped_and_stable` | L864 | `fn context_presets_are_compact_clamped_and_stable() -> ()` |
| function | `vision_memory_warning_is_explicitly_unknown` | L915 | `fn vision_memory_warning_is_explicitly_unknown() -> ()` |
| function | `managed_endpoint_accepts_ipv4_hostname_and_ipv6_loopback_only` | L926 | `fn managed_endpoint_accepts_ipv4_hostname_and_ipv6_loopback_only() -> ()` |
| function | `strict_readiness_rejects_http_errors_wrong_model_and_malformed_choices` | L945 | `fn strict_readiness_rejects_http_errors_wrong_model_and_malformed_choices() -> ()` |
| function | `spawn_stale_llama_server` | L967 | `fn spawn_stale_llama_server(expected: &'static str,) -> (String, mpsc::Sender<()>, thread::JoinHandle<()>)` |
| function | `strict_readiness_rejects_stale_server_after_new_child_exits` | L1006 | `fn strict_readiness_rejects_stale_server_after_new_child_exits() -> ()` |
| function | `spawn_long_lived_child` | L1066 | `fn spawn_long_lived_child() -> Child` |
| function | `spawn_exiting_child` | L1082 | `fn spawn_exiting_child() -> Child` |
| function | `llama_readiness_ignores_an_exited_whisper_child` | L1099 | `fn llama_readiness_ignores_an_exited_whisper_child() -> ()` |
| function | `build_tag_parses_with_or_without_b` | L1113 | `fn build_tag_parses_with_or_without_b() -> ()` |
| function | `engine_update_throttle` | L1124 | `fn engine_update_throttle() -> ()` |
| function | `swap_backs_up_and_overwrites_keeping_models` | L1147 | `fn swap_backs_up_and_overwrites_keeping_models() -> ()` |
| function | `prune_engine_backups_keeps_count_and_spares_manual_and_live` | L1182 | `fn prune_engine_backups_keeps_count_and_spares_manual_and_live() -> ()` |
| function | `bundled_gigaam_vocab_is_sane` | L1218 | `fn bundled_gigaam_vocab_is_sane() -> ()` |
| function | `cuda_version_parse` | L1233 | `fn cuda_version_parse() -> ()` |
| function | `pick_newest_cuda_and_matching_cudart` | L1252 | `fn pick_newest_cuda_and_matching_cudart() -> ()` |
| function | `pick_cpu_when_forced` | L1275 | `fn pick_cpu_when_forced() -> ()` |
| function | `pick_vulkan_for_non_nvidia_gpu` | L1289 | `fn pick_vulkan_for_non_nvidia_gpu() -> ()` |
| function | `pick_cpu_when_non_nvidia_but_no_vulkan_asset` | L1307 | `fn pick_cpu_when_non_nvidia_but_no_vulkan_asset() -> ()` |
| function | `pick_llama_macos_metal` | L1322 | `fn pick_llama_macos_metal() -> ()` |
| function | `pick_whisper_cpu_takes_plain_build` | L1335 | `fn pick_whisper_cpu_takes_plain_build() -> ()` |
| function | `pick_whisper_gpu_takes_highest_cublas` | L1351 | `fn pick_whisper_gpu_takes_highest_cublas() -> ()` |
| function | `pick_whisper_gpu_falls_back_to_cpu_when_no_cublas` | L1367 | `fn pick_whisper_gpu_falls_back_to_cpu_when_no_cublas() -> ()` |
| function | `pick_whisper_macos` | L1382 | `fn pick_whisper_macos() -> ()` |
| function | `live_pick_llama_is_blackwell_capable` | L1393 | `fn live_pick_llama_is_blackwell_capable() -> ()` |
| function | `compute_apps_detects_llama` | L1423 | `fn compute_apps_detects_llama() -> ()` |
| function | `listener_pids_on_port_parses_only_listening_target_port` | L1432 | `fn listener_pids_on_port_parses_only_listening_target_port() -> ()` |
| function | `path_is_under_root_rejects_sibling_prefix` | L1446 | `fn path_is_under_root_rejects_sibling_prefix() -> ()` |
| function | `apply_result_sets_local_and_keeps_secrets` | L1461 | `fn apply_result_sets_local_and_keeps_secrets() -> ()` |
| function | `apply_primary_routes_vision_only_when_the_installer_verified_it` | L1492 | `fn apply_primary_routes_vision_only_when_the_installer_verified_it() -> ()` |
| function | `swap_installs_nested_engine_layout` | L1540 | `fn swap_installs_nested_engine_layout() -> ()` |
| function | `swap_installs_flat_engine_layout` | L1560 | `fn swap_installs_flat_engine_layout() -> ()` |
| function | `swap_fails_without_llama_server_so_no_phantom_update` | L1572 | `fn swap_fails_without_llama_server_so_no_phantom_update() -> ()` |
| function | `swap_rejects_ambiguous_duplicate_engine_file` | L1586 | `fn swap_rejects_ambiguous_duplicate_engine_file() -> ()` |
| function | `archive_entry_safety_rejects_zip_slip` | L1605 | `fn archive_entry_safety_rejects_zip_slip() -> ()` |
| function | `all_managed_models_accessible_regardless_of_hardware` | L1638 | `fn all_managed_models_accessible_regardless_of_hardware() -> ()` |
| function | `low_hardware_profile_is_recommendation_only` | L1659 | `fn low_hardware_profile_is_recommendation_only() -> ()` |
| function | `sixteen_vram_31_ram_normalizes_and_exposes_all_profiles` | L1675 | `fn sixteen_vram_31_ram_normalizes_and_exposes_all_profiles() -> ()` |
| function | `unknown_hardware_does_not_block_26b_download_path` | L1688 | `fn unknown_hardware_does_not_block_26b_download_path() -> ()` |
| function | `model_specs_pin_immutable_revisions_with_full_sha` | L1711 | `fn model_specs_pin_immutable_revisions_with_full_sha() -> ()` |
| function | `legacy_4b_spec_matches_the_pinned_hf_revision` | L1766 | `fn legacy_4b_spec_matches_the_pinned_hf_revision() -> ()` |
| function | `legacy_presence_uses_the_pinned_4b_size` | L1785 | `fn legacy_presence_uses_the_pinned_4b_size() -> ()` |
| function | `hardware_never_blocks_any_managed_model_download` | L1812 | `fn hardware_never_blocks_any_managed_model_download() -> ()` |
| function | `old_size_4b_upgrade_compatibility_and_new_spec_integrity` | L1837 | `fn old_size_4b_upgrade_compatibility_and_new_spec_integrity() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L974

## Configuration Access

- Configuration read at L281: `assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);`
- Configuration read at L282: `assert!(!cfg.ai_local_quality);`
- Configuration read at L283: `assert_eq!(cfg.ai_local_model, GEMMA_FILE);`
- Configuration read at L284: `assert!(cfg.ai_local_prep_model.is_empty());`
- Configuration read at L285: `assert!(!cfg.ai_local_vision);`
- Configuration read at L313: `assert!(cfg.ai_local_vision);`
- Configuration read at L360: `assert!(cfg.ai_local_vision);`
- Configuration read at L365: `assert!(!cfg.ai_local_vision);`
- Configuration read at L394: `assert_eq!(cfg.ai_local_custom_gguf, good.to_string_lossy());`
- Configuration read at L395: `assert_eq!(cfg.ai_local_model, "my-model.GGUF");`
- Configuration read at L396: `assert!(!cfg.ai_local_quality);`
- Configuration read at L397: `assert!(!cfg.ai_local_vision);`
- Configuration read at L403: `assert!(cfg.ai_local_custom_gguf.is_empty());`
- Configuration read at L404: `assert_eq!(cfg.ai_local_model, GEMMA_FILE);`
- Configuration read at L432: `assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);`
- Configuration read at L433: `assert!(!cfg.ai_local_quality);`
- Configuration read at L434: `assert_eq!(cfg.ai_local_model, LEGACY_GEMMA_FILE);`
- Configuration read at L435: `assert!(cfg.ai_local_prep_model.is_empty());`
- Configuration read at L442: `assert_eq!(cfg.ai_local_model, LEGACY_GEMMA_FILE);`
- Configuration read at L475: `assert_eq!(cfg.ai_provider, "local");`
- Configuration read at L476: `assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);`
- Configuration read at L477: `assert_eq!(cfg.ai_local_model, GEMMA_FILE);`
- Configuration read at L478: `assert!(cfg.ai_local_prep_model.is_empty());`
- Configuration read at L479: `assert_eq!(cfg.ai_endpoint(true).model, GEMMA_FILE);`
- Configuration read at L1474: `assert_eq!(cfg.ai_provider, "local");`
- Configuration read at L1475: `assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);`
- Configuration read at L1476: `assert_eq!(cfg.ai_local_model, GEMMA_FILE);`
- Configuration read at L1477: `assert!(!cfg.ai_local_quality);`
- Configuration read at L1478: `assert!(cfg.ai_local_prep_model.is_empty());`
- Configuration read at L1484: `assert_eq!(cfg.ai_bearer, "bridge_secret");`
- Configuration read at L1487: `assert!(cfg.ai_local_vision);`
- Configuration read at L1499: `assert_eq!(cloud_cfg.ai_local_model, GEMMA26_FILE);`
- Configuration read at L1500: `assert!(cloud_cfg.ai_local_quality);`
- Configuration read at L1501: `assert!(cloud_cfg.ai_local_prep_model.is_empty());`
- Configuration read at L1502: `assert!(!cloud_cfg.ai_local_vision);`
- Configuration read at L1513: `assert!(stale_same_cfg.ai_local_vision);`
- Configuration read at L1903: `assert_eq!(cfg.ai_local_model, LEGACY_GEMMA_FILE);`
