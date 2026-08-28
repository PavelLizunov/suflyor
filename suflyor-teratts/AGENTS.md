# `suflyor-teratts` Sidecar Crate Agent Guide

This guide extends the repository root `AGENTS.md`; do not duplicate root workflow or DSH/homelab policy here.

## Purpose
`suflyor-teratts` is Suflyor's experimental TeraTTSv2 neural read-aloud sidecar binary (`suflyor-teratts.exe`). It links ONNX Runtime through the `ort` crate exclusively (`=2.0.0-rc.13`) to perform 44.1 kHz Russian/English speech synthesis using pinned TeraTTSv2 ONNX graphs without sharing a process space with `overlay-host` or `suflyor-tts`.

## Code Map

### Core Modules
- `src/main.rs`: Sidecar entry point, CLI `status` mode check, stdin command reader loop, stdout event emitter (`emit`), generation-based cancellation controller (`Controller`), and async worker message router (`worker`, `synth_worker`).
- `src/protocol.rs`: Line-protocol parser (`parse_cmd`) and event serializer (`Event::to_line`). Supports standard TTS commands (`SPEAK`, `PAUSE`, `RESUME`, `STOP`, `SEEK`, `SPEED`, `RATE`, `VOICE`) plus additive language tag command `LANG <ru|en>`.
- `src/tera.rs`: `TeraEngine` graph orchestration for 4 ONNX models (`text_encoder.onnx`, `duration_predictor.onnx`, `sampler_distilled_cfg3_8step.onnx`, `vocoder.onnx`). Enforces single-declared-output validation per graph and causal overlap-save vocoder streaming (20-frame context, 16-frame chunks at 44.1 kHz).
- `src/manifest.rs`: Compiles in `manifest/teratts-v2.json` (`Manifest::pinned`), validates 40-char git revision digest and per-file size pins, calculates release directory path (`teratts-v2-<revision>`), and scans installed voice styles (`styles/*/`).

### Text Normalization & Synthesis Helpers
- `src/textnorm.rs`: Russian/English text normalization, language tag handling (`ensure_language_tags`), and stress/homograph preparation.
- `src/num2words.rs`: Russian/English cardinal number text expansion.
- `src/chunk.rs`: Text sanitization and sentence/clause chunking (`chunk_text`, `sanitize`) for streaming synthesis.
- `src/indexer.rs`: `UnicodeIndexer` mapping normalized characters to model token IDs using `unicode_indexer.json`.
- `src/npy.rs`: Parser for NPY array headers (`load_f32`) to read `style_dp.npy` and `style_ttl.npy` voice assets.
- `src/rng.rs`: Minimal deterministic pseudo-random number generator (`Rng`) for latent noise initialization.

### Platform Transports
- `src/playback.rs`: Windows WASAPI render transport (`wasapi::Playback`). Handles pitch-preserving WSOLA time-stretching (`suflyor-wsola`), sample feeding, playback control, and playback completion notifications.
- `src/playback_macos.rs`: macOS CoreAudio transport through `cpal`.

### Assets & Manifest
- `manifest/teratts-v2.json`: Pinned release manifest (`TeraSpace/TeraTTSv2` revision `f05ea799094571a3553904a555df3834fb0b963b`, 27 files, ~370 MB total).
- `NOTICE.md`: Licensing release gate documentation detailing upstream notice requirements and redistribution restrictions.

## Invariants
1. **Process Isolation & Ort-Only Runtime**: `suflyor-teratts` links ONNX Runtime via `ort` ONLY. It MUST remain in its own standalone sidecar process. Never link `sherpa-onnx`, `transcribe-rs`, or `overlay-backend` into this crate, and never merge `suflyor-teratts` into `overlay-host` or `suflyor-tts`. Statically linking multiple ONNX runtime instances in one process crashes natively.
2. **On-Demand Model & Upstream Licensing Release Gate**: Upstream `TeraSpace/TeraTTSv2` has NO verified public license file (only unverified Telegram statements and a bundled MIT notice for RUAccent). Weights and voice styles MUST NOT be packaged into the NSIS installer or mirrored on Suflyor release channels without an archived written license grant covering code, weights, styles, RUAccent assets, and commercial redistribution. Assets download on demand to `%APPDATA%\suflyor\tts\teratts-v2-<revision>`.
3. **Cancellation Generation & Asynchronous Synthesis**: Synthesis runs on a dedicated worker thread (`synth_worker`). Utterance IDs act as cancellation generations stored in an `Arc<AtomicU64>`. Issuing `STOP` or a new `SPEAK` invalidates the active generation immediately, stopping playback and dropping stale in-flight synthesis results before they reach the audio player.
4. **Single-Output Graph Schema & Tensor Validation**: Every pinned ONNX graph declares exactly one output tensor. Output shapes and tensor element counts are strictly validated before slicing/indexing to guarantee graph integrity and prevent panics or user text leaks into error tokens.
5. **Clippy Denial Policy**: Production code denies `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic`. Inline test modules (`#[cfg(test)]`) allow them locally.

## Protocol & Handshake

### Read-Aloud Protocol (stdin → stdout)
- **stdin commands** (one line per command):
  - `VOICE <id>`: Select voice style ID (e.g. `ru_f1`, `ru_m5`).
  - `RATE <-10..10>`: Adjust synthesis rate (maps to duration scale).
  - `LANG <ru|en>`: Set language mode tag for untagged text (default `ru`).
  - `SPEAK <base64-utf8>`: Synthesize and play UTF-8 text (interrupts active audio).
  - `PAUSE` / `RESUME` / `STOP`
  - `SEEK <-30..30>`: Relative seek offset in seconds.
  - `SPEED <50..300>`: Set WSOLA playback speed percentage (50% .. 300%).
- **stdout events**:
  - `READY engine=tera revision=<hex> voices=<list> sample_rate=44100 state=<ready|not-installed|error>`
  - `STARTED id=<n>`
  - `PLAYING id=<n>`
  - `DONE id=<n>`
  - `FAILED id=<n> reason=<token>`
  - `REJECTED reason=<token>`

## Targeted Checks
Run these on the appropriate homelab worker against the exact candidate SHA, not on the DSH control plane:
- **Single-crate tests**: `cargo test --manifest-path suflyor-teratts/Cargo.toml`
- **Single-crate clippy**: `cargo clippy --manifest-path suflyor-teratts/Cargo.toml --all-targets`
- **Single-crate fmt check**: `cargo fmt --manifest-path suflyor-teratts/Cargo.toml --all -- --check`
- **Native repository gate**: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/git-gate-native.ps1 manual`
