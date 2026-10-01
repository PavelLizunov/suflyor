# Grok candidate reconciliation: source-inspection checkpoint

Reviewed source: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Existing Grok-labeled reports are candidate evidence; no new Grok/Astra calls.

## Current measured state

- Original candidates: 119.
- Coordinator inspected exact original claims: 55.
- Status counts: `confirmed` 21, `hypothesis` 30, `rejected` 4, `unresolved` 64.
- Confirmed means source mechanism established, not native consequence reproduced. Hypotheses remain open.
- The initial four Opus outputs are unaccepted partial proposals; eight follow-up Opus results were null and produced no exact-claim artifacts. Both outcomes are preserved in dispatch receipts.

## Priorities already established by source

- Catalog deletion is not lossless: curated memory/diarization are not reconstructible from JSONL.
- STT semaphore is acquired after task/PCM enqueue; queued task count is not bounded by the permit count.
- Journal channel is unbounded and flush acknowledgement is not fsync durability.
- Auxiliary window reveal is not fail-closed after affinity application failure.
- mask_host is not a full URL/secret redactor; diagnostics panel details can display raw endpoint/path values.
- Native gate misses compiled knowledge Markdown and cross-cutting script-only trigger coverage.
- Nemotron V3 is an optional Windows diarizer in source branches, not the published RC STT replacement.

## Verified counterevidence

- Actual FTS5 DELETE by stored UNINDEXED session_id works in Python SQLite using real migrations.
- Curated memory and diarization tables have no session FK cascade in shipped migrations.
- Conversation clipboard formatting filters system-role messages and images; system prompt storage alone does not establish copy leakage.
- Default catalog sweep calls model backfill; permanent headline model drift is overstated for that path.
- Nemotron WAV duration preflight exists and cancellation/lifetime limits distinguish it from legacy diarization.

## Next work

- Continue remaining unresolved original IDs directly or through a verified functioning route. Do not repeat completed/unknown model attempts blindly.
- Reproduce high-impact mechanisms on exact-SHA Windows/macOS workers under native safety procedures before applying production fixes.
- Treat remediation as separately scoped tasks; this branch changes research artifacts only.

## Exact candidate status table

