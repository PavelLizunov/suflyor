# Reconciliation verification evidence

## Reviewed source and changed scope

- Source baseline: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`.
- Task branch: `codex/research-reconciliation`.
- Change scope: research/docs metadata and dependency-free checkpoint/test helper under docs. No production Rust/Slint/installer/CI source changed.
- First GitHub checkpoint: `c3051441b8071f83981b079604197dda4d7f5417` (remote branch SHA verified after push).

## Executed local checks

| Check | Observed result | Boundary |
| --- | --- | --- |
| Exact candidate register | 119 unique original IDs with original redacted claims | Source triage, not exhaustive project review |
| Candidate source references | All start/end ranges exist and fall within actual file lines | Range validity does not independently prove claim truth |
| Input continuity | All frozen source paths and 14 raw Grok hashes unchanged | Historical pre-snapshot source coherence remains unestablished |
| JSON/JSONL decode | Every published research JSON/JSONL decoded | Record semantics/symbol completeness not compiler-verified |
| Python syntax | Checkpoint/test files parsed successfully | Not native Suflyor application compilation |
| Navigation | Main corrected README/model/recovery/topology links resolve | Historical generated review links not all audited |
| Recovery/SQL tests | `python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py' -v`: 24 tests, OK (20 provenance/recovery + 4 SQLite fixtures) | Helper and Python SQLite mechanics only; no native application or independent host-crash restart |
| Checkpoint/recover commands | Completed; null/misaligned lane files labeled `unaccepted_proposal` | No automatic redispatch or independent acceptance |
| Actual SQL migrations | Python SQLite 3.45.1 in-memory: migrations loaded; triggered FTS row deleted by session_id; no session FK on memory/diarization | Not native bundled rusqlite/Cargo tests or full DB durability |
| Git whitespace | `git diff --check`, staged check and first checkpoint `git show --check`: clean | Docs gate, not behavioral/native gate |
| Release observation | GitHub API/tag queries: latest observed RC `v0.38.1-rc.3` predates Nemotron introduction; task branch contains new source | No new publication or installer run |

The initial SQL fixture used a nonexistent `utterances.seq` column and failed before the check; the fixture was corrected against the real migration and rerun successfully. Two model-correction scripts initially used wrong constant names and failed before publishing corrected model inventory; final pins were copied from exact source. These ordinary failed checks are not hidden as passing attempts.

## Continuation round 1 evidence

- Three bounded feature contracts were added for live transcription, speaker diarization and session lifecycle/storage, with 83 unique linked source ranges/file references after spotcheck corrections. Link/range validation passed; semantic/native coverage remains partial.
- Canonical coordinator records are now checked separately from unaccepted worker proposals; receipt mismatches, missing counterevidence and out-of-range source references fail validation.
- A real `git archive 927003f2` extraction (no raw Grok reports and no prior campaign DB) initially failed source continuity on four vendored license/notice files because Git normalized CRLF to LF. Investigation proved exact CRLF-to-LF equality and preserved both hashes; all 34 verified source text-form differences are recorded for cross-platform checkout compatibility.
- Repeating tracked-only recovery with the updated helper and portability record passed: 119 canonical records recovered, zero worker proposals accepted, no automatic dispatch. The precommit experiment used the updated working helper. A subsequent [exact covering-SHA experiment](portable-recovery-evidence.json) at `059a04b1b90017fab032c25fc93d68b2be1f39eb` used only Git-archived files and passed all 24 tests plus verify/checkpoint/recover, without raw reports or prior DB. No file overlays were used.
- Normal session Start/Stop and recovery callbacks spawn work on Tokio runtime. This corrected the historical blanket UI-blocking classification; final event-loop shutdown has a synchronous stop but is a different path.
- Bounded Gemini spotcheck `workflow-3` completed with issues. Coordinator confirmed and fixed managed Whisper Turbo identity, capture-watchdog vs meeting-ending hint and diarization run/persist/poll links. Two quoted guarantees were absent from current documents and were not accepted as existing errors. Successful diarization replacement of manual names was additionally verified against SQL shape; native speaker rerun remains unexecuted.
- Selected historical O1–O8 corrections are recorded separately (11 items); no blanket historical review acceptance or full all-line claim.

## Model dispatch outcome

- `workflow-1`: four Opus final results null with partial, topic-misaligned JSON; one Gemini summary returned, then corrected against source. No blanket Opus acceptance.
- `workflow-2`: eight Opus final results null; no exact-slice files produced; reason not supplied. No quota/exhaustion/approval diagnosis inferred.
- All jobs collected; none left running by this reconciliation pass. No Astra or new Grok calls.

## Source findings beyond the original Grok register

[Additional source findings](additional-source-findings.json) preserve two follow-up contracts without changing the 119 original IDs: successful diarization rerun replaces prior manual speaker names, and meeting-ending is a visual hint rather than automatic stop. The replacement mechanism is reproduced in the actual shipped schema through Python SQLite; native analysis/identity remapping is not tested. [Selected historical corrections](historical-review-corrections.json) cover 11 O1–O8 assertions, not every historical sentence.

## Remaining acceptance

- 39 confirmed source mechanisms, 75 hypotheses, 5 rejected original claims after caller-level follow-up. Source mechanisms do not imply 39 reproduced bugs. Normal Start/Stop dispatch counterevidence downgraded the earlier blanket UI-blocking claim.
- Native build/tests/live UI/stealth/driver latency/fault/security reproductions: **not run**.
- Full all-language symbols, feature contracts, line semantics and independent acceptance: **not complete**.
- DSH crash-restart supervisor: **not implemented**; durable checkpoint/portable continuation tested only.

The evidence-overstatement incident is recorded as `INC-1381` in the local Trajectory ledger. The ledger is operational evidence outside this repository; no private session log was copied into public research.
