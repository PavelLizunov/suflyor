# Source-linked feature contracts

These contracts map selected production chains at frozen research baseline `a10c356a`. They are manually inspected navigation/dataflow evidence, not compiler-derived call graphs, every-line/symbol coverage or native acceptance. Nine contracts currently index 277 source references; count is navigation scope, not completeness proof.

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

[Structured index](contracts.json) records IDs, baseline, source references and limits. [Original Grok register](../reconciliation/candidates.json) preserves counterevidence; [verification](../reconciliation/VERIFICATION.md) distinguishes executed local checks from unexecuted native behavior.

Still missing separate complete contracts: KB/search/archive/retranscription/summaries/coaching, updates/releases and build gates, full MLX/Hermes integration, all platform-native adapters, complete translation/assets/config-key schemas. Reliable language-aware symbol/line coverage and independent exact-SHA native acceptance remain open. Empty historical regex tables do not prove absence of routines.
