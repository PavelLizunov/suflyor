# Original bridge C03/C04: bounded generation fence TOCTOU and post-stop stamping evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_bridge_generation_fence_hypotheses.py) inspect frozen Slint session forwarder and auto-tile spawning logic. They do not run the Slint UI thread, spawn Tokio runtimes, or perform live AI requests. C03 and C04 remain hypotheses.

## C03 — pre-AI mutations before post-AI generation check

In `slint_session.rs`, [maybe_spawn_auto_tile](<../../../slint-experiment/src/slint_session.rs#L870-L990>) receives `session_gen: u64` from the caller. It does not check whether `lock(&rt).session_gen == session_gen` at function entry.
Prior to awaiting `ai::complete_with_usage_endpoint`, it mutates session state:
- appends timestamps to `s.recent_tile_triggers` under lock;
- prunes and pushes normalized prefixes into `s.recent_question_prefixes`.

Only after the AI completion returns does [post-AI check](<../../../slint-experiment/src/slint_session.rs#L1280-L1315>) verify:
`if lock(&rt).session_gen != session_gen { return; }`.
If the check passes, the lock is dropped and re-acquired separately to insert into `s.qa_cache` and update `s.session_cost_microcents`.

## C04 — forwarder task abort unwind vs post-stop generation stamping

In [transcript_forwarder](<../../../slint-experiment/src/slint_session.rs#L748-L774>), the forwarder reads lines from `stt_rx`.
When [stop_session](<../../../slint-experiment/src/slint_session.rs#L1396-L1430>) is called, it increments `s.session_gen = s.session_gen.wrapping_add(1)` and calls `h.abort()` on `s.transcript_task`.
Because `abort()` notifies the task but only takes effect when the task reaches an `.await` cancellation point, synchronous code in `transcript_forwarder` up to `tokio::spawn` executes with the **incremented** generation `let gen_for_tile = lock(&rt).session_gen`.
The spawned `maybe_spawn_auto_tile` task therefore receives the post-stop generation, allowing its terminal generation check (`lock(&rt).session_gen == session_gen`) to succeed if no further session state transitions occurred.

## Limits

No native thread races or task interleavings were induced, and no AI completion calls were executed. Original statuses in `candidates.json` remain `hypothesis`.
