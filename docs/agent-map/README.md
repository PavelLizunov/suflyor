# Suflyor project map: partial research

This directory contains a historical heuristic index and architectural notes, not an accepted exhaustive code audit. Use current source, manifests and tests to verify behavior. The original `100%` figures counted input lines, not reviewed semantics or complete symbols.

## Where to start

- [Reconciliation task](../goal-agent-map-reconciliation.md): approved scope, constraints and remaining work.
- [Frozen evidence snapshot](reconciliation/snapshot.json): exact source baseline and SHA-256 hashes of the existing Grok reports.
- [Grok summary](reconciliation/SUMMARY.md) and [candidate register](reconciliation/candidates.json): all 119 original claims source-triaged; 39 source mechanisms confirmed, 75 hypotheses and 5 rejected. Caller-level follow-up corrected an earlier UI-thread assumption. No native reproduction or independent acceptance is implied.
- [Source-linked features](features/README.md): 15 bounded contracts covering speech/session/storage, AI/vision/local models, TTS/OCR, personal memory, config transfer, hotkeys/window/capture, KB/archive/re-STT/summary/coaching, updater/release and MLX/Hermes. Source references and limits are explicit; this is not complete project coverage.
- [Model registry](reconciliation/speech-models.md): distinguishes transcription, speaker diarization, CoreML history and source-versus-release status.
- [Continuation queue](reconciliation/NEXT-STEPS.md): exact remaining full-map and native evidence work.
- [Artifact manifest](manifest.json): measured record counts, hashes and explicit completeness limits.
- [Operations](operations/): reproducibility and checkpoint/resume instructions. No independent automatic session-restart service is claimed.

## Historical index

The earlier publication contains 303 file maps and an inventory of 828 source paths. The counts describe that historical extraction, not the current repository size or a proof of coverage.

| Record | What it means | What it does not establish |
| --- | --- | --- |
| [Files](inventory/files.jsonl) | Historical file hashes and classifications | A coherent immutable source snapshot was not enforced in the original run |
| [Partitions](inventory/partitions.json) | Historical grouping into B01–B15 | Full review of every member |
| [Symbols](records/symbols.jsonl) | 4,500 regex matches, including functions, constants and Slint properties | Compiler-resolved AST, complete methods, reliable scope or signatures |
| [Types](records/types.jsonl) | 406 declaration candidates | Full fields, variants, generics or platform resolution |
| [Errors](records/errors.jsonl) | Source-verified WSOLA error enum; see its provenance | Every error type or recovery path |
| [Error sites](records/error_sites.jsonl) | 461 historical bailout/context matches | Audited propagation, recovery or reachable consequences |
| [Config sites](records/configs.jsonl) | 213 historical access candidates | Complete key schema, defaults, persistence and UI wiring |
| [Hotkey sites](records/hotkeys.jsonl) | 18 historical textual matches | Complete registration and functional dispatch matrix |
| [Behavior sites](records/behaviors.jsonl) | 457 historical threading/channel matches | Proven race, safety, bounded memory or complete dataflow |

[Per-file maps](files/) retain their original heuristic output. Rust and Slint were matched with regexes; the final generator did not parse other languages. An empty table is not evidence that a file declares no symbols. Line counts are input metadata only.

## Review provenance

[O1–O8](reviews/) are historical candidate evidence. O1 and O2 were recovered from tool output; O2 has output-framing contamination. O3–O8 were reconstructed or edited by the previous coordinator and are not preserved verbatim independent reviewer reports. The previous final reviewer returned `changes_required`; no subsequent independent acceptance was established. Historical contradictions remain until individually reconciled.

New reconciliation uses only explicit Gemini and Opus routes. Existing Grok reports are inspected as untrusted candidate claims; no new Grok or Astra calls are authorized. The speech-model inventory distinguishes transcription from speaker diarization, including the recently added optional Nemotron V3 path.

## Reading and querying

Search the records to locate a candidate, then read its implementation and callers. Use repository search/read tools when available. Local Python can inspect JSONL without executing application code:

```python
import json
from pathlib import Path
for line in Path('docs/agent-map/records/symbols.jsonl').read_text().splitlines():
    item = json.loads(line)
    if item.get('name') == 'replace_session':
        print(item)
```

## Outstanding work

- Resolve native/runtime preconditions of the 75 Grok hypotheses; source triage of all original candidates is complete with counterevidence and reproduction limits retained.
- Reconcile old Opus claims and create source-linked feature and model contracts.
- Add reliable language-aware extraction and line/symbol semantics before claiming exhaustive coverage.
- Test durable resumption and leave unknown attempts unresolved rather than silently repeating work.
- Native Windows/macOS behavior, UI, builds and release checks require separately recorded exact-SHA worker evidence. They were not performed by the historical map generator.
