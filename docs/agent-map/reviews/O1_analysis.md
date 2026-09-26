> **TL;DR:** Suflyor enforces strict process isolation across three OS processes — `overlay-host` (in-process `ort`/GigaAM STT), `suflyor-tts` (sherpa-onnx Piper TTS + diarization), and `suflyor-teratts` (ort TeraTTSv2) — because two statically-linked ONNX Runtime builds in one binary crash natively on the second model load. IPC uses a line-oriented stdin/stdout protocol with base64-encoded text payloads, generation-based cancellation, and a crash-counted auto-respawn with Piper fallback.

---

# Exhaustive Analysis: Process, IPC, and ONNX Runtime Isolation

## 1. Process Boundary Map

```
┌─────────────────────────────────────────────────────────────────┐
│  overlay-host.exe  (slint-experiment)                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  overlay-backend (path dep)                              │   │
│  │  ┌─────────────────────┐  ┌────────────────────────────┐ │   │
│  │  │ stt.rs              │  │ tts.rs                     │ │   │
│  │  │ transcribe-rs/ort   │  │ Sidecar client (stdin/out) │ │   │
│  │  │ GigaAM-v3 INT8      │  │ No sherpa-onnx, no ort    │ │   │
│  │  │ DirectML / CoreML   │  │ Pure base64 + line framing │ │   │
│  │  └─────────────────────┘  └─────┬──────────┬───────────┘ │   │
│  └──────────────────────────────────┼──────────┼────────────┘   │
└─────────────────────────────────────┼──────────┼────────────────┘
              stdin/stdout pipes      │          │
         ┌────────────────────────────┘          │
         ▼                                       ▼
┌────────────────────────┐    ┌──────────────────────────────────┐
│  suflyor-tts.exe       │    │  suflyor-teratts.exe             │
│  sherpa-onnx (static)  │    │  ort =2.0.0-rc.13 (static)      │
│  Piper VITS voices     │    │  TeraTTSv2 4-graph pipeline      │
│  Speaker diarization   │    │  44.1 kHz, ~370 MB models        │
│  22.05 kHz output      │    │  Dedicated synth worker thread   │
│  suflyor-wsola stretch │    │  suflyor-wsola stretch           │
│  WASAPI / cpal output  │    │  WASAPI / cpal output            │
└────────────────────────┘    └──────────────────────────────────┘
```

### Three binaries, three ONNX runtimes, three address spaces

| Process | Crate | ONNX binding | Model | Sample rate |
|---|---|---|---|---|
| `overlay-host` | `overlay-backend` → `transcribe-rs` | `ort` (DirectML/CoreML) | GigaAM-v3 INT8 STT | 16 kHz input |
| `suflyor-tts` | `suflyor-tts` | `sherpa-onnx` (static, CPU) | Piper VITS voices + pyannote/WeSpeaker diar | 22.05 kHz output |
| `suflyor-teratts` | `suflyor-teratts` | `ort =2.0.0-rc.13` (CPU) | TeraTTSv2 4-graph pipeline | 44.1 kHz output |

**`suflyor-tts` also serves a second role** as a diarization engine: `suflyor-tts diarize <wav> --seg <seg.onnx> --emb <emb.onnx>` runs a one-shot speaker diarization in a **separate process invocation** (`diarize.rs:1–14`). A live read-aloud stdin loop and a diarization batch never share an address space (enforced by the subcommand dispatch at `main.rs:131–134`).

---

## 2. The Fundamental ONNX Runtime Invariant

### Why two ONNX runtimes crash in one process

The root cause is documented identically across **six independent locations**:

- `suflyor-tts/Cargo.toml:4–5`: *"two onnxruntimes statically linked into one binary collide (native access-violation on the 2nd model load)"*
- `suflyor-tts/src/main.rs:4–6`: *"its onnxruntime never shares a binary with the main app's `ort`/GigaAM STT runtime (the two collide when static-linked together → native crash)"*
- `suflyor-teratts/Cargo.toml:4–5`: *"two different onnxruntime builds must never share one binary"*
- `overlay-backend/Cargo.toml:32–37`: *"crashes natively when static-linked into the same binary as the `ort`/GigaAM STT runtime (two onnxruntimes collide on the 2nd model load)"*
- `overlay-backend/src/tts.rs:3–4`: *"two statically-linked onnxruntimes collide and crash natively on the second model load"*
- `overlay-backend/src/diarize.rs:2–3`: *"sherpa's onnxruntime can't share ours"*

