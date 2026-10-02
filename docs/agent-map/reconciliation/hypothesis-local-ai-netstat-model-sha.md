# Original local AI C04/C05: bounded netstat parsing and model hash invariants evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_local_ai_netstat_model_sha_hypotheses.py) inspect frozen local AI port-clearing and model-selection logic. They do not run `netstat`, terminate processes, or verify multi-gigabyte GGUF model files. C04 and C05 remain hypotheses.

## C04 — netstat line parsing, port suffix matching, and stranger conservatism

In Windows port reclaiming, [stop_listener_on_port](<../../../overlay-backend/src/local_ai.rs#L1211-L1268>) runs `netstat -ano -p tcp` and parses output lines by splitting columns on whitespace.
Lines with status `LISTENING` match target ports using `cols[1].ends_with(":{port}")`. This condition matches both IPv4 (`127.0.0.1:8080`) and bracketed IPv6 loopback (`[::1]:8080`).
If `exe_path_for_pid` cannot resolve a path (e.g. elevated or other-user processes) or points outside the app installation root, `stop_listener_on_port` sets `free_of_strangers = false` and refrains from killing the process.

## C05 — selective GGUF hash verification vs size-only launch invariants

In `model_state.rs`, [selected_llama_gguf](<../../../overlay-backend/src/local_ai/model_state.rs#L385-L428>) performs hash verification with `cached_pinned_file_matches(..., GEMMA26_SHA256)` **only** for `ManagedModel::Primary26B`.
For `ManagedModel::Legacy4B` and `ManagedModel::Fallback12B`, it checks file existence and size using `file_has_expected_size` without recalculating SHA-256 digests.
Similarly, [quality_model_present](<../../../overlay-backend/src/local_ai/model_state.rs#L113-L115>) is a fast metadata size check (`file_has_expected_size(&quality_gguf_path(root), GEMMA26_SIZE)`), and [mmproj_for_model](<../../../overlay-backend/src/local_ai/model_state.rs#L480-L497>) attaches vision projectors matching expected byte length and llama build capability stamps without hashing.

## Limits

No elevated processes were probed, no live `netstat` calls were issued, and no model corruption was simulated on disk. Original statuses in `candidates.json` remain `hypothesis`.
