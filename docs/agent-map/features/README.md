# Source-linked feature contracts

These contracts map selected production chains at frozen research baseline `a10c356a`. They are manually inspected navigation/dataflow evidence, not compiler-derived call graphs, every-line/symbol coverage or native acceptance. Seventeen bounded contracts now cover principal runtime and platform integration chains; structured index records exact reference counts. Counts are navigation scope, not completeness proof.

| Feature | Contract | Key boundary |
| --- | --- | --- |
| Live transcription | [Config/UI/capture/VAD/backends/events/storage](live-transcription.md) | GigaAM/Whisper text, separate from speakers; semaphore is not global queued PCM bound |
| Speaker diarization | [Transcript controls/models/CLI/alignment/curated persistence](speaker-diarization.md) | Legacy vs optional Windows Nemotron; successful rerun can replace manual names |
| Session lifecycle/storage | [Start/stop/threading/health/mute/journal/catalog/debrief](session-lifecycle-and-storage.md) | Normal controls run on workers; curated DB data not losslessly reconstructible |
| AI and vision | [Config/provider/protocol/request/stream/image/tile routing](ai-and-vision-routing.md) | Codex stdio differs from HTTP permit path; OCR does not use vision endpoint |
| Managed local AI | [Ownership/install/switch/rollback/engine/deep-lock/MLX seam](managed-local-ai.md) | Reachability is not model identity; external listeners are not ours to kill |
| Read-aloud and OCR | [Selection/region/text/protocol/playback/voice/installer](read-aloud-and-ocr.md) | Piper/Tera isolated; Windows JobObject attaches; warm call exists; local OCR distinct from LLM |
| Personal memory | [Consent/capture/review/CRUD/retrieval/summary/provenance](personal-memory.md) | Normalizer helper has no reviewed production caller; approve legacy projection omits V2 fields |
| Settings/config | [Load/save/reset/import/export/preview/credentials](settings-and-portable-config.md) | Secret-bearing portable files, per-handler save/rollback and heterogeneous reset boundaries |
| Hotkeys/windows/capture | [13 keys/dispatch/realization/monitor/stealth/self-exclusion](hotkeys-windows-and-capture.md) | Registration not functional proof; intent/effective and internal/external capture differ |
| KB/reference | [Embedded Markdown/search/palette/grounding/snippet limits](kb-and-reference-search.md) | Empty palette query is empty; reference budget uses bytes, separate from archive FTS |
| Archive/playback/re-STT | [Saved catalog/audio/source/rename/delete/re-summary chain](archive-playback-and-retranscription.md) | Chunked re-STT returns two aggregate lines; delete not atomic; force paths differ |
| Summary/coaching | [Conspect/cache/map/reduce/retry/prep/debrief/style](summary-conspect-and-coaching.md) | Transcript-only cache, best-effort persistence; WPM live pill not established |
| Updates/build/release | [Stable updater/digest/gates/installer/cleanup acceptance](updates-build-and-release.md) | Initial URL check not final redirect policy; source RC version is not publication |
| MLX | [Pinned snapshots/owned child/Swift protocol/gate/runtime](mlx-sidecar-and-model-install.md) | Start uses fast snapshot, one active model does not bound waiters |
| Hermes | [Bridge/tool client/install/config/profile-prep API](hermes-bridge-and-plugin.md) | Configured nonloopback allowed; summary stores differ; save failure still can acknowledge |
| Startup/wizard/health/diagnostics | [Preflight/first-run/checks/ticker/report/logs](startup-wizard-health-and-diagnostics.md) | Config absence, not completion flag; config readiness ≠ live proof; UI detail and exported redaction differ |
| Config/UI/translation/assets | [Schema/default mechanisms/import graph/catalog/resources](config-ui-translation-and-assets.md) | Unevaluated defaults; duplicate catalog entries retained; static resource existence not native embedding |

[Structured index](contracts.json) records IDs, baseline, source references and limits. [Original Grok register](../reconciliation/candidates.json) preserves counterevidence; [verification](../reconciliation/VERIFICATION.md) distinguishes executed local checks from unexecuted native behavior.

Still missing complete per-module/all-caller coverage: all platform-native adapters, translation/assets/config-key schemas, remaining startup timers/health error callers, every process/model edge and experiments/vendor boundaries. Reliable language-aware symbol/line coverage and independent exact-SHA native acceptance remain open. Empty historical regex tables do not prove absence of routines.