### The technical mechanism

Both `sherpa-onnx` (used by suflyor-tts) and the `ort` crate (used by overlay-backend and suflyor-teratts) **statically link** the ONNX Runtime C library. Each static link embeds the full `onnxruntime` native code with its global state — environment initialization, thread pools, allocator registries, and execution provider singletons.

When two such static copies coexist in a single binary:

1. **Symbol collision**: Both expose the same C symbol names (`OrtGetApiBase`, `OrtCreateEnv`, etc.). On Windows with MSVC static linking, this produces linker errors or, worse, silent symbol resolution to whichever copy the linker encounters first.

2. **Double ORT environment initialization**: ONNX Runtime uses a process-global singleton `OrtEnv`. The first model load succeeds (initializes the global). The second model load from the *other* static copy attempts to re-initialize with incompatible version/config state → **native access violation** (STATUS_ACCESS_VIOLATION / SIGSEGV).

3. **This is not a Rust-level error** — it's a native crash inside the C runtime with no recoverable handler. The process dies immediately.

### Why `ort` ≠ `sherpa-onnx` but `ort` = `ort` also crashes

- `suflyor-tts` uses `sherpa-onnx` which bundles onnxruntime 1.x internals
- `overlay-backend` uses `ort` (via `transcribe-rs`) which bundles onnxruntime 2.x
- `suflyor-teratts` uses `ort =2.0.0-rc.13` directly

Any two of these in one process crash. **Even two `ort` instances** (overlay-backend's GigaAM + suflyor-teratts's TeraTTS) would crash because `transcribe-rs` and `suflyor-teratts` pin different ort versions/features. The invariant is: **exactly one ONNX runtime per OS process, period.**

### The profile.release constraint

`slint-experiment/Cargo.toml:184`: `panic = "abort"` is **deliberately NOT set** — *"GigaAM's mutex-poison recovery relies on unwinding."* This means the in-process GigaAM STT can catch panics through `catch_unwind` (used in `suflyor-teratts/src/main.rs:476` for synthesis), but a native ONNX collision is below the Rust panic mechanism and kills the process outright.

---

## 3. IPC Protocol: Stdin/Stdout Line Commands

### Protocol overview

Both sidecars share a **byte-compatible** line protocol: one UTF-8 command per line on stdin, one event per line on stdout. The host driver in `overlay-backend/src/tts.rs` uses the same `Sidecar` struct for both engines.

### Stdin commands (host → sidecar)

| Command | Format | Semantics | Validation |
|---|---|---|---|
| `SPEAK` | `SPEAK <base64-utf8>` | Synthesize + play, interrupting current speech | base64 decode → UTF-8 decode; failures → `FAILED`/`REJECTED` |
| `PAUSE` | `PAUSE` | Pause current playback | No-op if nothing playing |
| `RESUME` | `RESUME` | Resume paused playback | No-op if not paused |
| `STOP` | `STOP` | Stop current + clear pending chunks | Emits `DONE` for active utterance |
| `VOICE` | `VOICE <id>` | Select voice model | Path traversal rejected (`engine.rs:117–130`); unknown voice → `REJECTED` (teratts) |
| `RATE` | `RATE <-10..10>` | Set synthesis speed | Clamped; out-of-range → rejected |
| `SEEK` | `SEEK <-30..30>` | Relative seek in seconds | Clamped to retained PCM timeline |
| `SPEED` | `SPEED <50..300>` | Pitch-preserving playback speed % | Via suflyor-wsola time-stretch |
| `LANG` | `LANG <ru\|en>` | Language tag (teratts only) | Only `ru`/`en` accepted; `de` etc. → `REJECTED` |

### Why base64?

Text payloads use base64 encoding (`tts.rs:1256`, `main.rs:117–118`) because:
1. The protocol is **one command per line** — user text containing `\
` would break framing
2. Arbitrary Unicode must round-trip without escaping issues
3. The sidecar never needs to interpret the text structure — it just decodes and synthesizes

