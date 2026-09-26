---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_cb3148ceb9d6"
source_path: "overlay-backend/src/deep_lock.rs"
batch_id: "B06"
total_lines: 417
symbols_count: 23
review_state: validated
---

# File Map: `overlay-backend/src/deep_lock.rs`

- **Batch:** B06
- **Physical Lines:** 417
- **Coverage:** 417/417 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `LockAction` | L76 | pub |
| enum | `LockMode` | L98 | pub |
| enum | `LockStatus` | L189 | pub |

## Symbols & Routines (23)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `set_deep_lock_active` | L28 | `fn set_deep_lock_active(active: bool) -> ()` |
| function | `deep_lock_active` | L33 | `fn deep_lock_active() -> bool` |
| function | `is_blocked_error` | L43 | `fn is_blocked_error(msg: &str) -> bool` |
| function | `endpoint_blocked` | L51 | `fn endpoint_blocked(deep_lock: bool, base_url: &str) -> bool` |
| function | `lifecycle_launch_allowed` | L60 | `fn lifecycle_launch_allowed(deep_lock: bool, explicit_unlock: bool) -> bool` |
| function | `cfg_is_managed_local` | L67 | `fn cfg_is_managed_local(cfg: &crate::config::Config) -> bool` |
| function | `lock_mode_action` | L108 | `fn lock_mode_action(is_managed_local: bool, suppress_tiles: bool, deep_lock: bool, requested: LockMode,) -> LockAction` |
| function | `next_lock_action` | L134 | `fn next_lock_action(is_managed_local: bool, suppress_tiles: bool, deep_lock: bool,) -> LockAction` |
| function | `state_hint` | L155 | `fn state_hint(ru: bool, managed: bool, suppress_tiles: bool, deep_lock: bool) -> &'static str` |
| function | `status_text` | L200 | `fn status_text(ru: bool, status: LockStatus) -> &'static str` |
| function | `blocked_notice` | L222 | `fn blocked_notice(ru: bool) -> &'static str` |
| function | `blocked_test_result` | L233 | `fn blocked_test_result(ru: bool, msg: &str) -> Option<String>` |
| function | `unlock_failed_notice` | L239 | `fn unlock_failed_notice(ru: bool) -> &'static str` |
| function | `lifecycle_guard_notice` | L251 | `fn lifecycle_guard_notice(ru: bool) -> &'static str` |
| function | `cfg` | L265 | `fn cfg(provider: &str, base_url: &str) -> Config` |
| function | `managed_only_for_local_provider_on_bundled_loopback_endpoint` | L273 | `fn managed_only_for_local_provider_on_bundled_loopback_endpoint() -> ()` |
| function | `managed_clicks_walk_three_states` | L301 | `fn managed_clicks_walk_three_states() -> ()` |
| function | `non_managed_clicks_keep_the_two_state_toggle` | L315 | `fn non_managed_clicks_keep_the_two_state_toggle() -> ()` |
| function | `explicit_menu_preserves_listening_when_reloading_from_deep_lock` | L333 | `fn explicit_menu_preserves_listening_when_reloading_from_deep_lock() -> ()` |
| function | `endpoint_guard_blocks_only_managed_url_while_active` | L356 | `fn endpoint_guard_blocks_only_managed_url_while_active() -> ()` |
| function | `lifecycle_guard_has_one_explicit_unlock_bypass` | L368 | `fn lifecycle_guard_has_one_explicit_unlock_bypass() -> ()` |
| function | `blocked_error_marker_matches_its_chains` | L376 | `fn blocked_error_marker_matches_its_chains() -> ()` |
| function | `copy_is_localized_and_state_distinct` | L386 | `fn copy_is_localized_and_state_distinct() -> ()` |

## Configuration Access

- Configuration read at L68: `(cfg.ai_provider == "local"`
- Configuration read at L69: `&& crate::local_ai::is_managed_llama_endpoint(&cfg.ai_local_base_url))`
- Configuration read at L70: `|| (cfg.ai_provider == "mlx" && cfg!(target_os = "macos"))`
