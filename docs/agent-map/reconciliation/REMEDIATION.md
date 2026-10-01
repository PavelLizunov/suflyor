# Remediation queue and acceptance boundaries

This is a prioritized queue after original-claim source triage, not permission to apply every suggestion. Production source was not modified. Each fix needs a proportional task record, invariant decision and exact-SHA native evidence for the affected surface. Normal development remains targeted; Full is reserved for owner-authorized stable publication.

## Highest-priority verification/fix candidates

| Work package | Source-inspected mechanism | Acceptance evidence needed |
| --- | --- | --- |
| Catalog safety and backup contract | Pre-migration raw DB copy ignores checkpoint outcome; curated memory/diarization not recoverable by deleting catalog | Two-connection WAL snapshot fixture, restored curated rows, migration failure rollback; no destructive reset of owner data |
| Live STT queue bounds | Spawn/PCM ownership precedes semaphore acquisition; permits limit execution but not queued memory | Slow mocked inference, bounded outstanding PCM/tasks, preserved utterance order and explicit drop/backpressure policy |
| Journal delivery/durability | Unbounded journal command channel; flush ack not stable-storage fsync; error-loop continuation | Injected failing writer, bounded memory, clear ack meaning; avoid blocking realtime audio/UI |
| Screenshot privacy | Aux-window reveal logs WDA failure then moves on-screen; effective state is bar-only; diagnostics UI uses raw details | Exact-binary WDA failure injection and before/after capture evidence; sanitized diagnostics examples; decide fail-closed vs accessible control UX |
| Config persistence | Some handlers mutate live config before failed save; temp delete/rename fallback can lose old file | Permission-failure fixtures preserving prior runtime/disk, explicit safe fallback, live Settings state verification |
| Selective gate triggers | Native Markdown-only check misses compiled knowledge; NSI/script-only changes map to no owning crate | Classifier table tests covering knowledge, installer/version guard, CI/build seams; no unrequested Full promotion |

## Important but unproven runtime hypotheses

- Stale session/generation writebacks in auto-tile, namer and stream completion.
- Slow Settings endpoint replies replacing newer choices; reused in-progress install state.
- TTS pipe writes under mutex with large SPEAK lines; reader lifecycle and queue correlation.
- Native audio stop/join latency, per-chunk remainder loss, macOS pending TCC lifecycle.
- Same-user writable-asset TOCTOU, platform archive traversal and uninstaller reparse behavior.
- Large streaming Markdown/conversation memory cost and unknown hardware context feasibility.

Do not convert these into accepted bugs without the relevant precondition and reproducer. The [candidate register](candidates.json) stores source references, counterevidence and remaining checks per original ID.

## Conclusions not to carry forward

- Do not delete catalog automatically when journals vanish: retained catalog history and curated tables are intentional.
- Do not invent session FK cascade into memory/diarization: actual migrations have no such FKs.
- Do not claim FTS5 `DELETE WHERE session_id` fails: locally executed real schema works.
- Do not infer system-prompt clipboard leak merely from storage: clipboard formatter filters system messages.
- Do not claim macOS GigaAM GPU toggle is ignored: source selects CoreMl when enabled; CPU defaults are a separate memory-policy change.
- Do not call Nemotron a newly integrated text recognizer: this optional Windows route outputs speaker RTTM intervals.
- Do not infer official release from branch/version changes: observed latest published RC predates Nemotron.
- Do not infer license isolation from process boundaries alone. Runtime and model notices remain separate review subjects.

## Research still required for the full original map

The 119 Grok candidates are now triaged, but that does not complete the full project-map objective. Remaining work includes reliable language-aware symbol extraction, every feature/config/hotkey contract, checked call/dataflow relationships, line/symbol coverage with real evidence, reconciliation of historical O1–O8 assertions, and native exact-SHA acceptance. The current repository backup is a useful partial evidence base, not an exhaustive certification.
