# Original TTS C01/C02: bounded stdio line buffer and pipe deadlock evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_tts_stdio_pipe_hypotheses.py) inspect frozen TTS host and sidecar sources. They do not spawn sidecars, run audio synthesis, or execute speech playback. C01 and C02 remain hypotheses.

## C01 — base64 single-line encoding and unconstrained stdin line ingestion

In `tts.rs`, [speak](<../../../overlay-backend/src/tts.rs#L1251-L1275>) normalizes speech text, base64-encodes the entire string in one pass via `base64::engine::general_purpose::STANDARD.encode(&spoken)`, and sends it as a single line `SPEAK <b64>` without chunking or maximum character clamping.
In [write_raw](<../../../overlay-backend/src/tts.rs#L950-L959>), the host writes the line into the child's stdin pipe and immediately flushes via `writeln!(si, "{line}").and_then(|_| si.flush())`.
Both sidecar entrypoints ([suflyor-tts](<../../../suflyor-tts/src/main.rs#L140-L151>) and [suflyor-teratts](<../../../suflyor-teratts/src/main.rs#L648-L662>)) read commands using standard `stdin.lock().lines()`, growing an unbounded `String` in memory until the terminating newline byte `\n`.

## C02 — sidecar stdout background drain thread and pipe deadlock avoidance

When spawning sidecar processes in `tts.rs`, [spawn loop](<../../../overlay-backend/src/tts.rs#L892-L921>) immediately extracts `child.stdout` and spawns a dedicated background OS thread (`std::thread::Builder::new().name(thread_name).spawn(...)`).
This thread continuously consumes child stdout lines with `BufReader::new(stdout).lines()` and dispatches them to `handle_playback_line`, preventing OS pipe buffer saturation and write deadlocks while the main thread writes commands to child stdin.

## Limits

No sidecar process was launched, no multi-megabyte payloads were transmitted over OS pipes, and no ONNX synthesis was performed. Original statuses in `candidates.json` remain `hypothesis`.
