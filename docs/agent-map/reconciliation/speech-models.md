# Speech recognition and speaker diarization: reconciled model inventory

Reviewed source: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. This is a bounded source/history review, not a native runtime acceptance. [Structured registry](speech-models.json) contains integration edges, source pins and outstanding checks. The coordinator corrected inaccurate pins, licenses and duration claims in the initial Gemini output.

## What was recently added

**Nemotron V3 is an optional Windows speaker-diarization engine, not the new STT text recognizer in this integration.** It reads recorded session audio and returns RTTM speaker intervals. It does not replace GigaAM or Whisper transcription.

- Config-resolved STT still has three variants: [`Cloud`, `Whisper`, `Gigaam`](../../../overlay-backend/src/config.rs#L1071-L1096).
- [`stt_backend`](../../../overlay-backend/src/config.rs#L863-L885) maps `stt_provider` to GigaAM in-process ONNX, local/server Whisper, or cloud Whisper.
- [STT Settings wiring](../../../slint-experiment/src/bin/overlay_host/settings_stt.rs#L32-L98) saves those providers and their endpoint/model settings.
- [Diarization engine selection](../../../overlay-backend/src/diarize.rs#L68-L140) separately chooses `Legacy` or `Nemotron3`.

## Model routes

| Role | Model/engine | Execution and boundary |
| --- | --- | --- |
| Transcription | GigaAM-v3 CTC int8 ONNX | In-process through `transcribe-rs`/`ort`; model path and GPU choice in STT config; native platform checks remain to verify |
| Transcription | Local Whisper `ggml-medium.bin` / configured server | `whisper-server` HTTP route; installed asset is medium, whereas legacy default server model text is `whisper-large-v3-turbo` |
| Transcription | Cloud Whisper | Groq HTTP API with configured model; availability depends on credentials/network |
| Speaker diarization | Legacy pyannote segmentation + WeSpeaker embedding | Isolated `suflyor-tts diarize` process; not the STT text path |
| Speaker diarization | Nemotron-3-Diarization V3 Q8 GGUF | Isolated Windows `nemo-speech.exe`; optional selection in transcript window, automatic speaker count, cancellation |

Do not infer model weight licenses from Rust or runtime-library licenses. Source pins are copied from [local-model constants](../../../overlay-backend/src/local_ai.rs#L90-L111) and [legacy diarization assets](../../../overlay-backend/src/diar_install.rs#L68-L85).

## Older transcription changes that were easy to lose

Git history also contains a real STT platform change: `a78978ec` enabled GigaAM through Core ML on macOS; `ad03df2b` migrated retired/unknown STT providers to GigaAM; `3bff600c` later bounded macOS GigaAM memory use. These are not Nemotron changes. [Accelerator selection](../../../overlay-backend/src/stt.rs#L36-L65) still chooses CoreMl when enabled and falls back to CPU on provider-load failure. [Defaults and migration](../../../overlay-backend/src/config.rs#L1532-L1555) now prefer CPU for affected older configs. The GPU toggle is not ignored on macOS.

The remembered “new audio model” could refer to this older GigaAM STT work rather than the recent Nemotron diarizer. Both histories are now recorded explicitly instead of guessing the intended one.

## Nemotron integration chain

1. [Optional transcript toggle and run controls](../../../slint-experiment/ui/transcript.slint#L464-L606) select the per-run engine; this is not a persistent `stt_provider` entry.
2. [Transcript controller](../../../slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs#L499-L770) installs the selected asset and calls cancellation-aware diarization, applying results only on success.
3. [Nemotron installer](../../../overlay-backend/src/diar_install.rs#L386-L483) pins `Nemotron-3-Diarization.q8_0.gguf`, revision, 107,270,112 bytes and SHA-256. The model is downloaded on demand, not bundled.
4. [Windows run path](../../../overlay-backend/src/diarize.rs#L246-L278) checks models, recorded WAV existence and `guard_wav_len` before spawning the runner. Non-Windows explicitly rejects this engine.
5. [Runner](../../../overlay-backend/src/nemotron_diar.rs#L15-L125) uses hidden process spawning, a shared lifetime JobObject, cancellation, bounded log lines, an elapsed-process cap and validated RTTM parsing.
6. [Build script](../../../scripts/build-slint-release.ps1#L75-L109) verifies vendored native binary hashes before staging. [NSIS](../../../scripts/slint-installer.nsi#L68-L83) packages executable/DLLs and notices.
7. [Vendor provenance](../../../vendor/nemotron-win-x64/README.md) documents upstream revision and a historical manual smoke. This is not a matching full application build/UI/release evidence bundle.

The WAV duration preflight is present at [the call site](../../../overlay-backend/src/diarize.rs#L259-L266) and implemented by [guard_wav_len](../../../overlay-backend/src/diarize.rs#L357-L373). The initial report's claim that this preflight was absent is rejected.

## Git and release status

The source history begins with `aa3c48c4` (`feat(diarization): add optional Nemotron V3 Windows engine`), followed by translation, integrity, cancellation and packaging changes through `2b1cd252`. The corrected full commit identities and actual Git timestamps are in the JSON registry.

The coordinator queried remote tags and the GitHub releases API during reconciliation:

- Latest observed prerelease: [`v0.38.1-rc.3`](https://github.com/PavelLizunov/suflyor/releases/tag/v0.38.1-rc.3), target `06792626`, predating Nemotron introduction.
- Latest observed stable: [`v0.38.0`](https://github.com/PavelLizunov/suflyor/releases/tag/v0.38.0).
- Remote `codex/nemotron-windows-rc` points to `2b1cd252`; remote master `8a38bacc` does not contain the Nemotron introduction.
- No observed published tag contains `aa3c48c4`. Therefore **source committed in task branches; published release inclusion not established and absent from observed latest RC**. An installer filename/version bump alone is not publication.

## Remaining checks

- Exact-candidate native Windows build, installer, cancellation/runtime smoke and transcript-window Slint screenshots/functional controls.
- Confirm intended new-model requirement: if a new *text recognizer* was intended, this Nemotron diarization integration does not supply it.
- Check each model's upstream weights license and runtime notices separately. Bundled [OpenMDW-1.1](../../../vendor/nemotron-win-x64/MODEL_LICENSE) has notice/attribution and warranty terms; the original claim of military/surveillance restrictions is not in that bundled text.
- Bound actual native memory/GPU behavior and long-recording runtime; no inference performance claim is accepted from source alone.
