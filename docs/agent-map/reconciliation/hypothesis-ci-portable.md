# Original CI hypotheses: proportional portable fixture evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; original register IDs/claim text untouched. Eight [new fixture tests](<../operations/test_ci_hypothesis_fixtures.py>) exercise actual Git/Bash against own temporary repositories and a labelled **source-extracted Python path-classifier model**, not native PowerShell gate/runtime. No Cargo, cleanup, installer, release, GitHub workflow/branch-policy mutation or native host action.

## C04 — missing push base

Original `wave4_worker3_cicd-C04` says missing `origin/master` falls back to `HEAD~1`, so a code commit followed by docs commit can make the native push classifier skip code. [Source](<../../../scripts/git-gate-native.ps1#L37-L76>) corroborates fallback and prefix/tier behavior. Fixture creates empty base → Rust change → README change in isolated repo, proves missing remote ref, obtains actual Git `HEAD~1...HEAD` and full baseline triple-dot name lists. Source-model fallback tier docs; full diff targeted/overlay-backend. **Portable mechanism observed**; not native PowerShell/Windows execution or exploit/required-hook proof. Registered hypothesis kept unchanged.

Counterevidence: [GitHub classifier invocation](<../../../.github/workflows/ci.yml#L43-L65>) uses explicit event base/head; actual [NUL-safe classifier](<../../../.github/scripts/is-docs-only.sh#L17-L55>) returns false for full code+docs fixture and errors on invalid base. Native fallback fixture does not establish server CI bypass, and missing base with a truly single code commit is not falsely docs. Hook/admin/force-push/branch-protection preconditions unverified.

## C05 — Git quoted path prefix

Original `wave4_worker3_cicd-C05` groups spaces/quotes/Unicode. [Native source](<../../../scripts/git-gate-native.ps1#L55-L76>) reads newline-delimited Git names, only replaces backslashes, uses crate-prefix matching. Actual Git fixture shows:

| Input path kind | Observed Git/source-model result |
|---|---|
| ASCII space in Rust basename | Crate prefix preserved: affected overlay-backend; plain spaces **not reproduced** as prefix bypass |
| Double quote in Rust basename | Git emits double-quoted C-style name; prefix starts quote, affectedCrates empty in model; tier still targeted |
| Cyrillic with core.quotePath=true | Quoted path, no model crate; with false raw path, crate recognized |

The model converts escaped backslashes exactly as selected source, but does not claim byte-for-byte PowerShell semantics. No actual native cargo selection/hook bypass run. Same-name file classification isn't proof arbitrary executable behavior. Original hypothesis retained; refine preconditions to quoting-sensitive paths, not every path containing spaces.

Counterevidence: real GitHub classifier returns false on each code fixture (NUL separated names) and conservatively treats Rust→docs rename as code via no-renames. Actual tab/newline docs paths classify docs; equivalent code paths false. This addresses classifier robustness, not whether hostile/failing filenames are Windows-compatible.

## C06 — fail-closed gate versus advisory jobs

[Main gate](<../../../.github/workflows/ci.yml#L143-L162>) waits `changes,rust`, validates success/boolean and requires Rust success when needed. It doesn't depend on advisory macOS or validate, so shell validation/macOS aren't included in this one status. [macOS job](<../../../.github/workflows/ci.yml#L126-L140>) continue-on-error true. [Existing shell regression](<../../../.github/scripts/test-gate-logic.sh#L6-L51>) has 15 cases, all pass (expected rejected values/failures included). This is local source/shell-model evidence, not an actual Actions status graph or branch-protection acceptance. Required-context/admin policy external and uninspected; hypothesis unchanged.

## C12 — separate security versus required branch policy

[Security source](<../../../.github/workflows/security.yml#L47-L97>) runs gitleaks separately and cargo-deny always validates classification before code checks/docs no-op. Three-crate matrix remains narrower than five standalone crate map. New fixture checks source presence/selection and existing 15-case gate/deny logic tests; no network/advisory/key/exploit scanner execution. External branch protection choosing which contexts block merges was not queried; no “security missing on PR” conclusion. Original hypothesis unchanged.

## Exact checks and limits

Local: eight new research fixtures pass (including original-ID/hash/status integrity); existing docs classifier regression **24 cases**, gate/deny regression **15 cases** pass. Test helper temp repos configure dummy identities and disable hooks per command; never switch/clean project worktree. Existing shell test removes only its own temporary repo. Source/ranges validated by frozen checkpoint; compiler/native/independent review not run.

Not advanced in this slice: C07 arbitrary ref cleanup mutations (would need approved safety setup), C08 native build/evidence mismatch (requires exact worker), C10 whether shallow UI/headless CI permits specific visible defects (requires source/UI/native evidence). Portable mechanism fixtures don't promote any original candidate from hypothesis/confirm a runtime exploit. Next bounded checks should target these remaining preconditions or other high-risk original hypotheses, never repeat tests merely to increase counts.
