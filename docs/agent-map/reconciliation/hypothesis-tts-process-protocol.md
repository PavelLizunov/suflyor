# Original TTS C03/C04: bounded process cleanup and protocol desynchronization evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_tts_process_protocol_hypotheses.py) inspect frozen TTS process management and protocol tracking logic. They do not kill processes, spawn children, or inject protocol desynchronization faults. C03 and C04 remain hypotheses.

## C03 — Windows JobObject assignment vs POSIX process cleanup

In `tts.rs`, [spawn_engine_sidecar](<../../../overlay-backend/src/tts.rs#L1594-L1604>) invokes `crate::local_ai::assign_to_lifetime_job(&proc)` under `#[cfg(windows)]` only.
On non-Windows/POSIX platforms, the child process is not assigned `kill_on_drop`, `PR_SET_PDEATHSIG`, or process-group signal forwarding.
Furthermore, the `Sidecar` struct does not implement an explicit `Drop` trait to call `kill()` or `wait()`. Process termination relies on child exit upon reading EOF on stdin when the parent pipe closes. [crashed_out](<../../../overlay-backend/src/tts.rs#L941-L945>) tracks repeated sidecar failures against `TERA_CRASH_LIMIT = 3`.

## C04 — PlaybackTracker FIFO pending tracking and rejected event handling

In `tts.rs`, [PlaybackTracker](<../../../overlay-backend/src/tts.rs#L484-L545>) manages correlation between host generation IDs and sidecar utterance IDs.
When the sidecar emits `STARTED id=<n>`, [started](<../../../overlay-backend/src/tts.rs#L501-L505>) pops the head generation from the FIFO queue (`self.pending.pop_front()?`) and binds it to `id`.
In [rejected](<../../../overlay-backend/src/tts.rs#L533-L538>), only `invalid-base64` and `invalid-utf8` rejections pop from `pending`. Other rejection reasons leave the pending queue intact to avoid consuming future utterance generations.

## Limits

No native process termination was tested under POSIX, no child crash loops were induced, and no out-of-order protocol streams were executed. Original statuses in `candidates.json` remain `hypothesis`.
