# Source-linked feature contracts

These contracts map selected production chains at the frozen research baseline. They are manually inspected navigation/dataflow evidence, not compiler-derived call graphs, exhaustive symbol coverage or native acceptance.

| Feature | Contract | Key boundary |
| --- | --- | --- |
| Live transcription | [Config/UI/capture/VAD/backends/events/storage](live-transcription.md) | GigaAM and Whisper recognize text; queued PCM/task count is not bounded by the inference semaphore |
| Speaker diarization | [Transcript controls/models/CLI/alignment/curated persistence](speaker-diarization.md) | Legacy and optional Nemotron label speakers; not new STT text model |
| Session lifecycle and storage | [Start/stop threading/health/mute/journal/catalog/debrief](session-lifecycle-and-storage.md) | Normal controls dispatch helpers on runtime workers; curated DB data is not JSONL-rebuildable |

[Structured contract index](contracts.json) records feature IDs, baseline, evidence limits and source references. [Reconciliation register](../reconciliation/candidates.json) holds per-claim counterevidence; [verification log](../reconciliation/VERIFICATION.md) states executed checks.

Missing feature contracts remain explicit work: AI/vision/local model routing, TTS/OCR, personal memory, hotkeys/window capture/privacy, settings/import/export, updates/releases, MLX/Hermes, installer and platform seams. Empty old tables must not be used as evidence those features have no routines.
