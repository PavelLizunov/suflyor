# Read-aloud (TTS) — implemented state (2026-06-18)

**Status: working end-to-end, user-tested live, UNCOMMITTED (changes on disk only), NOT released.**
Resume from "Remaining" below; then 5-layer gate → show user → ship only on explicit «релизь».

Full design plans: workflow outputs `w3jr5zfhl.output` (engine), `wrlwvcewk.output`
(shortcuts + Tesseract), `a0f44e34b6e53ae97.output` (TTS-landscape research) under the
session transcript dir. Memory: `[[read-aloud-tts-sidecar]]`.

## Architecture (load-bearing)

Neural TTS (sherpa-onnx) **cannot** share a process with the app's `ort`/GigaAM STT —
two static onnxruntimes crash natively (`0xc0000005`) on the 2nd model load
(repro: `overlay-backend/examples/stt_double.rs`). So TTS runs in a **separate
`suflyor-tts.exe` sidecar** (crate `suflyor-tts/`, links sherpa **static**, NO ort,
self-contained ~17 MB). It synthesizes + plays audio and takes one command per stdin line:
`VOICE <dir>` / `RATE <-10..10>` / `SPEAK <base64-utf8>` / `PAUSE` / `RESUME` / `STOP`
(stdin-EOF → stop + exit).

- `suflyor-tts/src/main.rs` — stdin reader + interleaved synth/feed worker.
- `suflyor-tts/src/engine.rs` — sherpa `NeuralEngine` + voice scan/load/pick + text chunk/sanitize + rate→speed.
- `suflyor-tts/src/playback.rs` — WASAPI **stereo** render (declares 2ch + duplicates L+R).
- `overlay-backend/src/tts.rs` — thin **client**: spawns the sibling exe lazily, `warm()`
  preloads Irina at startup, respawns on broken pipe, base64-frames text, scans installed
  voices for the chooser. Public API unchanged. (Removed sherpa dep + `tts_neural.rs`/`playback.rs`
  from overlay-backend; added `base64`.)

No root workspace → the release script must build BOTH crates into `target/release/` (sidecar
beside `overlay-host.exe`) and the NSIS installer must ship `suflyor-tts.exe`. Dev build reuse:
`CARGO_TARGET_DIR=slint-experiment/target` lets the sidecar reuse the cached sherpa static lib.

## Voices