| ID | Status | Severity | Coordinator checked |
| --- | --- | --- | --- |
| `wave1_worker1_persistence-C01` | confirmed | medium | yes |
| `wave1_worker1_persistence-C02` | hypothesis | medium | yes |
| `wave1_worker1_persistence-C03` | confirmed | medium | yes |
| `wave1_worker1_persistence-C04` | confirmed | medium | yes |
| `wave1_worker1_persistence-C05` | hypothesis | medium | yes |
| `wave1_worker1_persistence-C06` | hypothesis | low | yes |
| `wave1_worker1_persistence-C07` | rejected | low | yes |
| `wave1_worker1_persistence-C08` | hypothesis | medium | yes |
| `wave1_worker1_persistence-C09` | hypothesis | low | yes |
| `wave1_worker1_persistence-C10` | confirmed | high | yes |
| `wave1_worker1_persistence-C11` | rejected | low | yes |
| `wave1_worker1_persistence-C12` | confirmed | low | yes |
| `wave1_worker1_persistence-C13` | hypothesis | medium | yes |
| `wave1_worker1_persistence-C14` | hypothesis | medium | yes |
| `wave1_worker1_persistence-C15` | hypothesis | medium | yes |
| `wave1_worker1_persistence-C16` | confirmed | medium | yes |
| `wave1_worker1_persistence-C17` | hypothesis | low | yes |
| `wave1_worker1_persistence-C18` | hypothesis | low | yes |
| `wave1_worker2_memory-C01` | hypothesis | medium | yes |
| `wave1_worker2_memory-C02` | hypothesis | low | yes |
| `wave1_worker2_memory-C03` | confirmed | medium | yes |
| `wave1_worker2_memory-C04` | hypothesis | medium | yes |
| `wave1_worker2_memory-C05` | unresolved | unassigned | no |
| `wave1_worker2_memory-C06` | confirmed | low | yes |
| `wave1_worker2_memory-C07` | hypothesis | low | yes |
| `wave1_worker2_memory-C08` | hypothesis | low | yes |
| `wave1_worker3_config-C01` | unresolved | unassigned | no |
| `wave1_worker3_config-C02` | confirmed | high | yes |
| `wave1_worker3_config-C03` | confirmed | medium | yes |
| `wave1_worker3_config-C04` | unresolved | unassigned | no |
| `wave1_worker3_config-C05` | hypothesis | low | yes |
| `wave1_worker3_config-C06` | unresolved | unassigned | no |
| `wave1_worker3_config-C07` | unresolved | unassigned | no |
| `wave1_worker3_config-C08` | unresolved | unassigned | no |
| `wave1_worker3_config-C09` | unresolved | unassigned | no |
| `wave1_worker3_config-C10` | rejected | low | yes |
| `wave2_worker1_audio-C01` | unresolved | unassigned | no |
| `wave2_worker1_audio-C02` | unresolved | unassigned | no |
| `wave2_worker1_audio-C03` | unresolved | unassigned | no |
| `wave2_worker1_audio-C04` | unresolved | unassigned | no |
| `wave2_worker1_audio-C05` | unresolved | unassigned | no |
| `wave2_worker1_audio-C06` | unresolved | unassigned | no |
| `wave2_worker1_audio-C07` | unresolved | unassigned | no |
| `wave2_worker1_audio-C08` | unresolved | unassigned | no |
| `wave2_worker1_audio-C09` | unresolved | unassigned | no |
| `wave2_worker2_stt-C01` | confirmed | high | yes |
| `wave2_worker2_stt-C02` | confirmed | medium | yes |
| `wave2_worker2_stt-C03` | hypothesis | high | yes |
| `wave2_worker2_stt-C04` | hypothesis | medium | yes |
| `wave2_worker3_tts-C01` | hypothesis | medium | yes |
| `wave2_worker3_tts-C02` | unresolved | unassigned | no |
| `wave2_worker3_tts-C03` | hypothesis | medium | yes |
| `wave2_worker3_tts-C04` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C01` | confirmed | medium | yes |
| `wave2_worker4_local_ai-C02` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C03` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C04` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C05` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C06` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C07` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C08` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C09` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C10` | unresolved | unassigned | no |
| `wave2_worker4_local_ai-C11` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C01` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C02` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C03` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C04` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C05` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C06` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C07` | unresolved | unassigned | no |
| `wave3_worker1_bridge-C08` | unresolved | unassigned | no |
| `wave3_worker2_window-C01` | confirmed | high | yes |
| `wave3_worker2_window-C02` | unresolved | unassigned | no |
| `wave3_worker2_window-C03` | unresolved | unassigned | no |
| `wave3_worker2_window-C04` | unresolved | unassigned | no |
| `wave3_worker2_window-C05` | unresolved | unassigned | no |
| `wave3_worker2_window-C06` | unresolved | unassigned | no |
| `wave3_worker2_window-C07` | unresolved | unassigned | no |
| `wave3_worker2_window-C08` | unresolved | unassigned | no |
| `wave3_worker2_window-C09` | unresolved | unassigned | no |
| `wave3_worker2_window-C10` | unresolved | unassigned | no |
| `wave3_worker2_window-C11` | unresolved | unassigned | no |
| `wave3_worker3_tile-C01` | hypothesis | high | yes |
| `wave3_worker3_tile-C02` | confirmed | medium | yes |
| `wave3_worker3_tile-C03` | unresolved | unassigned | no |
| `wave3_worker3_tile-C04` | rejected | medium | yes |
| `wave3_worker4_settings-C01` | unresolved | unassigned | no |
| `wave3_worker4_settings-C02` | unresolved | unassigned | no |
| `wave3_worker4_settings-C03` | unresolved | unassigned | no |
| `wave3_worker4_settings-C04` | unresolved | unassigned | no |
| `wave3_worker4_settings-C05` | unresolved | unassigned | no |
| `wave3_worker4_settings-C06` | unresolved | unassigned | no |
| `wave3_worker4_settings-C07` | unresolved | unassigned | no |
| `wave3_worker4_settings-C08` | unresolved | unassigned | no |
| `wave3_worker4_settings-C09` | unresolved | unassigned | no |
| `wave3_worker4_settings-C10` | unresolved | unassigned | no |
| `wave4_worker1_privacy-C01` | unresolved | unassigned | no |
| `wave4_worker1_privacy-C02` | unresolved | unassigned | no |
| `wave4_worker1_privacy-C03` | unresolved | unassigned | no |
| `wave4_worker1_privacy-C04` | unresolved | unassigned | no |
| `wave4_worker1_privacy-C05` | unresolved | unassigned | no |
| `wave4_worker2_installers-C01` | hypothesis | medium | yes |
| `wave4_worker2_installers-C02` | unresolved | unassigned | no |
| `wave4_worker2_installers-C03` | hypothesis | medium | yes |
| `wave4_worker2_installers-C04` | unresolved | unassigned | no |
| `wave4_worker3_cicd-C01` | confirmed | medium | yes |
| `wave4_worker3_cicd-C02` | confirmed | medium | yes |
| `wave4_worker3_cicd-C03` | confirmed | low | yes |
| `wave4_worker3_cicd-C04` | hypothesis | medium | yes |
| `wave4_worker3_cicd-C05` | hypothesis | low | yes |
| `wave4_worker3_cicd-C06` | hypothesis | low | yes |
| `wave4_worker3_cicd-C07` | hypothesis | medium | yes |
| `wave4_worker3_cicd-C08` | hypothesis | medium | yes |
| `wave4_worker3_cicd-C09` | confirmed | medium | yes |
| `wave4_worker3_cicd-C10` | hypothesis | medium | yes |
| `wave4_worker3_cicd-C11` | confirmed | medium | yes |
| `wave4_worker3_cicd-C12` | hypothesis | low | yes |
| `wave4_worker3_cicd-C13` | confirmed | medium | yes |
