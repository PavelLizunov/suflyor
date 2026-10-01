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
| Recovery tests | `python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_checkpoint.py' -v`: 9 tests, OK | Helper mechanics only; no independent host-crash restart |
| Checkpoint/recover commands | Completed; null/misaligned lane files labeled `unaccepted_proposal` | No automatic redispatch or independent acceptance |
| Actual SQL migrations | Python SQLite 3.45.1 in-memory: migrations loaded; triggered FTS row deleted by session_id; no session FK on memory/diarization | Not native bundled rusqlite/Cargo tests or full DB durability |
| Git whitespace | `git diff --check`, staged check and first checkpoint `git show --check`: clean | Docs gate, not behavioral/native gate |
| Release observation | GitHub API/tag queries: latest observed RC `v0.38.1-rc.3` predates Nemotron introduction; task branch contains new source | No new publication or installer run |

The initial SQL fixture used a nonexistent `utterances.seq` column and failed before the check; the fixture was corrected against the real migration and rerun successfully. Two model-correction scripts initially used wrong constant names and failed before publishing corrected model inventory; final pins were copied from exact source. These ordinary failed checks are not hidden as passing attempts.

## Model dispatch outcome

- `workflow-1`: four Opus final results null with partial, topic-misaligned JSON; one Gemini summary returned, then corrected against source. No blanket Opus acceptance.
- `workflow-2`: eight Opus final results null; no exact-slice files produced; reason not supplied. No quota/exhaustion/approval diagnosis inferred.
- All jobs collected; none left running by this reconciliation pass. No Astra or new Grok calls.

## Remaining acceptance

- 40 confirmed source mechanisms, 74 hypotheses, 5 rejected original claims. Source mechanisms do not imply 40 reproduced bugs.
- Native build/tests/live UI/stealth/driver latency/fault/security reproductions: **not run**.
- Full all-language symbols, feature contracts, line semantics and independent acceptance: **not complete**.
- DSH crash-restart supervisor: **not implemented**; durable checkpoint/portable continuation tested only.

The evidence-overstatement incident is recorded as `INC-1381` in the local Trajectory ledger. The ledger is operational evidence outside this repository; no private session log was copied into public research.
