---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2e56170c8fa7"
source_path: "overlay-backend/src/local_ai.rs"
batch_id: "B06"
total_lines: 3165
symbols_count: 93
review_state: validated
---

# File Map: `overlay-backend/src/local_ai.rs`

- **Batch:** B06
- **Physical Lines:** 3165
- **Coverage:** 3165/3165 lines (100%)

## Types & Structures (11)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Progress` | L183 | pub |
| enum | `ModelSwitch` | L1288 | pub |
| enum | `EngineUpdate` | L1552 | pub |
| struct | `InstallOptions` | L157 | pub |
| struct | `LocalAiResult` | L201 | pub |
| struct | `InstallServerCleanup` | L220 | private |
| struct | `GhAsset` | L2144 | private |
| struct | `GhRelease` | L2152 | private |
| struct | `LlamaPick` | L2162 | private |
| struct | `FileStamp` | L2778 | private |
| struct | `PinnedFileVerification` | L2784 | private |

## Symbols & Routines (93)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `lifecycle_lock` | L127 | `fn lifecycle_lock() -> Arc<tokio::sync::Semaphore>` |
| function | `blocking_acquire_lifecycle` | L133 | `fn blocking_acquire_lifecycle(lock: &Arc<tokio::sync::Semaphore>,) -> Option<tokio::sync::OwnedSemaphorePermit>` |
| function | `default` | L169 | `fn default() -> Self` |
| function | `into_children` | L225 | `fn into_children(mut self) -> Vec<Child>` |
| function | `drop` | L231 | `fn drop(&mut self) -> ()` |
| function | `system_memory_telemetry` | L236 | `fn system_memory_telemetry(pid: Option<u32>) -> Option<String>` |
| function | `log_memory_telemetry` | L265 | `fn log_memory_telemetry(stage: &str, pid: Option<u32>, exited: bool) -> ()` |
| function | `wait_for_nvidia_vram_release` | L298 | `fn wait_for_nvidia_vram_release(before_mib: Option<u64>, budget: Duration) -> bool` |
| function | `vram_has_released` | L312 | `fn vram_has_released(before_mib: u64, after_mib: Option<u64>) -> bool` |
| function | `wait_for_nvidia_vram_baseline` | L316 | `fn wait_for_nvidia_vram_baseline(baseline_mib: Option<u64>, budget: Duration) -> bool` |
| function | `vram_is_at_baseline` | L330 | `fn vram_is_at_baseline(baseline_mib: u64, after_mib: Option<u64>) -> bool` |
| function | `apply_result` | L338 | `fn apply_result(cfg: &mut crate::config::Config, res: &LocalAiResult) -> ()` |
| function | `curl_exe` | L894 | `fn curl_exe() -> &'static str` |
| function | `dev_null` | L902 | `fn dev_null() -> &'static str` |
| function | `is_reachable` | L912 | `fn is_reachable(url: &str) -> bool` |
| function | `models_list_expected_model` | L921 | `fn models_list_expected_model(http_success: bool, body: &str, expected: &str) -> bool` |
| function | `completion_has_choice` | L933 | `fn completion_has_choice(http_success: bool, body: &str) -> bool` |
| function | `expected_model_is_ready` | L937 | `fn expected_model_is_ready(models_http_success: bool, models_body: &str, expected: &str, completion_http_success: bool, completion_body: &str,) -> bool` |
| function | `curl_success_body` | L948 | `fn curl_success_body(args: &[&str]) -> Option<String>` |
| function | `launched_llama_alive` | L958 | `fn launched_llama_alive(child: &mut Child) -> bool` |
| function | `launched_llama_owns_listener` | L963 | `fn launched_llama_owns_listener(child: &mut Child) -> bool` |
| function | `launched_llama_owns_listener` | L981 | `fn launched_llama_owns_listener(child: &mut Child) -> bool` |
| function | `wait_for_expected_llama` | L985 | `fn wait_for_expected_llama(expected: &str, budget: Duration, llama: &mut Child) -> bool` |
| function | `wait_for_expected_model_at` | L992 | `fn wait_for_expected_model_at(base_url: &str, expected: &str, budget: Duration, llama: &mut Child,) -> bool` |
| function | `stop_managed_servers` | L1064 | `fn stop_managed_servers(root: &Path, servers: I) -> ()` |
| function | `terminate_servers` | L1078 | `fn terminate_servers(servers: I) -> ()` |
| function | `terminate_child_tree` | L1087 | `fn terminate_child_tree(mut child: Child) -> ()` |
| function | `exe_path_for_pid` | L1108 | `fn exe_path_for_pid(pid: &str) -> Option<String>` |
| function | `is_pid_alive` | L1127 | `fn is_pid_alive(pid_num: u32) -> bool` |
| function | `wait_for_pid_exit` | L1144 | `fn wait_for_pid_exit(pid: &str, budget: Duration) -> bool` |
| function | `kill_pid_tree` | L1173 | `fn kill_pid_tree(pid: &str) -> bool` |
| function | `listener_pids_on_port` | L1180 | `fn listener_pids_on_port(netstat: &'a str, port: &str) -> Vec<&'a str>` |
| function | `path_is_under_root` | L1200 | `fn path_is_under_root(path: &str, root_lc: &str) -> bool` |
| function | `stop_listener_on_port` | L1212 | `fn stop_listener_on_port(port: &str, root: &Path) -> bool` |
| function | `stop_listener_on_port` | L1271 | `fn stop_listener_on_port(_port: &str, _root: &Path) -> bool` |
| function | `free_llama_port` | L1281 | `fn free_llama_port(root: &Path) -> bool` |
| function | `switch_commits_choice` | L1311 | `fn switch_commits_choice(outcome: ModelSwitch) -> bool` |
| function | `switch_has_ready_server` | L1317 | `fn switch_has_ready_server(outcome: ModelSwitch) -> bool` |
| function | `switch_local_model` | L1328 | `fn switch_local_model(root: &Path, previous: ManagedLlamaChoice, target: ManagedLlamaChoice, want_whisper: bool,) -> (ModelSwitch, Vec<Child>)` |
| function | `llama_reachable` | L1404 | `fn llama_reachable() -> bool` |
| function | `ensure_llama_serving` | L1417 | `fn ensure_llama_serving(root: &Path, choice: ManagedLlamaChoice) -> (ModelSwitch, Vec<Child>)` |
| function | `restart_llama_server` | L1431 | `fn restart_llama_server(root: &Path, choice: ManagedLlamaChoice) -> (ModelSwitch, Vec<Child>)` |
| function | `restart_llama_server_for_unlock` | L1440 | `fn restart_llama_server_for_unlock(root: &Path, choice: ManagedLlamaChoice,) -> (ModelSwitch, Vec<Child>)` |
| function | `restart_llama_server_for_route` | L1450 | `fn restart_llama_server_for_route(root: &Path, choice: ManagedLlamaChoice, prep: bool,) -> (ModelSwitch, Vec<Child>)` |
| function | `restart_llama_server_for_route_inner` | L1458 | `fn restart_llama_server_for_route_inner(root: &Path, choice: ManagedLlamaChoice, prep: bool, allow_deep_locked_launch: bool,) -> (ModelSwitch, Vec<Child>)` |
| function | `now_unix` | L1564 | `fn now_unix() -> u64` |
| function | `should_check_engine_update` | L1575 | `fn should_check_engine_update(root: &Path) -> bool` |
| function | `mark_engine_update_checked` | L1591 | `fn mark_engine_update_checked(root: &Path) -> ()` |
| function | `installed_engine_build` | L1599 | `fn installed_engine_build(root: &Path) -> Option<u32>` |
| function | `verify_engine_runs` | L1740 | `fn verify_engine_runs(staged_exe: &Path, llama_dir: &Path, use_gpu: bool, cancel: &AtomicBool,) -> bool` |
| function | `swap_engine_binaries` | L1798 | `fn swap_engine_binaries(staging: &Path, live: &Path, backup: &Path) -> Result<()>` |
| function | `prune_engine_backups` | L1877 | `fn prune_engine_backups(root: &Path, keep: usize) -> usize` |
| function | `sweep_orphaned_engine_artifacts` | L1928 | `fn sweep_orphaned_engine_artifacts(root: &Path) -> ()` |
| function | `ensure_servers` | L1949 | `fn ensure_servers(root: &Path, want_llama: bool, want_whisper: bool, choice: ManagedLlamaChoice,) -> Vec<Child>` |
| function | `ensure_servers_for_route` | L1958 | `fn ensure_servers_for_route(root: &Path, want_llama: bool, want_whisper: bool, choice: ManagedLlamaChoice, prep: bool, allow_deep_locked_launch: bool,) -> Vec<Child>` |
| function | `llama_server_args` | L2058 | `fn llama_server_args(model: &str, alias: &str, mmproj: Option<&str>, force_cpu: bool, profile: HardwareModelProfile, context: LocalContextPreset, prep: bool,) -> Vec<String>` |
| function | `github_release` | L2170 | `fn github_release(repo: &str) -> Result<GhRelease>` |
| function | `github_assets` | L2194 | `fn github_assets(repo: &str) -> Result<Vec<GhAsset>>` |
| function | `cuda_version_of` | L2201 | `fn cuda_version_of(name: &str) -> Option<(u32, u32)>` |
| function | `pick_llama` | L2211 | `fn pick_llama(assets: &[GhAsset], _gpu: GpuKind) -> Result<LlamaPick>` |
| function | `whisper_cublas_version_of` | L2290 | `fn whisper_cublas_version_of(name: &str) -> Option<(u32, u32, u32)>` |
| function | `pick_whisper` | L2306 | `fn pick_whisper(assets: &[GhAsset], force_cpu: bool) -> Result<(String, u64)>` |
| function | `is_trusted_release_url` | L2444 | `fn is_trusted_release_url(url: &str) -> bool` |
| function | `system_tar` | L2475 | `fn system_tar() -> PathBuf` |
| function | `archive_entry_is_safe` | L2487 | `fn archive_entry_is_safe(entry: &str) -> bool` |
| function | `extract_zip` | L2513 | `fn extract_zip(zip: &Path, dest_dir: &Path) -> Result<()>` |
| function | `curl_small` | L2642 | `fn curl_small(url: &str, out: &Path) -> Result<()>` |
| function | `parse_compute_apps` | L2678 | `fn parse_compute_apps(stdout: &str) -> bool` |
| function | `verify_gpu_offload` | L2684 | `fn verify_gpu_offload(tries: u32) -> bool` |
| function | `wait_ready` | L2706 | `fn wait_ready(url: &str, max_secs: u64) -> Result<()>` |
| function | `llama_reply_has_text_content` | L2725 | `fn llama_reply_has_text_content(body: &str) -> bool` |
| function | `preflight` | L2744 | `fn preflight() -> Result<()>` |
| function | `file_len` | L2754 | `fn file_len(p: &Path) -> u64` |
| function | `file_has_expected_size` | L2758 | `fn file_has_expected_size(path: &Path, expected_size: u64) -> bool` |
| function | `legacy_gguf_complete` | L2765 | `fn legacy_gguf_complete(path: &Path) -> bool` |
| function | `pinned_file_matches` | L2772 | `fn pinned_file_matches(path: &Path, expected_size: u64, expected_sha256: &str) -> bool` |
| function | `file_stamp` | L2796 | `fn file_stamp(path: &Path) -> Option<FileStamp>` |
| function | `cached_pinned_file_matches` | L2804 | `fn cached_pinned_file_matches(path: &Path, expected_size: u64, expected_sha256: &str,) -> bool` |
| function | `cache_quality_model_verification` | L2843 | `fn cache_quality_model_verification(path: &Path, matches: bool) -> ()` |
| function | `discard_rejected_pinned_file` | L2864 | `fn discard_rejected_pinned_file(path: &Path, expected_size: u64) -> ()` |
| function | `quality_model_verified` | L2879 | `fn quality_model_verified(root: &Path) -> bool` |
| function | `bail_if_cancelled` | L2897 | `fn bail_if_cancelled(cancel: &AtomicBool) -> Result<()>` |
| function | `reuse_if_available` | L2918 | `fn reuse_if_available(dest: &Path, expected: u64, expected_sha256: &str, candidates: &[PathBuf],) -> bool` |
| function | `sha256_hex_of` | L2953 | `fn sha256_hex_of(path: &Path) -> Option<String>` |
| function | `verify_sha256` | L2975 | `fn verify_sha256(path: &Path, expected_hex: &str, label: &str) -> Result<()>` |
| function | `free_bytes_on_volume` | L2992 | `fn free_bytes_on_volume(path: &Path) -> Option<u64>` |
| function | `find_exe` | L3040 | `fn find_exe(dir: &Path, name: &str) -> Option<PathBuf>` |
| function | `hidden_command` | L3063 | `fn hidden_command(exe: &str, args: &[&str]) -> Command` |
| function | `spawn_hidden` | L3071 | `fn spawn_hidden(exe: &str, args: &[&str]) -> Result<Child>` |
| function | `launch_hidden` | L3081 | `fn launch_hidden(exe: &Path, args: &[&str]) -> Result<Child>` |
| function | `assign_to_lifetime_job` | L3104 | `fn assign_to_lifetime_job(child: &Child) -> ()` |
| function | `launch_hidden_wait` | L3149 | `fn launch_hidden_wait(exe: &str, args: &[&str]) -> Result<std::process::ExitStatus>` |
| function | `run_capture` | L3157 | `fn run_capture(exe: &str, args: &[&str]) -> Result<std::process::Output>` |

## Configuration Access

- Configuration read at L339: `cfg.ai_provider = "local".to_string();`
- Configuration read at L340: `cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();`
- Configuration read at L341: `cfg.ai_local_model = res.ai_local_model.clone();`
- Configuration read at L342: `cfg.ai_local_custom_gguf.clear();`
- Configuration read at L343: `cfg.ai_local_prep_model.clear();`
- Configuration read at L344: `cfg.ai_local_quality = res.ai_local_quality;`
- Configuration read at L353: `cfg.ai_local_vision = res.ai_local_vision;`
- Configuration read at L354: `if cfg.ai_local_vision {`
