# Reconciliation verification evidence

## Round 5 — polyglot parser checkpoint (local, covering SHA pending)

- Entry checkout `7909617a`; unchanged handoff 62 research tests passed with pinned original parsers and saved 13,961-node validation.
- Task-owned changes only: frozen syntax index/parser helper/fixtures/hash pins/provenance and living research docs. No production source/dependency/UI/script behavior changes.
- Extended index: 840 paths; 290 syntax successes, seven PowerShell parse-error files with partial nodes, one unsupported NSIS, 22 exclusions, 520 nonselected; 16,243 navigation nodes. Rust/Python 13,961-node subtotal preserved. Source-range/hash/count/parent validation returned zero errors; frozen 119-claim checkpoint validator issues empty, original 39/75/5 counts unchanged.
- Current local test suite: **71 passed, zero skips**, all supplied grammars pinned. 23 syntax/index fixtures plus previous 48 recovery/SQL/source tests. Offline hash-locked install of five added wheels exercised in a new ignored target. Slint shared-library archive and library SHA256 verified before loading; ABI 15/binding 0.25.2 smoke/fixtures/full-source parse passed.
- Missing anonymous CST delimiter tokens now explicitly traversed in Rust/polyglot parsers; tests revealed and corrected node-kind/name assumptions before accepted generation. PowerShell ERROR spans not suppressed or mislabeled as source bugs.
- [Parser decision/provenance/limits](parser-polyglot-research.md), [wheel/library metadata](parser-polyglot-provenance.json), [current syntax scope](../syntax/README.md).
- Native PowerShell parser, Cargo/Swift builds, live UI/model/audio/installer behavior, full semantic/caller coverage and independent review **not run/not accepted**. Permitted tools currently cannot select explicit Gemini/Opus except workflow, which the owner has not explicitly requested in this session; no inherited-model substitution. Configured web-search HTTP 402 did not block direct upstream/registry inspection.
- Exact committed-SHA archive repeat is pending; local checks are not yet a portable covering-SHA receipt. This is an incomplete research checkpoint, never goal completion.


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
| Recovery/SQL tests | `python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py' -v`: 48 tests, OK (22 provenance/recovery + 7 SQLite fixtures + 19 source-seam checks); existing mocked Hermes suite: 3 tests OK | Helper and Python SQLite mechanics only; no native application or independent host-crash restart |
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

## Continuation round 2 evidence

- Six additional bounded contracts cover AI/vision/provider, managed local lifecycle, read-aloud/OCR, personal memory, portable config/Settings and hotkeys/window/capture. Nine contracts now index 277 source references; links/ranges checked, not an all-function coverage percentage.
- Direct source recheck corrected TTS Windows JobObject absence, confirms init actually calls warm, AI channel=64/shared permit=2, stream timeout=120s/completion=180s, and current OCR in-memory stdin/stdout with dimension guards. Several older prose/comment assumptions were not propagated.
- Seven SQL fixtures use actual migrations for FTS/curated tables, rerun name replacement, approval V2-field omission, source restore and active/default-profile query. Seven source-seam checks protect the exact enum/hotkey/AI/TTS/OCR/normalizer declarations without claiming compilation or behavior.
- First source-seam run failed on two incorrect delimiter substrings (permit select form and end-of-spawn helper marker); tests were rebased onto inspected actual source and rerun. These are fixture failures, not application fixes.
- Bounded Gemini workflow-4 completed with issues. Receipt records accepted queue/provenance/fallback caveats and rejected stale/misquoted rows; hypothetical Codex HTTP summary consequence is not established because its exclusive flag requires local managed prep.
- [Parser research](parser-research.md) records installed-tool absence and live upstream alternatives. Verdict Compose (stdlib AST/TOML + pinned syntax grammars); no parser package/toolchain installed and no accurate AST index claimed.
- [Exact round-2 covering-SHA receipt](portable-recovery-round2.json) at `8cd96a8bcbf3a68c9af8bde43bfa842fd34457a6` passed all 34 archived tests and verify/checkpoint/recover: nine contracts, 119 canonical records, zero rejected-proposal promotions, no raw reports/prior DB/dependency install or overlays.

## Continuation round 3 evidence

