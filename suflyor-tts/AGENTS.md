# `suflyor-tts` Crate Agent Guide

This guide extends the repository root `AGENTS.md`; do not duplicate root workflow or DSH/homelab policy here.

## Purpose
`suflyor-tts` is Suflyor's standalone neural read-aloud (Piper TTS) and offline speaker diarization sidecar binary (`suflyor-tts.exe`). It links `sherpa-onnx` statically to synthesize Russian/English speech and perform speaker diarization over recorded audio without sharing a process space with the main application.

## Code Map

### Core Modules
- `src/main.rs`: Process entry point and subcommand router. Dispatches `diarize` CLI arguments to `diar::run_cli`; otherwise runs the interactive stdin command loop (`SPEAK`, `PAUSE`, `RESUME`, `STOP`, `VOICE`, `RATE`, `SEEK`, `SPEED`) and emits stdout lifecycle events (`READY`, `STARTED`, `DONE`, `FAILED`).
- `src/engine.rs`: VITS `sherpa-onnx` `OfflineTts` engine wrapper (`NeuralEngine`), voice scanning/loading (`scan_voices`, `load_voice`), voice selection (`pick_voice_id`), speed mapping (`rate_to_speed`), and text processing (`text::chunk_text`, `text::sanitize`).
- `src/diar.rs`: Subcommand `suflyor-tts diarize <system.wav> --seg <seg.onnx> --emb <emb.onnx> [--num-speakers N] [--threshold T]`. Runs pyannote segmentation + WeSpeaker embeddings + agglomerative clustering via `sherpa-onnx::OfflineSpeakerDiarization`, post-processes segments (smoothing flicker and merging same-speaker gaps), and outputs JSON to stdout.

### Platform Transports
- `src/playback.rs`: Windows WASAPI render transport (`wasapi`). Plays mono `f32` PCM audio by duplicating samples to L+R stereo (avoiding single-channel device routing issues), integrates `suflyor-wsola` time-stretching, handles seeking, and prunes played timeline history.
- `src/playback_macos.rs`: macOS CoreAudio transport (`cpal`) with fractional-phase linear resampling (`ContinuousResampler`) and ring buffering.
- `src/playback_unavailable.rs`: Seam for unsupported platforms, failing cleanly on start.

### Test & Diagnostic Guards
- Inline unit tests: `parse_cmd`, `PlaybackSpeed`, `take_finished_playback` in `src/main.rs`; `tts_root` in `src/engine.rs`; `to_json`, `parse_args`, `postprocess` in `src/diar.rs`; timeline seeking and buffer draining in `src/playback.rs` and `src/playback_macos.rs`.

## Invariants
1. **Process Isolation & Single ONNX Runtime**: `suflyor-tts` links `sherpa-onnx` ONLY and MUST remain in its own separate process. Never link `ort`, `transcribe-rs`, or `overlay-backend` into this crate, and never merge `suflyor-tts` into `overlay-host`. Statically linking two ONNX runtime builds into a single binary crashes natively on secondary model loading.
2. **Diarization Process Isolation**: A `diarize` run is a separate, short-lived process (`suflyor-tts diarize ...`) invoked by `overlay-backend::diarize`. It must never run inside the main overlay process or share state with an active read-aloud stdin loop.
3. **Clippy Denial Policy**: Production code denies `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic`. Inline test modules (`#[cfg(test)]`) allow them locally.
4. **Stereo Audio Duplication**: Render transports MUST duplicate mono synthesized samples into L+R stereo frames to ensure balanced output across WASAPI and platform render endpoints.
5. **On-Demand Model Weights & License Boundaries**: Model weights (Piper VITS voices, pyannote segmentation, WeSpeaker embeddings) are licensed separately and downloaded on demand into `%APPDATA%\suflyor\` by `overlay-backend` installer routines with SHA-256 verification. Never bundle weights in the executable or NSIS installer.

## Protocol & CLI Contracts

### Read-Aloud Protocol (stdin → stdout)
- **stdin commands** (one per line):
  - `VOICE <dir>`: Select voice model directory name.
  - `RATE <-10..10>`: Adjust synthesis rate (-10 = 0.5x, 0 = 1.0x, +10 = 2.0x).
  - `SPEAK <base64-utf8>`: Synthesize and play UTF-8 text (interrupts current speech).
  - `PAUSE` / `RESUME` / `STOP`
  - `SEEK <-30..30>`: Seek relative offset in seconds.
  - `SPEED <50..300>`: Set WSOLA playback speed percentage (50% .. 300%).
- **stdout events**:
  - `READY`: Emitted once when worker loop starts.
  - `STARTED id=<n>`: Emitted when speech playback begins for utterance `id`.
  - `DONE id=<n>`: Emitted when speech playback finishes or is stopped.
  - `FAILED id=<n> reason=<reason>`: Emitted if synthesis or playback fails.

### Diarization CLI (`diarize` subcommand)
- **Invocation**: `suflyor-tts diarize <system.wav> --seg <seg.onnx> --emb <emb.onnx> [--num-speakers N] [--threshold T]`
- **stdout JSON**: `{"num_speakers":N,"segments":[{"s":start_ms,"e":end_ms,"sp":speaker_id},...]}`
- **Exit code**: `0` on success; `1` on error (with error details on stderr).

## Change Guidance
- **Dependencies**: Keep dependencies minimal (`sherpa-onnx`, `suflyor-wsola`, platform audio). Do not add `serde` or heavy dependencies unless strictly necessary (JSON generation in `diar.rs` is intentionally hand-rolled).
- **Audio Output**: When altering `playback.rs` or `playback_macos.rs`, preserve `suflyor-wsola` time-stretching and timeline pruning to prevent unbounded memory growth during long playback sessions.

## Targeted Checks
Run these on the appropriate homelab worker against the exact candidate SHA, not on the DSH control plane:
- **Single-crate tests**: `cargo test --manifest-path suflyor-tts/Cargo.toml`
- **Single-crate clippy**: `cargo clippy --manifest-path suflyor-tts/Cargo.toml --all-targets`
- **Single-crate fmt check**: `cargo fmt --manifest-path suflyor-tts/Cargo.toml --all -- --check`
- **Native repository gate**: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/git-gate-native.ps1 manual`
