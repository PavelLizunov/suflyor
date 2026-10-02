# Original bridge C05/C06: bounded single-flight state and task cleanup evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_bridge_single_flight_cleanup_hypotheses.py) inspect frozen Slint session single-flight permit logic and asynchronous task management. They do not run the Slint UI thread, spawn background threads, or execute live tasks. C05 and C06 remain hypotheses.

## C05 — single-flight permit generation packing, supersession, and rollover boundaries

In `slint_session.rs`, [try_acquire_auto_tile](<../../../slint-experiment/src/slint_session.rs#L118-L148>) packs the session generation into an `AtomicU64` as `session_gen.wrapping_shl(1) | 1`.
When a permit is active for generation `N` and a new generation `N+1` requests a permit, `current_gen < session_gen` holds true. The CAS succeeds, granting the newer generation a permit while the prior task is still executing. When the older permit drops, its CAS fails harmlessly because the atomic state now reflects generation `N+1`.
If `session_gen` were to wrap around from a high value back to a low value, `current_gen > session_gen` would cause `try_acquire_auto_tile` to return `None` until the atomic state is reset.

## C06 — aborted task handles without join and untracked task spawns

In `slint_session.rs`, [stop_session](<../../../slint-experiment/src/slint_session.rs#L1396-L1435>) extracts and calls `h.abort()` on `s.transcript_task`, `s.ai_task`, and `s.health_task`, but does not `.await` or join the handles.
Furthermore, several asynchronous task allocations in `slint_session.rs` do not retain their `JoinHandle`:
- [forward_audio_chunks](<../../../slint-experiment/src/slint_session.rs#L598-L620>) invokes `tokio::spawn` without storing the handle;
- [transcript_forwarder](<../../../slint-experiment/src/slint_session.rs#L748-L774>) spawns `maybe_spawn_auto_tile` on each qualifying line without binding it to `s.ai_task`;
- [maybe_run_debrief](<../../../slint-experiment/src/slint_session.rs#L1585-L1638>) spawns `run_post_meeting_debrief` on `rt_handle` without saving the resulting join handle.

## Limits

No task cancellation races were induced under load, no thread leaks were verified on live runtimes, and no wrapped integer generation counters were executed in production. Original statuses in `candidates.json` remain `hypothesis`.
