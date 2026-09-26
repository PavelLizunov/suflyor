---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_756230e465b0"
source_path: "overlay-backend/src/hermes_install.rs"
batch_id: "B09"
total_lines: 873
symbols_count: 35
review_state: validated
---

# File Map: `overlay-backend/src/hermes_install.rs`

- **Batch:** B09
- **Physical Lines:** 873
- **Coverage:** 873/873 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `ApiEdit` | L157 | pub |
| enum | `EnableEdit` | L400 | pub |

## Symbols & Routines (35)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `hermes_home` | L27 | `fn hermes_home() -> Option<PathBuf>` |
| function | `bridge_url_for_env` | L56 | `fn bridge_url_for_env(bind_host: &str, port: u16) -> String` |
| function | `install_plugin` | L68 | `fn install_plugin(bridge_url: &str, token: &str) -> Result<String, String>` |
| function | `ensure_api_server` | L128 | `fn ensure_api_server() -> Result<(String, bool), String>` |
| function | `ensure_api_server_text` | L169 | `fn ensure_api_server_text(existing: &str, gen_key: impl Fn() -> String) -> ApiEdit` |
| function | `finish_api` | L387 | `fn finish_api(text: String, eol: &str, key: String) -> ApiEdit` |
| function | `eol_of` | L411 | `fn eol_of(text: &str) -> &'static str` |
| function | `merge_env_text` | L422 | `fn merge_env_text(existing: &str, url: &str, token: &str) -> String` |
| function | `indent_of` | L468 | `fn indent_of(line: &str) -> usize` |
| function | `enable_in_config_text` | L476 | `fn enable_in_config_text(existing: &str) -> EnableEdit` |
| function | `finish` | L600 | `fn finish(text: String, eol: &str) -> EnableEdit` |
| function | `embedded_plugin_files_nonempty` | L617 | `fn embedded_plugin_files_nonempty() -> ()` |
| function | `env_merge_appends_when_missing` | L623 | `fn env_merge_appends_when_missing() -> ()` |
| function | `env_merge_replaces_in_place_and_is_idempotent` | L631 | `fn env_merge_replaces_in_place_and_is_idempotent() -> ()` |
| function | `env_merge_preserves_crlf` | L643 | `fn env_merge_preserves_crlf() -> ()` |
| function | `config_append_when_no_plugins_key` | L652 | `fn config_append_when_no_plugins_key() -> ()` |
| function | `config_create_when_empty` | L664 | `fn config_create_when_empty() -> ()` |
| function | `config_inserts_into_existing_enabled_list` | L672 | `fn config_inserts_into_existing_enabled_list() -> ()` |
| function | `config_adds_enabled_under_bare_plugins` | L684 | `fn config_adds_enabled_under_bare_plugins() -> ()` |
| function | `config_already_enabled_detected` | L696 | `fn config_already_enabled_detected() -> ()` |
| function | `config_empty_flow_list_converted` | L705 | `fn config_empty_flow_list_converted() -> ()` |
| function | `config_flow_forms_unsupported` | L714 | `fn config_flow_forms_unsupported() -> ()` |
| function | `config_crlf_preserved` | L726 | `fn config_crlf_preserved() -> ()` |
| function | `config_block_ends_at_next_top_level_key` | L735 | `fn config_block_ends_at_next_top_level_key() -> ()` |
| function | `genkey` | L748 | `fn genkey() -> String` |
| function | `api_appends_block_to_stock_config` | L753 | `fn api_appends_block_to_stock_config() -> ()` |
| function | `api_inserts_under_existing_platforms` | L768 | `fn api_inserts_under_existing_platforms() -> ()` |
| function | `api_flips_enabled_false_keeps_key` | L780 | `fn api_flips_enabled_false_keeps_key() -> ()` |
| function | `api_ready_when_all_set` | L792 | `fn api_ready_when_all_set() -> ()` |
| function | `api_inserts_missing_enabled_keeps_key` | L801 | `fn api_inserts_missing_enabled_keeps_key() -> ()` |
| function | `api_fills_empty_key` | L811 | `fn api_fills_empty_key() -> ()` |
| function | `api_missing_extra_added` | L821 | `fn api_missing_extra_added() -> ()` |
| function | `api_flow_forms_unsupported` | L833 | `fn api_flow_forms_unsupported() -> ()` |
| function | `api_crlf_preserved` | L852 | `fn api_crlf_preserved() -> ()` |
| function | `bridge_url_host_selection` | L861 | `fn bridge_url_host_selection() -> ()` |
