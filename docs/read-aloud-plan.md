# Read-aloud feature (F8 → OCR → TTS) — plan + research

Goal: select on-screen text (region capture), transcribe it **verbatim**, and
**read it aloud** with a chooser of voices + transport (play / pause / speed).
The user's pain: "большой текст не хочется читать — выделил и озвучил".

Untracked working doc. Spans several releases; each stage ships + is tested.

## Decisions (locked)
- **Controls UI:** inline transport on the result tile (chip + play/pause/speed),
  NOT a floating mini-player (every new window = another stealth/monitor surface),
  NOT bar chips. (Research-backed; lowest blast radius.)
- **Verbatim** is the core; "summarize → read" is a later optional toggle (Stage D).
- **Multiple light voices + a chooser** (user ask): OneCore (built-in) + downloadable
  light ONNX neural voices; pick in Settings → "Озвучка".
- **Local-first**; cloud TTS opt-in later with an egress warning (like Vision).
- **Voice cloning** (clone-from-a-sample) = deferred to Stage D (needs CosyVoice2 /
  PyTorch / GPU). Revisit only if the user wants it.

## Engine research conclusion (decision-grade, 2 workflows)
The make-or-break for Russian is **stress (ударение)**. Engines split:
- **Native RU stress** (sound right): Silero, **Vosk-TTS**, F5-RU.
- **espeak-ng phonemizer** (wrong stress): Piper out-of-the-box, XTTS.
  Fix = pre-stage **RUAccent → RUPhon** (both ONNX, Apache-2.0, run on the app's
  `ort`); RUPhon for IPA also drops espeak's GPL dependency.

Picks under our constraints (offline · `ort`/ONNX · CPU so it doesn't fight
llama.cpp for the GPU · commercial license · configurable voices):
- **Vosk-TTS** (Natasha/multi) — native RU stress, ONNX/CPU, **Apache-2.0**,
  best clarity (CER ~0.7). Simplest "just works" neural RU. → neural default.
- **Piper** (Irina best vocoder) — ONNX/CPU/MIT, but needs RUAccent for stress.
- **OneCore "Ирина"** (Windows SAPI/WinRT) — 0 MB, native pause/resume, correct
  stress, dated voice. → MVP + zero-download fallback.
- **CosyVoice2** (Apache, GPU, PyTorch) — only one that **clones a voice from a
  short clip**. → optional Stage D.
- Rejected: Silero (CC-BY-NC + no ONNX TTS), XTTS (non-commercial + weak RU),
  Kokoro/MeloTTS (no Russian), Fish/F5-base (non-commercial).

Integration: `sherpa-onnx` (Rust crate, loads Vosk-TTS/Piper/MMS/Matcha) or
`piper-rs` (already on `ort 2.0-rc.12`). Speed without pitch change =
`signalsmith-stretch` at the output. Playback reuses the existing WASAPI render
loop (`overlay-backend/src/audio.rs::play_test_clip`, `wasapi 0.23`) — NO rodio/cpal.

## Architecture
```
Ctrl+F8 region → freeze → drag-region → hi-fi crop → vision OCR (verbatim) → text tile
                                                                                  │
                                              🔊 transport (play/pause/speed) ────┤
                                                                                  ▼
   TtsController → Segmenter(UAX#29 + RU abbrev) → Synth worker (ahead, cancellable)
                                                    │  bounded queue (N≈2)
                                                    ▼
              WASAPI render-loop ← signalsmith-stretch(speed) ← engine PCM
              (pause-gate + generation-cancel)
```
Engine behind a `TtsEngine` trait (`synth(text, speed) -> PCM`): `Sapi` (OneCore),
`Onnx` (Vosk/Piper via sherpa/piper-rs), opt. `Cloud`. Voices = a catalog +
download-on-demand (like the local-AI installer, SHA-verified); chooser in Settings.

## Stages (each = its own release, gated + tested)
- **A — MVP.** `VisionMode::Ocr` (verbatim text) + inline transport + chooser with
  **OneCore voices** (instant) + play/pause/speed.
  - **A.1 (DONE, in tree):** `VisionMode::Ocr` + strict verbatim profile-free prompt
    (`overlay-backend/src/vision.rs`), dispatch arms (`vision_capture.rs`),
    **Ctrl+F8** trigger (`hotkeys.rs` + `overlay_host.rs`). Test: Ctrl+F8 → select a
    text region → exact text in a "🔊 Текст с экрана" tile. (Capture-fidelity bump
    for small text is a follow-up if accuracy is short.)
  - **A.2:** `overlay-backend/src/tts.rs` (`TtsEngine` trait + `SapiEngine` via
    ISpVoice/WinRT) + config `tts_*` + Settings "Озвучка" + tile Speak chip + transport.
- **B — neural catalog.** Vosk Natasha (native stress) via sherpa-onnx/piper-rs +
  `playback.rs` (WASAPI render-loop + PCM queue + generation-cancel) + segmentation +
  download-on-demand voice catalog. Pause-gate.
- **C — quality.** Piper Irina/Ruslan (+RUAccent→RUPhon) in the chooser; speed via
  `signalsmith-stretch` (0.75×–2×, pitch-stable); sentence seek + progress.
- **D — optional.** Cloud TTS (Azure/OpenAI) opt-in; voice cloning (CosyVoice2);
  "summarize → read"; synth cache; long-text cap.

## Key code anchors
- F8 pipeline: `overlay_host.rs` F8 dispatch (~2005), `vision_capture.rs`
  (region-select → crop → `launch_vision_for_bgra`); OCR text = `StreamState.accumulated`
  (`tile_controller.rs:553`).
- Audio render template: `overlay-backend/src/audio.rs::play_test_clip` (~527).
- UI patterns: tile chrome buttons (`tile.slint`), `wire_copy` (`tile_copy.rs`),
  Settings clone target `settings_vision.rs` → `settings_voice.rs`, i18n guard
  (`tests/i18n_guard.rs`).