### Stdout events (sidecar → host)

**suflyor-tts** (Piper):
```
READY                           # Handshake — process alive
STARTED id=<n>                  # Utterance playback begun
DONE id=<n>                     # Normal finish or interruption
FAILED id=<n> reason=<token>    # Synthesis or playback failed
```

**suflyor-teratts** (Tera) — superset:
```
READY engine=tera revision=<hex> voices=<csv> sample_rate=44100 state=<ready|not-installed|error>
STARTED id=<n>                  # Utterance accepted
PLAYING id=<n>                  # First PCM reached the audio device
DONE id=<n>                     # Natural finish or interruption
FAILED id=<n> reason=<token>    # Synthesis cannot run
REJECTED reason=<token>         # Input validation failed
```

### Protocol invariants

1. **Every STARTED gets exactly one terminal event** (`DONE` or `FAILED`) — enforced by `suflyor-teratts` tests (`main.rs:884–916`): across supersession, stop, and synth-failure, each utterance id gets exactly one terminal.

2. **Stdout is single-writer** — only the worker thread writes stdout, so lines never interleave (`suflyor-teratts/src/main.rs:24`).

3. **No secrets or user text on stdout** — status lines carry only ids, fixed tokens, and counts (`protocol.rs:16–17`). Error reasons are filtered to `[a-zA-Z0-9_-]` by `reason_token()` (`main.rs:142–155`).

4. **VOICE path traversal prevention** — `suflyor-tts` validates voice directory names reject `..`, `/`, `\`, `:` (`engine.rs:117–130`, tested at `main.rs:395–403`). `suflyor-teratts` validates against its installed voice list.

### Wire framing implementation

**Host side** (`tts.rs`, `Sidecar::write_raw`):
```rust
fn write_raw(&mut self, line: &str) -> bool {
    if let Some(si) = self.stdin.as_mut() {
        if writeln!(si, "{line}").and_then(|_| si.flush()).is_ok() {
            return true;
        }
    }
    self.stdin = None;  // Broken pipe → drop handles
    self.proc = None;
    false
}
```

**Sidecar side** (`main.rs:141–151`, `protocol.rs:74–142`):
```rust
// stdin reader thread — BufRead::lines() handles \
 framing
for line in stdin.lock().lines() {
    let Ok(line) = line else { break };  // EOF → shutdown
    match protocol::parse_cmd(&line) {
        Ok(Some(cmd)) => /* dispatch */,
        Ok(None) => continue,       // blank lines ignored
        Err(reason) => /* REJECTED */,
    }
}
let _ = tx.send(Message::Shutdown);     // EOF on stdin → exit
```

---

## 4. Generation-Based Cancellation and STT Suppression

### The problem

Read-aloud competes with real-time STT: when the overlay speaks through the speakers, the microphone picks up the TTS output and the STT engine transcribes it as meeting speech. This creates a feedback loop ("read-aloud text appeared on the bar AFTER playback" — `tts.rs:262–268`).

### The solution: dual-layer generation tracking

**Layer 1: Host speaking state** (`SPEAKING_STATE` mutex in `tts.rs:141–159`)

```rust
struct SpeakingState {
    until_ms: u64,              // Estimated playback end (generous)
    stt_tail_until_ms: u64,     // Post-DONE drain tail (400ms)
    paused_remaining_ms: u64,   // Frozen remaining when paused
    paused: bool,
    active_generation: u64,     // Non-zero while an utterance is active
    loading_generation: u64,    // Cleared by PLAYING event
    next_generation: u64,       // Monotonic counter
}
```

`mark_speaking_for(chars)` estimates playback duration as:
```
synth_latency(1.5s) + chars / (base_cps × synthesis_speed × playback_speed) + tail_cooldown(2s)
```

The estimate is **deliberately generous** — it errs toward suppressing STT too long rather than allowing TTS audio to leak into the transcript.

**Layer 2: Per-sidecar PlaybackTracker** (`tts.rs:483–545`)

Maps the sidecar's process-local utterance ids (`id=1`, `id=2`, ...) to the host's suppression generations. This mapping is critical because:

- Each sidecar respawn resets its utterance counter to 1
- The host's generation counter is monotonic across respawns
- A delayed `DONE id=7` from a dead sidecar must not unmute STT over a newer utterance on a respawned sidecar

```rust
struct PlaybackTracker {
    pending: VecDeque<u64>,          // generations waiting for STARTED
    utterances: BTreeMap<u64, u64>,  // sidecar_id → host_generation
}
```

**Layer 3: Sidecar-internal cancellation** (suflyor-teratts only)

The Tera sidecar has a **dedicated synth worker thread** with an `Arc<AtomicU64>` generation counter (`main.rs:181`). When STOP or a newer SPEAK arrives:

1. The generation is atomically invalidated (`close_active()` stores 0)
2. The playback player is stopped immediately
3. The synth worker checks `generation.load()` before each chunk — stale results are **dropped before they reach playback** (`synth_worker()` at `main.rs:473`)

This means STOP is **observable within one protocol loop iteration** — it never waits for a CPU-heavy synthesis chunk to complete.

### Terminal event flow (DONE/FAILED)

```
Sidecar stdout    Host status thread           Host SPEAKING_STATE
─────────────     ──────────────────           ───────────────────
STARTED id=41  →  tracker.started(41)=gen17  → arm_terminal_watchdog(gen17)
                                                 (extends until_ms to 3× estimate)
