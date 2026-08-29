# macOS local STT production audit

## Provenance

- STT fix commit: `3bff600c739ade39d235a07c2fc0cab3ff3ba20f`
- Final gated code candidate: `ffe84b6e54639f06511bb0706698b20fbf15af20`
- Final candidate tree: `32013fea44053fc4c8ce0254ae2046fd8be9ae32`
- Visual baseline: `cf53df74e3c1793f85a5faf2be54f36d9d8b9d05`
- Worker: `mac-worker`, physical Apple Silicon macOS host
- GigaAM model: 224,893,347 bytes, SHA-256 `2e3fcb7a7b66030336fd10c2fcfb033bd1dc7e1bf238fe5cfd83b1d0cfc9d28e`
- Known-answer fixture: 2.5525 s, 16 kHz mono Int16, SHA-256 `42346de8056447ffd2e09ceebdf2c0c1bf7f6c42bdbb6281813d58eb78f505c5`

The probe printed only a non-sensitive known-answer match. Screenshots use an isolated configuration and the generic model alias `/tmp/dsh-stt-ui-model`; they contain no credentials, private endpoints, user-home paths, or personal session data.

## Confirmed defect and minimal fix

Before the fix, validation/ad-hoc STT and the live pipeline could retain two GigaAM model instances. The candidate uses one process-global `Arc<Mutex<Model>>` for validation, live, and ad-hoc transcription. The model lock intentionally serializes inference. Session stop and relevant Settings changes release the cache entry; in-flight owners remain safe through their cloned `Arc`.

The macOS default and one-time schema-v1 migration now select CPU. Core ML remains an explicit opt-in because it is slower on this workload, uses more memory, and leaves accelerator-runtime residual allocations. Windows keeps DirectML as its default.

## Exact-SHA timing

Five production `transcribe_once` calls were run per accelerator on the candidate.

| Accelerator | Cold | Warm median | Warm RTF | Peak RSS | Known answer |
|---|---:|---:|---:|---:|---|
| CPU | 292.797 ms | 75.558 ms | ~0.0296 | 669.7 MiB | 5/5 |
| Core ML preference | 2869.280 ms | 128.095 ms | ~0.0502 | 1176.8 MiB | 5/5 |

The probe reports the configured accelerator preference, not an unobservable claim about every operator's actual provider placement.

## Lifecycle and resources

RSS values are process resident memory sampled after each bounded phase. The baseline already has a validated resident model.

| Accelerator | Before: baseline | Before: ad-hoc + live | Before extra | After: baseline | After: ad-hoc + live | After extra |
|---|---:|---:|---:|---:|---:|---:|
| CPU | ~670 MiB | ~1115 MiB | ~445 MiB | 670.4 MiB | 672.8 MiB | 2.4 MiB |
| Core ML | ~1175 MiB | ~2267 MiB | ~1092 MiB | 1173.5 MiB | 1362.4 MiB | 188.9 MiB |

The remaining Core ML increase occurs while reusing the same shared model and is attributable to accelerator/runtime workspace commitment rather than a second Rust model owner. Cache reset reduced RSS to 523.2 MiB on CPU and 917.8 MiB on Core ML.

Five production-style load/start/stop/reset cycles completed on both accelerators. From cycle 2 to cycle 5, RSS rose by 33.7 MiB on CPU and 12.5 MiB on Core ML; this bounded run cannot prove long-duration leak freedom. File descriptors and thread counts stayed flat after reset (CPU: 9 FDs/13 threads; Core ML: 19 FDs/16 threads), swap stayed at zero, and macOS reported no thermal or performance warning. Core ML still emitted repeated `Context leak detected, CoreAnalytics returned false`; this upstream/runtime residual is mitigated by the new CPU default, not claimed fixed.

## Visible Settings audit

Both revisions were built with `--features ui-mcp` and `SLINT_EMIT_DEBUG_INFO=1`. The reused Settings window was opened through MCP and the STT tab selected at 720×600 using the same theme and geometry.

- English: the before/after pixel diff contains 776 pixels, bounded entirely to the toggle at `(228,392)-(265,414)`.
- Russian: the before/after pixel diff contains 780 pixels, bounded entirely to the toggle at `(228,401)-(265,423)`.
- MCP element geometry independently shows the switch thumb moving from x=247 (on) to x=231 (off) in both locales.
- Both isolated candidate configurations persisted `config_version=2` and `stt_gigaam_gpu=false` after migration.
- Native Vision OCR found the intended English and Russian STT copy in all four captures. No clipping or unrelated visual drift was detected by the exact-pixel comparison.

Evidence:

| Locale | Before (Core ML on) | Candidate (Core ML off) |
|---|---|---|
| English | [before-en.png](before-en.png) | [after-en.png](after-en.png) |
| Russian | [before-ru.png](before-ru.png) | [after-ru.png](after-ru.png) |

## Gates and limits

- macOS full native gate: passed on exact candidate `ffe84b6e`; 701 backend tests, Slint/host tests, Swift MLX tests, and TTS sidecar tests completed; checkout remained clean.
- Windows full native gate: passed on exact candidate `ffe84b6e`; backend reported 703 passed/1 ignored, and all Slint, WSOLA, TTS, and TeraTTS layers passed; checkout remained clean.
- The first Windows attempt exposed a pre-existing gate defect on Windows 10 LTSC: backend tests started from `target/debug/deps` without the bundled DirectML runtime and exited with `STATUS_ORDINAL_NOT_FOUND`. Commit `ffe84b6e` stages the backend build's own SHA-matched `DirectML.dll` after Clippy and before tests. The final gate deliberately started with that destination absent, observed `stage DirectML for backend tests` pass, and then ran the full backend suite.
- Live microphone/system-audio speech capture was not accepted as production evidence: the unattended worker session did not have a pre-authorized TCC path, and the isolated UI run observed silence. Synthetic injection exercised the production STT pipeline without changing TCC settings.
- Simultaneous live MLX/TTS workloads were not run. Their build/test seams passed, and TTS remains process-isolated, but this audit does not claim a measured three-workload concurrency envelope.
- External Whisper remained out of scope because no installed model/server/endpoint was available and no large model download was authorized.
