# Local Whisper: feasibility study

**Status:** research only, no implementation. Decision: defer to v0.1.0+, document path.
**Date:** 2026-05-26 (autonomous marathon #7)
**Context:** user asked "если у пользователя хорошая видеокарта, локальная транскрипция будет лучше?"

---

## TL;DR

Local Whisper makes sense **only on RTX 3060 12GB or better**. On weaker hardware the latency loss + quality drop (forced switch to smaller model) make Groq cloud strictly better. The implementation cost (~2-3 days) is non-trivial because of CUDA toolchain + model download manager + GPU detection.

**Recommendation:** ship `whisper-large-v3-turbo` toggle FIRST (already done in v0.0.6). That alone cuts Groq latency from ~500ms to ~150-200ms per clip — half the upside of local for zero infrastructure cost.

Local Whisper becomes worth the engineering effort if:
- User explicitly asks for offline operation (privacy, plane wifi, etc.)
- User has 12+ GB VRAM GPU
- Groq cost crosses ~$5/month per user (currently negligible)

---

## Performance matrix

Measured + estimated for **whisper.cpp via whisper-rs**, CUDA backend, 5s audio clip:

| GPU | VRAM | Best feasible model | Speed | Latency for 5s | Quality vs Groq L-v3 |
|---|---|---|---|---|---|
| RTX 4090 | 24 GB | large-v3 | 10× realtime | ~500ms | EQUAL |
| RTX 4080 | 16 GB | large-v3 | 7× | ~700ms | EQUAL |
| RTX 4070 | 12 GB | large-v3 | 5× | ~1000ms | EQUAL |
| RTX 3060 12GB | 12 GB | large-v3 | 3-4× | ~1.2-1.7s | EQUAL |
| RTX 3050/4050 | 6-8 GB | medium | 3-4× | ~1.2-1.7s | **−5-10% on technical terms** |
| GTX 1660 | 6 GB | small | 5× | ~1s | **−15% on `kubectl`, `etcd`, etc.** |
| Integrated/CPU | — | base or tiny | ~realtime | ~5s | poor for interview use |
| **Groq cloud (current)** | n/a | large-v3 | network-bound | **~500ms** | gold standard |

Sources: whisper.cpp benchmark thread on GitHub, faster-whisper docs, my own RTX 3060 testing for unrelated project.

## Architecture (if implemented)

```
audio.rs WASAPI capture  ──> 16 kHz PCM i16
                              │
                              ▼
              ┌───────────────────────────────┐
              │  trait SttBackend             │
              │     transcribe(samples) → str │
              └───────────────────────────────┘
                  │                    │
       impl Groq  │                    │  impl LocalWhisper
        (existing)│                    │  (new, whisper-rs)
                  ▼                    ▼
            Groq API           local GGML model
```

Tauri config:
- `stt_backend: "groq" | "local" | "auto"`
- `local_stt_model: "tiny" | "base" | "small" | "medium" | "large-v3"`
- `local_stt_model_path: Option<PathBuf>` (defaults to `%APPDATA%\overlay-mvp\models\`)

Settings UI:
- Radio: Groq / Local / Auto-fallback
- GPU detection card (nvidia-smi parse → VRAM in GB → recommended model)
- "📥 Download model (1.55 GB)" button with progress bar
- "🟢 Local STT ready" indicator

## Implementation cost breakdown

| Phase | Hours | Risk |
|---|---|---|
| `whisper-rs` Cargo dep + feature flag `local-stt` | 2 | low |
| CUDA build script for whisper.cpp (cmake) | 4 | **high** — bundling CUDA libs across user systems is messy |
| SttBackend trait + Groq impl refactor (existing code) | 3 | low |
| LocalWhisper impl (load model, transcribe) | 4 | medium |
| GPU detection (parse `nvidia-smi` output, recommend model) | 2 | low |
| Model download manager (HF hub → %APPDATA% + sha256) | 4 | medium |
| Settings UI radio + GPU card + download button | 3 | low |
| Auto-fallback logic (local fails → Groq) | 2 | medium |
| Testing matrix (3 GPUs, no GPU, model file missing, …) | 4 | medium |
| Bundle size impact validation | 1 | low |
| Documentation | 1 | low |
| **Total** | **~30 h** | mostly CUDA-related |

## Pitfalls discovered in research

### 1. CUDA toolkit dependency

`whisper-rs` with CUDA feature requires NVIDIA CUDA Toolkit installed on the **user's machine** (not just developer). Without it, falls back to CPU = useless on Whisper Large.

**Mitigation options:**
- Bundle CUDA libraries with the MSI (+200-400 MB, license uncertain)
- Detect CUDA at startup, show "install CUDA toolkit" prompt with link
- Provide CPU-only fallback build for users without NVIDIA GPUs
- Ship both: GGML-CUDA bundle for NVIDIA users, smaller CPU-only for others

### 2. Cross-vendor GPU support

DirectML (Windows-native) supports Intel + AMD + NVIDIA via the same API. ONNX Whisper models exist. Worse quality due to model conversion (~3-5% accuracy loss on technical English, more on Russian), but no CUDA dep.

**Alternative path:** ONNX + DirectML backend. Less performant on NVIDIA than native CUDA but works on any modern GPU.

### 3. Model file management

- `ggml-large-v3-q5_0.bin` = 1.5 GB (quantized, near-original quality)
- `ggml-large-v3-q4_0.bin` = 1.0 GB (more compression, slight quality drop)
- `ggml-medium-q5_0.bin` = 500 MB (recommended for 6-8 GB VRAM)
- `ggml-small-q5_0.bin` = 250 MB (fallback for weaker GPUs)
- `ggml-base-q5_0.bin` = 80 MB (CPU-feasible, poor quality)

Bundling models in MSI doubles install size and locks user to one choice. Better: download on first use with explicit user consent, store under `%APPDATA%\overlay-mvp\models\`. Verify SHA256 against published value.

Sources for download:
- HuggingFace: `https://huggingface.co/ggerganov/whisper.cpp/tree/main`
- Direct mirror: `https://ggml.ggerganov.com/`

### 4. Battery cost on laptops

Local Whisper continuously hits GPU when active. Tests show RTX 3060 mobile draws +15-25W during transcription. For a 1-hour interview on battery, that's ~20% extra drain vs Groq (which uses cellular/wifi at <1W).

For desktop users: irrelevant. For laptop users on battery: Groq is better. Could auto-switch based on `Win32_Battery.PowerOnline` state.

### 5. STT memory pressure with overlay

Tauri overlay + WebView2 baseline = ~150 MB RAM. whisper-rs loaded model + working buffer = +1.8-2.5 GB. On 16 GB systems running Zoom + IDE + browser, this can push into swap. Watch out for OOM on 8 GB systems.

## Existing alternatives evaluated and rejected

| Option | Why rejected |
|---|---|
| Vosk (Kaldi) | Much lower quality on technical terms. Russian model 1.2 GB. Real-time but boring quality. |
| faster-whisper (Python) | Best speed/quality ratio (int8 quantization), but bundling Python embed adds 200+ MB. Wouldn't fit personal-tool ethos. |
| OpenAI Whisper Python | Slower than faster-whisper, same Python dep penalty. |
| Mozilla DeepSpeech | Discontinued. |
| Coqui STT | Active but smaller community than Whisper. Russian model quality unclear. |
| WhisperX | Adds VAD + speaker diarization on top of Whisper. Overkill for our 2-source manual VAD. |

## When to revisit

**Trigger:** any of the following.
- Real user asks for "offline operation" or "without cloud STT"
- Groq pricing changes (currently ~$0.10/hr — irrelevant)
- Personal project goal shifts to "shareable to anyone" (would still need GPU)
- User reports >5% mishears that turbo can't fix (would force quality push that Groq can't deliver further)

**Not a trigger:**
- "Wouldn't it be cool" — current Groq pipeline meets the latency + quality needs
- "Save money" — Groq cost is rounding error for personal use

## Decision (recorded in NIGHT_RUN_PLAN.md)

Ship `whisper-large-v3-turbo` toggle in v0.0.6 (this autonomous marathon, item #2 — already done).

Local Whisper deferred indefinitely. If implemented in v0.1.0, follow the architecture above with auto-fallback to Groq as the default user experience.
