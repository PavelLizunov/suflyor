# Original local AI C10/C11: bounded JobObject assignment and Whisper readiness evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_local_ai_job_whisper_readiness_hypotheses.py) inspect frozen process spawning, JobObject assignment, and Whisper readiness checks. They do not spawn processes, create Windows JobObjects, or probe HTTP ports. C10 and C11 remain hypotheses.

## C10 — Windows JobObject limits and best-effort handling

In [launch_hidden](<../../../overlay-backend/src/local_ai.rs#L3081-L3089>), `assign_to_lifetime_job(&child)` is executed only under `#[cfg(windows)]`.
In [assign_to_lifetime_job](<../../../overlay-backend/src/local_ai.rs#L3104-L3146>), the process-wide JobObject is configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` only.
Failures during `CreateJobObjectW`, `SetInformationJobObject`, or `AssignProcessToJobObject` log warnings and return `0` / ignore the error without aborting process launch. No POSIX death-signal mechanics are attached to child processes.

## C11 — Whisper server readiness probe boundaries

During [install](<../../../overlay-backend/src/local_ai.rs#L857-L862>), Whisper server readiness is polled via `wait_ready(&format!("{WHISPER_BASE_URL}/models"), 60)`.
In [wait_ready](<../../../overlay-backend/src/local_ai.rs#L2706-L2720>), curl output is discarded into `dev_null()`, and only the exit status `out.status.success()` is checked.
In [ensure_servers_for_route](<../../../overlay-backend/src/local_ai.rs#L2027-L2048>), Whisper server presence is checked solely via `!is_reachable(&format!("{WHISPER_BASE_URL}/models"))`.
Unlike the llama server checks, neither path verifies that the child process owns the listening socket via PID lookup or checks model identity.

## Limits

No process termination was forced, no nested JobObject collisions were simulated, and no rogue HTTP server was bound to port 8081. Original statuses in `candidates.json` remain `hypothesis`.