`%APPDATA%\suflyor\tts\<voice-dir>\` (`*.onnx` + `tokens.txt`; Piper bundles `espeak-ng-data`
inside the voice dir). Ship voice = **Piper `vits-piper-ru_RU-irina-medium`** ("Ирина",
22050 Hz, handles ё + punctuation). `vits-mms-rus` = dev only (drops ё/punct). First-audio
latency warmed ≈ 1.4 s.

## Shortcuts (one-handed) — `hotkeys.rs` + `overlay_host.rs` dispatch + `win32.rs` helpers

- **Shift+Alt+1 — read SELECTED text (clipboard):** clear clipboard → `win32::send_ctrl_c()`
  (which **releases the held Shift+Alt first**, else the synthetic Ctrl+C is read as
  Shift+Alt+Ctrl+C and copies nothing) → 140 ms timer → read clipboard → restore the user's
  clipboard → `spawn_text_tile()` shows a tile with the text + 🔊/⏯/📋/✕ and auto-reads.
  **Zero OCR artifacts** (it's the real selected characters).
- **Shift+Alt+2 — OCR a region + read:** reuses `fire_f8_vision_capture(VisionMode::Ocr)`,
  now backed by **local Tesseract** (see OCR section below), NOT the VLM. Windows occasionally
  eats `Shift+Alt+2` (layout-switch chord); fallback combo `Ctrl+Alt+2`. Also on `Ctrl+F8`.
- **Shift+Alt+3 — pause / resume** the read-aloud. Shares one process-global latch
  (`SPEAK_PAUSED` in `tile_copy.rs`) with the tile's ⏯ button, so the key and the button
  stay coherent and the hotkey flips the speaking tile's ⏯ icon. A fresh read (🔊 / new
  tile) resets the latch (`reset_pause()`).

## Other fixes (this session, live-tested)

- 🔊 reads **only the latest assistant answer** (was reading the whole thread incl. prompts).
- Closing the speaking tile **or** the app **stops** audio.
- Mono→**stereo** (was one-ear); voice **warm** at startup.
- **Anti-feedback:** `tts::is_speaking()` (estimated from text length) gates the SYSTEM-audio
  STT submit in `stt.rs` so the reader's voice isn't transcribed and answered (the
  "суфлёр отвечает на читалку" loop — confirmed gone).

## Remaining (resume here)

1. ✅ **DONE (code) — Telegram copy on Shift+Alt+1.** `win32::send_ctrl_c` now sends real
   hardware SCAN CODES (via `MapVirtualKeyW`, + extended-key flag on right-Alt) instead of
   `wScan: 0` pure virtual keys — Qt apps like Telegram Desktop read the scan code and ignored
   the bare-vk synthetic Ctrl+C. Pending live test in Telegram.
2. ✅ **DONE (code) — Tesseract OCR** replaces the looping VLM on the OCR path. See OCR section.
   **Remaining for release:** delivery (download-on-first-use, see size note) + installer/git-gate
   + adversarial review + live test.
3. ✅ **DONE — Shift+Alt+3 pause/resume** (shared `SPEAK_PAUSED` latch; build + live test pending).
4. **RU text normalization** (numbers→words with case agreement, dates) — no off-the-shelf
   offline+commercial+Rust TTS does it; realistic path = keep Piper + a ~400-600-line Rust
   normalizer. ~1–1.5 weeks. Research: `a0f44e34b6e53ae97.output`.
5. ✅ **DONE (code) — Settings «Озвучка» panel.** New nav tab (idx 5, `speaker.svg`) +
   `settings_voice.rs` (`wire_voice_settings` + preset helpers, 2 tests): voice ComboBox (from
   `tts::voices()`), speed-preset ComboBox (0.75×/1.0×/1.3×/1.5×/2.0× → rate −5/0/3/5/10), 🔊 Test
   button; saves `tts_voice`/`tts_rate` + applies live via the `tts` client. 6 new @tr + ru.po.
   **Still for release:** **installer** ships `suflyor-tts.exe` + SHA-pinned first-run voice
   download; add `suflyor-tts` to the git-gate.
6. Pre-existing **debug-build** crash at `runtime.rs:1437` (Tokio "no reactor" on the auto-start
   AI-error path) — RELEASE is fine; smoke/test via release builds.

## OCR — Tesseract (local, deterministic) — replaces the VLM on the read-aloud path

The 4B VLM looped/hallucinated on dense text. OCR now runs the bundled **Tesseract 5.4.0**
(UB-Mannheim, Apache-2.0) as a **child process** — NOT FFI, NOT onnxruntime/RapidOCR (a 2nd
onnxruntime would crash against `ort`/GigaAM, the same reason TTS is a sidecar).

- `overlay-backend/src/ocr.rs` — `run_ocr(bgra,w,h,lang)`: hand-rolled 24-bit top-down **BMP**
  encoder (lossless, no image-codec dep) → spawns `tesseract stdin stdout -l rus+eng --oem 1
  --psm 6` (`CREATE_NO_WINDOW`, `TESSDATA_PREFIX` set), feeds the BMP on **stdin** + reads text
  on **stdout** (screen pixels never touch disk — privacy). `tesseract_root()` resolves
  `<exe_dir>\tesseract\` then `%APPDATA%\suflyor\tesseract\`. `is_available()` drives fallback.
  5 unit tests (BMP header/top-down, normalize, short-buffer reject).
- `vision_capture.rs` — `launch_vision_for_bgra` OCR branch: if `ocr::is_available()`, run
  Tesseract off-thread (`spawn_blocking`) and `fill_ocr_tile` on the UI thread; **else fall
  through to the VLM** (tester without the pack still gets a result). `fire_f8_vision_capture`
  now lets OCR run even with **Vision "off"** (local engine needs no endpoint; placeholder ep).
- `overlay_host.rs` — `fill_ocr_tile(weak,convo_id,bridge,text)`: fills the "⏳ Распознаю…"
  placeholder tile, hides 🔄, seeds the conversation, auto-reads (mirrors `spawn_text_tile`).

**Validated:** standalone OCR of Russian (ё + digits + punctuation + mixed Latin) is near-perfect;
the exact in-app invocation (BMP via stdin) reproduces it. Isolated Latin acronyms can homoglyph
to Cyrillic ("OCR"→"ОСК") — cosmetic. Live Slint region-select → read still to be user-tested.

**Adversarial review (3-lens workflow) — done, fixes applied:** (1) trust-boundary doc added —
the user-writable engine dir means the download-on-first-use step MUST SHA-verify before the
first spawn (verify-before-execute, like update.rs); recorded as a hard release requirement.
(2) `ep` is now `Option` — the placeholder-endpoint hack is gone; an OCR request whose engine
vanished mid-drag (TOCTOU) shows a generic "OCR недоступен" tile instead of a confusing VLM
"AI error". (3) empty OCR now hides the no-op 🔊/📋. Gate: clippy+fmt clean both crates,
321 backend tests (incl. 5 ocr), release built.

### ⚠️ Size / delivery finding (re-decide before release)
The UB-Mannheim runtime is **~158 MB of DLLs** (`libtesseract-5.dll` alone is **97 MB** — a
debug-enabled mingw build; ICU data +29 MB), NOT the ~25–30 MB estimated. **Bundling in the
installer is impractical.** Plan = **download-on-first-use**, SHA-pinned, into
`%APPDATA%\suflyor\tesseract\` (mirrors the llama.cpp/model pattern). Pinned sources:
- tesseract installer (extract, don't run): GitHub UB-Mannheim `v5.4.0.20240606`,
  SHA256 `c885fff6998e0608ba4bb8ab51436e1c6775c2bafc2559a19b423e18678b60c9` (winget-verified).
- `tessdata_fast` `rus` SHA256 `e16e5e036cce1d9ec2b00063cf8b54472625b9e14d893a169e2b0dedeb4df225`,
  `eng` `7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2` (tag 4.1.0).
- Could `llvm-strip` the 97 MB DLL (debug info) to shrink the download a lot — release-prep TODO.
- Dev box: full runtime hand-placed at `%APPDATA%\suflyor\tesseract\` (so the app finds it now).

## Gate status

clippy clean (both crates), unit tests pass (playback / tts_neural / tts + tile_copy speak
tests), release builds + boots, STT GigaAM unaffected. Not yet: adversarial review, i18n_guard
for any new `@tr`, NSIS/installer wiring, version bump.
