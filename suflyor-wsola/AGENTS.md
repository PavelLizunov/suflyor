# `suflyor-wsola` Crate Agent Guide

This guide extends the repository root `AGENTS.md`; do not duplicate root workflow or DSH/homelab policy here.

## Purpose
`suflyor-wsola` is Suflyor's pitch-preserving WSOLA (Waveform Similarity Overlap-Add) time-stretching helper library crate, derived from `timestretch` 0.5.0 (MIT licensed). It performs mono PCM sample stretching for live speech playback and transcript replay without changing pitch.

## Code Map

### Core Modules
- `src/lib.rs`: Crate root exposing `Wsola`, `StreamingWsola`, and `WsolaError`. Enforces `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` for production code.
- `src/wsola.rs`: Core WSOLA processor (`Wsola`). Manages time-domain cross-correlation, FFT-accelerated candidate search (`rustfft`), precomputed raised-cosine / equal-power crossfading, sub-sample parabolic interpolation, and non-allocating processing buffers.
- `src/stream.rs`: Streaming wrapper (`StreamingWsola`). Manages chunk boundaries for continuous live speech streaming, input overlap re-feeding, held output crossfade tails (16 ms time-based crossfade out), and final tail flushing (`finish()`).
- `src/error.rs`: Custom error type (`WsolaError`) covering invalid stretch ratio, input too short, buffer overflow, and invalid processor state.

## Realtime, Allocation, and Performance Invariants
1. **Zero Hot-Path Allocations (Pre-Reserved)**: Calling `Wsola::reserve_output_capacity(input_len, max_ratio)` pre-allocates internal output storage (`output_buf`). `process_into_no_grow` and `process_into` process without growing internal allocations.
2. **Buffer Reuse & Memory Swapping**: Reusable scratch buffers (`fft_fwd_scratch`, `fft_inv_scratch`, `fft_ref_buf`, `fft_search_buf`, `fft_corr_buf`, `prefix_sq_buf`, `corr_values_buf`, `norm_corr_values_buf`) grow lazily on demand and are zero-filled/reused across iterations via `std::mem::take`.
3. **Adaptive Correlation Engine**:
   - Direct search for small candidate sets (`num_candidates <= 64` or `overlap_len < 32`), using 8-way unrolled SIMD-friendly loops (`sum_and_square_sum`, `sum_cross_terms`).
   - FFT-accelerated cross-correlation via `rustfft` for large candidate sets (`num_candidates > 64` and `overlap_len >= 32`).
4. **Adaptive Overlap & Crossfading**:
   - Dynamic overlap adjustment (`overlap_for_ratio`): uses `segment_size / 4` for near-unity ratios (±15%) to reduce transient smearing, and `segment_size / 2` for larger ratios.
   - Raised-cosine crossfade by default; equal-power crossfade option (`set_equal_power_crossfade`) for uncorrelated/noise content.
5. **Sub-Sample Alignment**: Parabolic interpolation (`parabolic_interpolation`) refines correlation peaks for sub-sample accuracy; linear interpolation (`subsample_interpolate`) avoids pitch drift during overlap-add.

## API Consumers
- `suflyor-tts`: `src/playback.rs` and `src/playback_macos.rs` use `StreamingWsola` to stretch neural TTS speech audio output in real-time according to playback speed settings.
- `suflyor-teratts`: `src/playback.rs` and `src/playback_macos.rs` use `StreamingWsola` for real-time TeraTTS audio playback stretching.
- `slint-experiment`: `src/bin/overlay_host/transcript_player.rs` uses `Wsola` for variable-speed speech replay.

## Focused Benchmarks and Tests
Run these on the appropriate homelab worker against the exact candidate SHA, not on the DSH control plane:
- **Single-crate tests**: `cargo test --manifest-path suflyor-wsola/Cargo.toml`
- **Single-crate clippy**: `cargo clippy --manifest-path suflyor-wsola/Cargo.toml --all-targets -- -D warnings`
- **Single-crate fmt check**: `cargo fmt --manifest-path suflyor-wsola/Cargo.toml --all -- --check`
- **Test coverage**:
  - `src/stream.rs`: Tests streaming chunk continuity, finite output, empty input no-ops, and sample-rate-adaptive boundary crossfading.
  - `tests/speech_contract.rs`: Tests 1.0x bit-exact length and determinism, speech speed ratios (1.0x–3.0x) within timeline bounds (±1/25s tolerance), and output finiteness.
