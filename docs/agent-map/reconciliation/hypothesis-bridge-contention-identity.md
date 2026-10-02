# Original bridge C02/C08: bounded runtime lock contention and debrief/namer identity evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_bridge_contention_identity_hypotheses.py) inspect frozen Slint session locking and session identity tagging logic. They do not run the Slint UI thread, spawn audio streams, or execute live AI requests. C02 and C08 remain hypotheses.

## C02 — shared runtime lock contention across audio, session reset, and cache eviction

In `slint_session.rs`, [forward_audio_chunks](<../../../slint-experiment/src/slint_session.rs#L598-L640>) locks `rt` on each incoming `AudioChunk` to inspect pause/mute state and update mic frame timestamps, and locks `rt` again after `stt_tx.reserve().await` to re-check the pause flag.
In [start_session_inner](<../../../slint-experiment/src/slint_session.rs#L273-L343>), the same `rt` lock is held continuously across bulk state resets, clearing `full_transcript`, `speech_window`, `recent_question_prefixes`, `qa_cache`, and taking `s.journal`.
In [maybe_spawn_auto_tile](<../../../slint-experiment/src/slint_session.rs#L1290-L1305>), when `qa_cache` reaches `QA_CACHE_MAX_ENTRIES` (256), it performs an in-place sort by age and bulk eviction while holding `lock(&rt)`.

## C08 — debrief session ID without generation vs namer generation verification

In [maybe_run_debrief](<../../../slint-experiment/src/slint_session.rs#L1593-L1606>), the debrief task is spawned on `rt_handle` passing `session_id: String` without checking or capturing `session_gen`. If a new session starts while debrief is in-flight, the debrief result surfaces labeled with the old session's ID.
In contrast, in `session_namer.rs`, [maybe_spawn_namer](<../../../slint-experiment/src/session_namer.rs#L99-L127>) captures `s.session_gen` under lock, passes `gen` into the spawned task, and [generate_name completion](<../../../slint-experiment/src/session_namer.rs#L146-L175>) checks `if s.session_gen != gen` before persisting, discarding the title if the session changed during the LLM call.

## Limits

No multithreaded lock contention benchmarks were executed, no audio chunk backlog was measured under load, and no UI thread priority inversions were induced. Original statuses in `candidates.json` remain `hypothesis`.
