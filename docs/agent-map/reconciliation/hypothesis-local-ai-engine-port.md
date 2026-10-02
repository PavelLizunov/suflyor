# Original local AI C02/C03: bounded engine verification port and bind race evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_local_ai_engine_port_hypotheses.py) inspect frozen local AI engine management and server launching logic. They do not start local HTTP servers, bind ports, or launch child processes. C02 and C03 remain hypotheses.

## C02 — engine verify-before-swap port and readiness check

In `local_ai.rs`, [verify_engine_runs](<../../../overlay-backend/src/local_ai.rs#L1740-L1796>) tests a staged engine binary on `ENGINE_VERIFY_PORT = "8077"`.
It calls `let _ = stop_listener_on_port(ENGINE_VERIFY_PORT, root);`, discarding the return value, and then waits for the server to become ready via `wait_ready(&format!("http://127.0.0.1:{ENGINE_VERIFY_PORT}/v1/models"), 60)`.
Unlike [wait_for_expected_model_at](<../../../overlay-backend/src/local_ai.rs#L992-L1056>) (which validates both PID ownership via `launched_llama_owns_listener` and model identity via `expected_model_is_ready`), `verify_engine_runs` performs a generic HTTP reachability probe without PID listener ownership validation or model identity matching.

## C03 — server route launch without listener ownership check

In [ensure_servers_for_route](<../../../overlay-backend/src/local_ai.rs#L1958-L2053>), when local llama is not reachable on `LLAMA_PORT` ("8080"), the function calls `launch_hidden(&exe, &arg_refs)`. If spawning succeeds, the returned `Child` is immediately appended to `started.push(child)` without checking whether the newly spawned process successfully bound to the port or whether another process is already listening on it.
The same pattern applies to the Whisper server on `WHISPER_PORT` ("8081"): `started.push(child)` happens immediately upon `launch_hidden` success without PID port ownership verification.

## Limits

No native port collision was induced, no server binaries were executed, and no Windows network socket checks were performed. Original statuses in `candidates.json` remain `hypothesis`.
