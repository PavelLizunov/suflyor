# Original STT C03/C04: bounded GigaAM runtime mutex and diarization coordination evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_stt_diarization_runtime_hypotheses.py) inspect frozen speech-to-text inference and diarization coordination sources. They do not load ONNX models, perform live audio transcription, or execute the diarization sidecar. C03 and C04 remain hypotheses.

## C03 — GigaAM global model mutex and failure behavior

In `stt.rs`, [GIGAAM_CACHE](<../../../overlay-backend/src/stt.rs#L745-L761>) is a process-global `OnceLock<Mutex<Option<(String, SharedGigaamModel)>>>`.
[validate_gigaam_dir](<../../../overlay-backend/src/stt.rs#L215-L219>) invokes `shared_gigaam_model(model_dir)` synchronously on the calling thread, seeding the global cache.
During live capture in [start_stt](<../../../overlay-backend/src/stt.rs#L316-L339>), if `shared_gigaam_model` fails, `gigaam` is set to `None`. In the chunk processing loop, if `gigaam` is `None` and the configured backend was GigaAM, `http_target` is also `None`, resulting in speech chunks being dropped without automatic cloud fallback.

## C04 — Diarization sidecar invocation and duration limits

In `diarize.rs`, [run_sidecar](<../../../overlay-backend/src/diarize.rs#L405-L425>) executes the `suflyor-tts diarize` command using `Command::new(exe)...output()`, which blocks synchronously on process completion without an internal timeout or abort handle.
In [guard_wav_len](<../../../overlay-backend/src/diarize.rs#L358-L373>), audio file duration is capped by `MAX_DIAR_SECS` (3 hours = 10,800 seconds).
In [system_windows](<../../../overlay-backend/src/diarize.rs#L432-L445>), speech utterance windows are capped by `WINDOW_CAP_MS` (30,000 ms).

## Limits

No ONNX runtime crash was induced, no large WAV files were diarized, and no subprocess hangs were simulated. Original statuses in `candidates.json` remain `hypothesis`.
