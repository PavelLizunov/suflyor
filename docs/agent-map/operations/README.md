# Research operations and recovery

## Purpose and limits

This procedure preserves a verifiable research checkpoint. It does not dispatch models, monitor DSH independently, restart a session, install a daemon or resume jobs after host failure. DSH goals support continuation while the host/session permits it; they are not proof of a crash-recovery watchdog.

The obsolete local supervisor marks 389 regex units `accepted` and performs shallow key checks. That is historical extraction state, not semantic acceptance. Do not invoke its stale-lease retry logic while a model job outcome is unknown.

## Discover current work

1. Read [the task record](../../goal-agent-map-reconciliation.md) and [snapshot](../reconciliation/snapshot.json).
2. Inspect actual Git status and current jobs using DSH tools. Do not switch another task's branch or overwrite its uncommitted files.
3. Read the [candidate register](../reconciliation/candidates.json). Unresolved and hypothesis statuses remain open even when a report passes structural checks.
4. Both dispatches have settled and their outcomes were collected: [first receipt](../reconciliation/dispatch-receipt.json) (`workflow-1`) and [exact-claim receipt](../reconciliation/exact-dispatch-receipt.json) (`workflow-2`). Four initial Opus lanes returned null with partial files; eight bounded exact-claim lanes returned null without files. No failure reason was supplied. Do not infer quota exhaustion or replay them automatically. Coordinator source inspection is recorded separately.

## Local commands

Run from the repository root. Only Python's standard library is used; these are research checks, not native Suflyor builds.

```bash
python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py' -v
python3 -B docs/agent-map/operations/checkpoint.py verify
python3 -B docs/agent-map/operations/checkpoint.py checkpoint
python3 -B docs/agent-map/operations/checkpoint.py recover
```

`verify` reads frozen file/report hashes and candidate outputs. If raw local Grok files are absent in a fresh clone, it verifies linked redacted copies against provenance; it never claims redacted bytes equal raw bytes. [Source portability](../reconciliation/source-portability.json) records exact baseline Git-blob and frozen-worktree hashes for verified CRLF/LF differences. No arbitrary source normalization is allowed. It validates canonical coordinator IDs/counts/source ranges separately from failed worker proposals. Pending report files stay pending; no structural check decides the truth of a claim.

`checkpoint` stores the same report transactionally in a local SQLite database with `synchronous=FULL`. It preserves previous structurally validated artifact identities when inputs drift or output validation fails. It does not confer semantic acceptance.

`recover` revalidates current input hashes/canonical records/feature references, preserves historical artifact identities and changes recorded running attempts to `unknown`. Drift yields a nonzero exit; a prior saved clean report is not treated as current acceptance. It never dispatches replacements. Reconcile the job ID through DSH `job_list` and `job_output` first. If the original host cannot establish the outcome, record that fact, retain any partial artifacts, and make a bounded retry decision explicitly.

The default local database is `.campaign-state/reconciliation.sqlite`; it is not committed. Portable recovery uses the GitHub task record, frozen hashes, owned JSON/Markdown lane outputs and Git history. Never commit private session logs or credentials to make recovery easier.

## Test evidence

The latest test execution passed 24 dependency-free tests: 20 recovery/provenance safeguards and four real-migration SQLite fixtures. They cover canonical original-report identity, coordinator counts/references, feature evidence limits, exact Git text-form portability, current drift at recovery, failed proposals and curated/FTS/diarization replacement behavior. [An exact covering-SHA tracked-only test](../reconciliation/portable-recovery-evidence.json) at `059a04b1` also restored 119 canonical records without raw Grok files or prior database; all 24 archived tests passed and no worker proposal was promoted to acceptance. This tests helper/SQL mechanics, not native application behavior or host-failure session restart.

## Publication discipline

- Commit and push coherent research checkpoints to `codex/research-reconciliation`.
- Preserve raw preexisting Grok files without changes. Their hashes are recorded; publication requires redacted copies because some raw reports contain private-network examples.
- Never label copied/reconstructed historical review text as an original independent response.
- Record which checks actually ran and which require Windows/macOS workers.
- No Astra or new Grok workers in this task. Gemini is reserved for short worthwhile mechanical slices; Opus reviews are bounded.
