# Grok reconciliation: completed source triage, not runtime acceptance

Reviewed baseline: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. The original report bytes are unchanged; redacted copies and exact claims are preserved.

## What completed

- All 119 original candidate texts were inspected against source and relevant counterevidence.
- Status counts: `confirmed` 40, `hypothesis` 74, `rejected` 5.
- Confirmed is a source mechanism/gap, not proof of native exploit or user-visible failure. All 74 hypotheses retain explicit reproduction/precondition limits.
- Five claims rejected; actual FTS deletion/FK schema locally checked with Python SQLite 3.45.1.
- Nine checkpoint/recovery mechanics tests pass; no standalone automatic DSH watchdog implemented.
- Four first Opus lanes returned null with partial topic substitutions; eight follow-up lanes returned null with no files. Not labeled successful independent review.
- One short Gemini model inventory was corrected by coordinator: Nemotron diarization vs Whisper/GigaAM transcription, pins, CoreML history, licenses, duration guard and release status.

## Highest-priority source mechanisms

- Catalog deletion is not lossless: memory/diarization are not derived from JSONL.
- STT permits acquired after spawning PCM-owning tasks do not bound queued memory.
- Journal channel is unbounded and flush acknowledgement is not fsync durability.
- Auxiliary window reveal is not fail-closed when WDA application fails.
- URL masking and screenshot-visible diagnostics can retain sensitive endpoint/query/path information.
- Native knowledge/NSI/script-only classifier lacks affected-check mapping.

## Counterevidence worth preserving

- FTS5 DELETE by stored UNINDEXED session_id works; no session FK cascades exist into curated tables.
- Clipboard conversation formatter filters system-role messages and images.
- Default catalog sweep invokes model backfill; catalog history retention after journal pruning is deliberate.
- Nemotron validates WAV duration and bounds/cancels its process; legacy path differs.
- Local model profile/vision availability are populated through refresh_local_context_controls; initial missing-setter claim was incomplete.
- macOS GigaAM selects CoreMl when enabled with CPU fallback; migration defaults to CPU after memory concern.

## What did not complete

- Native Windows/macOS tests, builds, exploit/fault/latency reproductions, live UI acceptance and Nemotron RC publication.
- Entire-project line/symbol semantic coverage, all-language accurate extraction and every feature contract.
- Independent reviewer acceptance or host-crash automatic session restart.

Read [remediation queue](REMEDIATION.md), [speech history](speech-models.md), [exact register](candidates.json) and [dispatch receipts](dispatch-receipt.json).

## Original candidate disposition

| ID | Status | Severity | Evidence |
| --- | --- | --- | --- |
| `wave1_worker1_persistence-C01` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C03` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C04` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C05` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C06` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C07` | rejected | low | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C08` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C09` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C10` | confirmed | high | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C11` | rejected | low | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C12` | confirmed | low | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C13` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C14` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C15` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C16` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C17` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker1_persistence-C18` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker2_memory-C01` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker2_memory-C02` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker2_memory-C03` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker2_memory-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker2_memory-C05` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker2_memory-C06` | confirmed | low | coordinator source inspection; native not run |
| `wave1_worker2_memory-C07` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker2_memory-C08` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker3_config-C01` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker3_config-C02` | confirmed | high | coordinator source inspection; native not run |
| `wave1_worker3_config-C03` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker3_config-C04` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker3_config-C05` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker3_config-C06` | hypothesis | low | coordinator source inspection; native not run |
| `wave1_worker3_config-C07` | hypothesis | medium | coordinator source inspection; native not run |
| `wave1_worker3_config-C08` | confirmed | medium | coordinator source inspection; native not run |
| `wave1_worker3_config-C09` | confirmed | low | coordinator source inspection; native not run |
| `wave1_worker3_config-C10` | rejected | low | coordinator source inspection; native not run |
| `wave2_worker1_audio-C01` | confirmed | low | coordinator source inspection; native not run |
| `wave2_worker1_audio-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker1_audio-C03` | confirmed | medium | coordinator source inspection; native not run |
| `wave2_worker1_audio-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker1_audio-C05` | hypothesis | low | coordinator source inspection; native not run |
| `wave2_worker1_audio-C06` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker1_audio-C07` | confirmed | low | coordinator source inspection; native not run |
| `wave2_worker1_audio-C08` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker1_audio-C09` | confirmed | low | coordinator source inspection; native not run |
| `wave2_worker2_stt-C01` | confirmed | high | coordinator source inspection; native not run |
| `wave2_worker2_stt-C02` | confirmed | medium | coordinator source inspection; native not run |
| `wave2_worker2_stt-C03` | hypothesis | high | coordinator source inspection; native not run |
| `wave2_worker2_stt-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker3_tts-C01` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker3_tts-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker3_tts-C03` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker3_tts-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C01` | confirmed | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C03` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C04` | hypothesis | low | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C05` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C06` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C07` | hypothesis | low | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C08` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C09` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C10` | hypothesis | medium | coordinator source inspection; native not run |
| `wave2_worker4_local_ai-C11` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C01` | rejected | low | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C03` | hypothesis | high | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C05` | hypothesis | low | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C06` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C07` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker1_bridge-C08` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C01` | confirmed | high | coordinator source inspection; native not run |
| `wave3_worker2_window-C02` | hypothesis | high | coordinator source inspection; native not run |
| `wave3_worker2_window-C03` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C05` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C06` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C07` | hypothesis | low | coordinator source inspection; native not run |
| `wave3_worker2_window-C08` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C09` | hypothesis | low | coordinator source inspection; native not run |
| `wave3_worker2_window-C10` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker2_window-C11` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker3_tile-C01` | hypothesis | high | coordinator source inspection; native not run |
| `wave3_worker3_tile-C02` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker3_tile-C03` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker3_tile-C04` | rejected | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C01` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C03` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C04` | hypothesis | low | coordinator source inspection; native not run |
| `wave3_worker4_settings-C05` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C06` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C07` | hypothesis | low | coordinator source inspection; native not run |
| `wave3_worker4_settings-C08` | confirmed | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C09` | hypothesis | medium | coordinator source inspection; native not run |
| `wave3_worker4_settings-C10` | confirmed | low | coordinator source inspection; native not run |
| `wave4_worker1_privacy-C01` | confirmed | low | coordinator source inspection; native not run |
| `wave4_worker1_privacy-C02` | confirmed | medium | coordinator source inspection; native not run |
| `wave4_worker1_privacy-C03` | confirmed | medium | coordinator source inspection; native not run |
| `wave4_worker1_privacy-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker1_privacy-C05` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker2_installers-C01` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker2_installers-C02` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker2_installers-C03` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker2_installers-C04` | hypothesis | low | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C01` | confirmed | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C02` | confirmed | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C03` | confirmed | low | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C04` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C05` | hypothesis | low | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C06` | hypothesis | low | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C07` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C08` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C09` | confirmed | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C10` | hypothesis | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C11` | confirmed | medium | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C12` | hypothesis | low | coordinator source inspection; native not run |
| `wave4_worker3_cicd-C13` | confirmed | medium | coordinator source inspection; native not run |
