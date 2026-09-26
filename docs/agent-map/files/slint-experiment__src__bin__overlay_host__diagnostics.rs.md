---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3b3cd1ae8204"
source_path: "slint-experiment/src/bin/overlay_host/diagnostics.rs"
batch_id: "B01"
total_lines: 955
symbols_count: 23
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/diagnostics.rs`

- **Batch:** B01
- **Physical Lines:** 955
- **Coverage:** 955/955 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (23)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `populate_diagnostics` | L30 | `fn populate_diagnostics(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `is_ipv4` | L71 | `fn is_ipv4(s: &str) -> bool` |
| function | `redact_ipv4` | L86 | `fn redact_ipv4(s: &str) -> String` |
| function | `redact_urls` | L117 | `fn redact_urls(s: &str) -> String` |
| function | `redact_secrets` | L146 | `fn redact_secrets(s: &str) -> String` |
| function | `redact_user_home` | L191 | `fn redact_user_home(s: &str) -> String` |
| function | `redact_home_all_forms` | L216 | `fn redact_home_all_forms(s: &str, home: &str) -> String` |
| function | `redact_home_in` | L232 | `fn redact_home_in(s: &str, home: &str) -> String` |
| function | `prime_gpu_cache` | L261 | `fn prime_gpu_cache() -> ()` |
| function | `gpu_name` | L276 | `fn gpu_name() -> String` |
| function | `build_diag_report` | L314 | `fn build_diag_report(cfg: &overlay_backend::config::SharedConfig) -> String` |
| function | `wire_diagnostics` | L386 | `fn wire_diagnostics(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |
| function | `collect_redacted_log` | L732 | `fn collect_redacted_log() -> std::io::Result<std::path::PathBuf>` |
| function | `reveal_in_explorer` | L747 | `fn reveal_in_explorer(path: &std::path::Path) -> ()` |
| function | `redact_ipv4_masks_lan_ip_keeps_port_and_path` | L777 | `fn redact_ipv4_masks_lan_ip_keeps_port_and_path() -> ()` |
| function | `redact_user_home_masks_profile_dir_keeps_rest` | L796 | `fn redact_user_home_masks_profile_dir_keeps_rest() -> ()` |
| function | `redact_user_home_masks_double_backslash_and_forward_slash_forms` | L826 | `fn redact_user_home_masks_double_backslash_and_forward_slash_forms() -> ()` |
| function | `redact_user_home_masks_posix_mac_home_paths` | L850 | `fn redact_user_home_masks_posix_mac_home_paths() -> ()` |
| function | `redact_urls_masks_dns_ipv6_and_ipv4_hosts_keeping_scheme_port_path` | L860 | `fn redact_urls_masks_dns_ipv6_and_ipv4_hosts_keeping_scheme_port_path() -> ()` |
| function | `redact_urls_masks_uppercase_and_mixed_case_schemes` | L893 | `fn redact_urls_masks_uppercase_and_mixed_case_schemes() -> ()` |
| function | `redact_urls_masks_dns_host_in_a_report_shaped_string` | L905 | `fn redact_urls_masks_dns_host_in_a_report_shaped_string() -> ()` |
| function | `redact_secrets_masks_bearer_gsk_and_sk_tokens` | L930 | `fn redact_secrets_masks_bearer_gsk_and_sk_tokens() -> ()` |
| function | `is_ipv4_accepts_valid_rejects_ports_and_versions` | L946 | `fn is_ipv4_accepts_valid_rejects_ports_and_versions() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L264
- Spawns asynchronous thread/task at L421
- Spawns asynchronous thread/task at L461
- Spawns asynchronous thread/task at L488
- Spawns asynchronous thread/task at L541
- Spawns asynchronous thread/task at L694