PLAYING id=41  →  tracker.generation(41)=17  → clear_loading_generation(17)
DONE id=41     →  tracker.terminal(41)=gen17 → clear_speaking_generation(17)
                                                 stt_tail_until_ms = now + 400ms
```

**Missing terminal event**: The watchdog (`terminal_watchdog_until`) sets `until_ms` to `max(3× remaining, 60s)` when STARTED arrives. If neither DONE nor FAILED ever comes (sidecar crash without EOF), the estimate eventually expires and STT unmutes. The crash is detected on the next `ensure()` call.

---

## 5. Process Respawning and Failure Recovery

### Lazy spawn + auto-respawn (`Sidecar::ensure()`, `tts.rs:869–939`)

```
First SPEAK ──► ensure() ──► spawn_engine_sidecar()
                              ├─ cmd.stdin(Piped).stdout(Piped).stderr(logfile)
                              ├─ CREATE_NO_WINDOW (no console popup)
                              ├─ assign_to_lifetime_job() (Windows JobObject)
                              └─ stdout reader thread (piper-status / teratts-status)
              ├─ Re-apply: LANG, VOICE, RATE, SPEED (state survives respawn)
              └─ If exe missing → log warning, skip spawn
```

**Crash detection** happens inside `ensure()`:
```rust
let alive = self.proc.as_mut()
    .map(|p| matches!(p.try_wait(), Ok(None)))
    .unwrap_or(false);
