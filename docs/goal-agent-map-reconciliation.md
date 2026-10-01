# Suflyor research reconciliation

## Authorization and intended result

The owner approved correction of the partial agent-map, frozen source evidence, verification of existing Grok reports, tracing the new audio model, and tested resumability. Use Gemini and Opus only; no Astra or new Grok calls. Gemini is optional and must justify short independent assignments. Preserve the original exhaustive-map objective without claiming it already exists.

## Frozen baseline

- Reviewed source commit: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`.
- Task branch: `codex/research-reconciliation`.
- Previous research publication: `e14ff1936349cd6790be749d68f5c7b5737af926`.
- Earlier manifest incorrectly identifies `a4c998740de4d25c3939d1bcf63a7c07af820682` as the sole analyzed source despite subsequent changes.
- Fourteen untracked Grok reports contain 119 candidate claims. Their bytes remain untouched. Hashes and source applicability will be recorded before review. File labels do not independently prove backend identity or a precise original snapshot.

## Scope and invariants

- May change task-owned research documentation, agent-map labels/metadata and validated evidence records.
- Inspect production sources and historical commits read-only. No production fixes, releases, native builds, SDK installs, service restarts, merges or force pushes in this stage.
- Do not overwrite raw Grok reports, other audit evidence, private material, or another task's handoff.
- Archive original metadata by Git provenance; never manufacture reviewer receipts, usage totals, source coverage or a favorable verdict.
- Distinguish STT (speech to text) from diarization (speaker segmentation). Trace the exact added model instead of assuming the two are equivalent.
- Use bounded read-only Opus assignments with exact source references, separate report ownership, no recursive delegation, and no worker Git operations.

## Acceptance checks

1. Documentation describes regex extraction as heuristic candidate indexing, not AST or semantic completeness.
2. All 119 Grok claims have stable IDs and explicit confirmed/hypothesis/rejected/duplicate/obsolete/unresolved classifications with source evidence and actual verification limits.
3. Speech/model registry traces config, runtime selection, UI, installers, native process isolation, tests and release evidence; unknown publication status remains unknown.
4. Artifact hashes/counts and baseline drift are measured. No hardcoded 100% semantics.
5. Recovery is tested for durable state, preserved completed results, interrupted/unknown attempts, and snapshot drift. No promise of independent automatic session recovery without a supported and tested transport/watchdog.
6. Docs/JSON structural checks and diff whitespace checks pass. Commit and push coherent task-owned checkpoints to the configured GitHub task branch. Do not add raw untracked reports until privacy review permits their publication; record hashes meanwhile.

## Current state and next steps

- Initial reality check completed: existing 389 accepted attempts use `mechanical_ast/ast-extractor-v1`, not Gemini. No background campaign runs.
- Existing map contains false completeness labels, unsupported language maps, incorrect WSOLA error documentation and three stale manifest hashes.
- Git history identifies an optional Windows Nemotron V3 diarization engine; actual STT routes and other model history still need review.
- Frozen source/report inventory and exact redacted original claim register are persisted. Raw Grok bytes remain unchanged; redacted copies can be published.
- Historical map false completeness labels corrected; actual WSOLA enum corrected; measured manifest replaces fake completion.
- One short Gemini speech-model inventory returned and was corrected against actual model pins/licenses/WAV duration and GitHub release data. Nemotron V3 is an unreleased optional Windows diarizer, not a new text recognizer.
- workflow-1: four Opus final results null with partial misaligned candidate files. workflow-2: eight exact-claim Opus results null, no files. Cause not reported. Both receipts retained; no further model replay until route outcome is understood.
- Coordinator checked all 119 exact original Grok claims: 39 source mechanisms confirmed, 75 hypotheses and 5 rejected. These are not native runtime reproductions. Source counterevidence rejects several broad claims; composite claims retain their unsupported parts explicitly.
- 24 local research tests pass (20 recovery/provenance, 4 SQLite schema fixtures), including original-report identity and post-checkpoint drift rejection. Independent DSH session watchdog is not implemented.
- Checkpoints `c3051441` and `927003f2` are published on the task branch. Full original exhaustive project-map objective remains incomplete; do not mark it complete.
- Continuation round 1: create source-linked contracts for live transcription, session lifecycle/storage and speaker diarization; validate original-claim/coordinator records through checkpoint tooling and test recovery from a real tracked-only Git snapshot without raw local reports. Scope remains documentation/research helper only; no production edits or model redispatch.

## Continuation round 1 outcome

- Three bounded source-linked feature contracts added; 83 unique source references indexed after spotcheck corrections. Caller-level session dispatch downgraded blanket UI-blocking claim to hypothesis; original Grok counts are now 39 confirmed mechanisms, 75 hypotheses, 5 rejected.
- Canonical report/claim/receipt/reference/count validation added; 24 tests pass (20 helper/provenance + 4 real-migration SQL fixtures). Tracked-only recovery found CRLF/LF license-byte drift, handled only by exact recorded baseline Git hash forms. Exact covering-SHA portable recovery at `059a04b1b90017fab032c25fc93d68b2be1f39eb` passed all 24 tests plus verify/checkpoint/recover with Git-archived files only, no raw reports or prior DB.
- Short Gemini feature-counterevidence spotcheck workflow-3 completed and was collected; corrections preserve its receipt without whole-project acceptance. Managed local Whisper is large-v3-turbo Q8, meeting-ending is a status hint, and successful diarization rerun can replace manual names.
- Eleven selected historical review corrections recorded. No production files changed, native builds/services/raw-report mutation/Astra/new Grok calls. Next: publish verified feature/recovery checkpoint and continue remaining feature contracts/all-language coverage.