- Six additional principal contracts map KB/reference, archive/playback/re-STT, summary/conspect/coaching, updater/build/release, MLX owned sidecar/install and Hermes bridge/plugin/profile-prep. Fifteen bounded contracts are now indexed; not every source/caller/native branch is accepted.
- Source checks corrected empty-query KB palette, UTF8-byte reference budget, synchronous archive delete/partial cleanup, force=false audio summary wrapper, transcript-only recap cache and best-effort conspect save. Numeric live WPM/filler pill is not established by current coaching style source/UI search.
- Updater draft erroneously claimed final redirect validation. Actual code allowlists original URL, follows default redirects and hashes bytes before write, with no `.url()`/redirect policy. Contract corrected; hypothetical arbitrary executable remains unproven because digest check exists.
- MLX contract distinguishes fast marker runtime load vs full file hash at install, exact owned-child readiness and serialized inference vs unbounded waiters. Hermes contract distinguishes configured nonloopback bind from stale loopback-only header, catalog summary vs conspect and save failure vs response success.
- Source-seam fixtures initially failed on guessed identifiers/substring shapes, then reread actual implementation and reran; all 48 research tests passed (22 recovery/provenance +7 SQL +19 source seams). Three existing plugin limit tests ran with requests mocked, no network/user data.
- workflow-5 bounded Gemini retrospective spotcheck settled with null result and no file output; receipt retains no reason/acceptance. Coordinator source review is not independent model acceptance.
- [Round-3 exact covering-SHA receipt](portable-recovery-round3.json) at `3188e6b05f36273bd32faebec3ab64dad0301afe` reran 48 research + three existing mocked Hermes tests and verify/checkpoint/recover from tracked-only Git archive: 15 contracts, 119 records, zero failed-proposal promotions, no overlays/raw reports/prior DB/network/native builds.

## Continuation round 4 / context handoff

- Pinned prebuilt MIT parser wheels installed only under ignored research target, no production/global/DSH runtime dependency change. First binding 0.26.0 passed fixtures but SIGSEGV on large traversal; per-file attempt reported 89 native failures. Failed receipt preserved; not accepted as an index.
- Alternate binding 0.25.2 + Rust grammar0.24.2 parsed all selected frozen Rust/Python files in subprocess isolation: 224 files (220 Rust/four Python), 13,961 nodes/declarations including 7,692 macro invocations, zero observed parse/native failures. 74 unsupported,22 protected/vendor excluded,520 nonselected remain explicit.
- Syntax validator checks exact source SHA, UTF8 byte/line ranges, signature prefixes, parents and counts with no semantic acceptance. It passed for all saved output; Python stdlib version3.12 and exact wheel provenance preserved.
- Full pinned suite passed 62 research checks plus three existing mocked Hermes tests. Syntax absence can skip Rust fixtures and is not parser acceptance. [Exact covering-commit receipt](portable-recovery-round4.json) at `3bfc15ae5ff7fdc405f46055ab1d60a8c943b3f8` passed all62 research+3 mocked tests, saved syntax validation and checkpoint verify/recover. Parser wheels supplied from isolated pinned site, not shipped Git archive; no raw/prior state/overlays.
- User requested context transfer and commit of completed work. [Continuation prompt](CONTINUE-PROMPT.md) records scope/next tasks; whole goal remains incomplete and must not be marked complete because of this handoff.

## Model dispatch outcome

- `workflow-1`: four Opus final results null with partial, topic-misaligned JSON; one Gemini summary returned, then corrected against source. No blanket Opus acceptance.
- `workflow-2`: eight Opus final results null; no exact-slice files produced; reason not supplied. No quota/exhaustion/approval diagnosis inferred.
- All jobs collected; none left running by this reconciliation pass. No Astra or new Grok calls.

## Source findings beyond the original Grok register

[Additional source findings](additional-source-findings.json) preserve three follow-up contracts without changing the 119 original IDs: successful diarization rerun replaces prior manual speaker names, meeting-ending is a visual hint rather than automatic stop, and candidate approval omits V2 source/entity/normalization metadata from the minted item. The replacement mechanism is reproduced in the actual shipped schema through Python SQLite; native analysis/identity remapping is not tested. [Selected historical corrections](historical-review-corrections.json) cover 14 selected historical/code-summary assertions, not every historical sentence.

## Remaining acceptance

- 39 confirmed source mechanisms, 75 hypotheses, 5 rejected original claims after caller-level follow-up. Source mechanisms do not imply 39 reproduced bugs. Normal Start/Stop dispatch counterevidence downgraded the earlier blanket UI-blocking claim.
- Native build/tests/live UI/stealth/driver latency/fault/security reproductions: **not run**.
- Full all-language symbols, feature contracts, line semantics and independent acceptance: **not complete**.
- DSH crash-restart supervisor: **not implemented**; durable checkpoint/portable continuation tested only.

The evidence-overstatement incident is recorded as `INC-1381` in the local Trajectory ledger. The ledger is operational evidence outside this repository; no private session log was copied into public research.