if alive { return; }
if self.proc.is_some() {
    self.crashes += 1;  // Dead child found → crash count
}
```

### Crash limit and fallback (`TERA_CRASH_LIMIT = 3`)

After 3 Tera crashes in one session, the engine is **bypassed** — `crashed_out()` returns true. Read-aloud automatically falls back to Piper for the remainder of the session. This prevents an infinite crash-respawn loop on a broken model.

```rust
// In Tts::speak():
if self.engine_kind() == EngineKind::Tera {
    if self.tera_usable() {
        if self.send_tera_speak(...) { return true; }
        log::warn!("Tera sidecar did not accept SPEAK — falling back to Piper");
    }
}
// Automatic Piper fallback
if self.send_piper_speak(...) { return true; }
```

### Control commands never respawn (`send_if_alive`)

PAUSE, RESUME, and STOP use `send_if_alive()` instead of `send()` — they never trigger a sidecar respawn. Rationale (`tts.rs:1020–1025`): *"respawning a dead sidecar just to deliver a control line would boot the whole engine for nothing (a Tera cold load is hundreds of MB of graphs), and a dead sidecar is not playing anything anyway."*

### Graceful shutdown

**EOF-driven exit**: When the parent process (`overlay-host`) exits, its pipe handles close. The sidecar's stdin reader thread hits EOF and sends `Message::Shutdown`, which breaks the main loop. Both sidecars **explicitly stop playback** on shutdown:

```rust
// suflyor-tts/src/main.rs:344–348
// stdin closed (the app exited / was closed): STOP immediately so speech
// does not keep playing after the app is gone (the tester hit read-aloud
// continuing after closing the app).
if let Some((_, pb)) = current.take() {
    pb.stop();
}
```

```rust
// suflyor-teratts/src/main.rs:513–514
controller.close_active();  // stops playback, emits terminal DONE
```

**Windows JobObject** (`tts.rs:1601`): `assign_to_lifetime_job(&proc)` registers both sidecars with the parent's JobObject, so even a `TerminateProcess` (Task Manager kill) of `overlay-host` automatically kills the sidecars — preventing orphaned speech processes.

### Host-side EOF handling (`handle_playback_eof`, `tts.rs:629–640`)

When the status reader thread detects stdout EOF (sidecar died), it drains all tracked generations and clears their speaking state. This ensures a crashed sidecar **never leaves STT permanently muted**:

```rust
fn handle_playback_eof(engine: EngineKind, playback: &Mutex<PlaybackTracker>) {
    let generations = playback.lock()...drain_generations();
    for generation in generations {
        clear_speaking_generation(generation);
    }
}
```

---

## 6. Write-Failure Atomicity

A critical design point: the host must **never** mark STT suppression for a SPEAK that wasn't delivered. The `send_tera_speak` / `send_piper_speak` methods implement a two-phase commit (`tts.rs:969–1018`):

```
1. mark_speaking_for(chars)         ← estimate suppression window
2. tracker.register_speak(generation) ← register in pending queue
3. write_raw(SPEAK line)            ← attempt delivery
   ├─ Success → return true (suppression stays)
   └─ Failure → tracker.cancel_speak(generation)
                clear_speaking_generation(generation)
                return false (suppression rolled back)
```

This ensures a broken pipe (dead sidecar between spawn and write) cannot leave the mic falsely suppressed.

---

## 7. Diarization Process Isolation

Diarization runs as a **separate, one-shot process invocation** of the same `suflyor-tts` binary:

```
overlay-backend (diarize.rs)
    └─ Command::new("suflyor-tts")
           .args(["diarize", "system.wav", "--seg", "seg.onnx", "--emb", "emb.onnx"])
           .stdout(Piped)
           .spawn()
```

This is **not** the same process as the read-aloud sidecar. The subcommand dispatch in `main.rs:131–134` routes `diarize` to `diar::run_cli()` which exits immediately after printing JSON to stdout. The invariant from `suflyor-tts/AGENTS.md`: *"A live read-aloud and a diarize batch never share an address space."*

The diarization sidecar resolves its own exe path through `diarize.rs` (not through `tts.rs`'s `sidecar_exe_path()`), keeping the read-aloud and diarization paths independent.

---

## 8. Summary of Failure Recovery Semantics

| Failure mode | Detection | Recovery | STT impact |
|---|---|---|---|
| Sidecar crash mid-utterance | `try_wait()` in next `ensure()` | Auto-respawn (crash counter incremented) | `handle_playback_eof` drains all generations → STT unmuted |
| Sidecar crash ≥3× (Tera) | `crashed_out()` check | Tera bypassed; Piper fallback for session | No STT impact (Piper used) |
| Sidecar exe not found | `exe.is_file()` in `ensure()` | Log warning; `speak()` returns false | No suppression marked |
| Broken pipe on write | `writeln!` + `flush()` returns Err | Handles dropped; generation rolled back | No suppression marked |
| Missing terminal event | Watchdog in `arm_terminal_watchdog` | `until_ms` set to `max(3× remaining, 60s)` → eventual expiry | STT unmuted after watchdog expires |
| Stale DONE from old sidecar | `clear_speaking_generation(gen)` | Only clears if `gen == active_generation` | Newer utterance unaffected |
| Parent process killed | Windows JobObject | Sidecars auto-killed by OS | N/A (process dead) |
| Parent process exits normally | stdin pipe closed → EOF | Sidecars drain playback, stop, exit | N/A |
| `panic!` in Tera synth worker | `catch_unwind` in `synth_worker` | Worker thread exits; dispatch fails future jobs | Utterance gets FAILED terminal |
| GigaAM STT accelerator failure | `transcribe-rs` load error | Auto-fallback from DirectML/CoreML to CPU | Degraded performance, still functional |
