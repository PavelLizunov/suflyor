# O3: Audio Pipeline, STT, Diarization & WSOLA

## 1. Audio Capture Architecture (`overlay-backend/src/audio.rs` & `audio_macos.rs`)
- **Windows WASAPI**:
  - Dual capture loops running on dedicated threads (`audio-system` and `audio-mic`).
  - System audio capture uses the WASAPI loopback technique: opening a render endpoint with `Direction::Capture`.
  - Mic capture uses standard WASAPI capture endpoints.
  - Device follow modes: `FollowDefault` (recovers on default endpoint shifts) vs. `Pinned(name)` (persists across default changes but recovers on disconnect).
- **macOS CoreAudio Seam**:
  - Mic capture uses `AVAudioEngine` input node tap.
  - System audio uses Core Audio process taps with aggregate devices, capturing speaker audio without muting output or requiring virtual drivers (like BlackHole).
  - Real-time safety: Audio callback blocks downmix into lock-free SPSC ring buffers without heap allocations or blocking mutexes.

## 2. Format Normalization & Decimation
- **16 kHz Mono Standard**: Both platforms capture at native device sample rates (typically 48 kHz or 44.1 kHz) and decimate to 16 kHz mono i16:
  - Exact 3:1 integer decimation (48 kHz -> 16 kHz) uses a high-performance 3-sample averaging filter.
  - Non-integer ratios employ fractional bin-averaging decimation.
  - Samples are clamped to `[-1.0, 1.0]` before quantization to prevent numeric wrap-around.
- **Buffer & Channel Sizing**: Decimated chunks are packed into 200ms slices and pushed to a bounded Tokio channel (capacity 128 chunks ≈ 25s). If downstream STT stalls, chunks are dropped and counted rather than back-pressuring the realtime OS audio loop.

## 3. VAD & Anti-Hallucination Pipeline (`overlay-backend/src/stt.rs`)
- **RMS Energy Gate**: RMS threshold set to 50 (i16 scale) with 800ms silence hang time. Max buffer duration capped at 10 seconds.
- **Voice Onset Correction**: Utterance `start_ts_ms` snaps to the first voiced chunk minus one window to avoid inaccurate timestamps during quiet meeting openings.
- **Anti-Hallucination Filtering**:
  - Two-stage pre-STT gate requiring both overall RMS ≥ 30 and ≥ 25% voiced sub-windows.
  - Post-STT text heuristics discarding common Whisper hallucinations ("Спасибо за просмотр", "Субтитры создавал...", repetition loops).
- **TTS Anti-Feedback**: Both microphone and system audio channels are suppressed from STT ingestion while local TTS playback is active, preventing the assistant from transcribing its own speech.

## 4. Session Audio Recording & Repair (`overlay-backend/src/recorder.rs`)
- **Separate Channels**: Writes `mic.wav` and `system.wav` per session in the background via non-blocking channels.
- **Timeline Alignment (Padding)**: Loopback silence gaps are padded with synthetic zero-samples so WAV sample offsets strictly match elapsed session wall-clock time (capped at 10m per gap / 30m total).
- **Crash Recovery**: If the application terminates abruptly, unfinalized WAV files lack RIFF header sizes. On next startup, `repair_unfinalized_in()` inspects file lengths and patches valid canonical 44-byte WAV headers.

## 5. Offline Diarization & Speaker Separation (`suflyor-tts/src/diar.rs`)
- **Sherpa-ONNX Sidecar Execution**: Diarization runs via `suflyor-tts diarize <system.wav>` in a one-shot process invocation. This preserves ONNX Runtime isolation from the main host binary.
- **Model Pipeline**: Pyannote 3.0 segmentation ONNX + WeSpeaker ResNet34 embedding ONNX, followed by agglomerative clustering.
- **Alignment**: System utterances are mapped to dominant speakers via maximum temporal overlap, dropping phantom speakers (e.g. brief notification chimes).

## 6. Time-Stretching via WSOLA (`suflyor-wsola/`)
- **Pitch-Preserving Stretch**: Waveform Similarity Overlap-Add algorithm allowing playback speed adjustment (0.5x – 3.0x) without altering voice pitch.
- **Dual Correlation Engine**:
  - Direct search with 8-way unrolled loops for small window displacements.
  - FFT-accelerated cross-correlation using `rustfft` for large displacement ranges.
- **Parabolic Sub-Sample Interpolation**: Refines correlation peaks to fractional sample precision, eliminating phase smearing during speech expansion.
