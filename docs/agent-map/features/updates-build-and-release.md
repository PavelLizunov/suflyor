# Updates, build gates and publication: source-linked contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no update downloaded/executed, native gate/build, installer/uninstaller or release cleanup run. Code/branch/version, packaged artifact, published tag and required platform acceptance are separate facts.

## In-app updater

[check_latest](../../../overlay-backend/src/update.rs#L94-L127) queries official GitHub `/releases/latest` with 15-second timeout and exact canonical installer asset name. This API returns stable releases, not RC channel. [Version comparison](../../../overlay-backend/src/update.rs#L56-L78) compares numeric major/minor/patch plus final-vs-prerelease rank; it does not compare differing RC suffix numbers or implement full SemVer precedence.

[Allowed URLs](../../../overlay-backend/src/update.rs#L129-L157) require HTTPS/no userinfo/standard port, official repo path on GitHub or approved GitHub CDN host. [Expected digest](../../../overlay-backend/src/update.rs#L170-L197) re-queries latest and matches canonical asset URL; missing/malformed API digest is distinct from network error. [download_installer](../../../overlay-backend/src/update.rs#L194-L260) checks the **initial URL**, follows the HTTP client's default redirect behavior and reads bytes; it does not inspect final response URL or install a custom redirect policy. It verifies those bytes against digest obtained for the original official asset URL before writing fixed temp path. Digest mismatch fails closed, but per-hop/final destination trust is not enforced by a URL check here. Authenticode/release-account compromise is explicitly outside digest protection; no current installer signature acceptance is asserted.

[Settings callbacks](../../../slint-experiment/src/bin/overlay_host/settings_updates.rs#L35-L151) run network on detached runtime worker, set UI status from confirmed result and spawn installer only after returned verified path. [run_installer](../../../overlay-backend/src/update.rs#L264-L283) requires file exists and launches it without further digest check; writable temp path race remains a separate same-user precondition/hypothesis. Error text displayed by updater is not universally screenshot-sanitized by this module.

## Native gate selection

[Root policy](../../../AGENTS.md#L85-L98): docs, targeted normal development/prerelease, Full only explicit owner-authorized stable release. A normal multi-crate/high-risk diff does not auto-promote to Full. [Native classifier](../../../scripts/git-gate-native.ps1#L37-L84) chooses changed path set, affected crates and extension-only docs predicate; it is not identical to GitHub classifier.

[Native checks](../../../scripts/git-gate-native.ps1#L100-L184) parse changed PowerShell, run whitespace, then explicit full or per-affected-crate targeted fmt/clippy/tests. UI-only selected guards are narrower than full Slint test suite. Script/NSI-only diff can map no owning crate; compiled KB Markdown incorrectly falls docs-only in this native predicate. These source gaps are retained original Grok candidates, not fixes applied here.

[GitHub classifier](../../../.github/scripts/is-docs-only.sh#L20-L55) recognizes docs subtree/Markdown but explicitly excludes compiled knowledge and disables rename folding. [CI](../../../.github/workflows/ci.yml#L62-L110) runs Windows backend/Slint/Piper fmt/clippy/tests and ui-mcp check on non-doc path; this is not all-five-crate test proof. [Gate job](../../../.github/workflows/ci.yml#L142-L178) verifies selected rust result; macOS/validate/security contexts are separate and branch-protection configuration determines required checks. No GitHub green status was observed as native acceptance here.

[macOS gate](../../../scripts/git-gate-macos.sh#L6-L47) enforces arm64, exclusive heavy process preflight, >=40% memory availability and two jobs/test threads, then Swift/sidecar/host checks. Run only on authorized native worker exact SHA. DSH control plane lacks Cargo and must not be provisioned as builder for this task.

## Packaging and model isolation

[Build script](../../../scripts/build-slint-release.ps1#L16-L109) uses nonincremental release builds for host and isolated TTS/Tera sidecars, stages pinned Nemotron runtime only when Installer requested and validates its executable/DLL hashes. [DirectML and NSIS stage](../../../scripts/build-slint-release.ps1#L110-L174) resolves runtime DLL and invokes makensis if requested. Build output location/version metadata is not proof binary behavior or successful publication.

[NSIS](../../../scripts/slint-installer.nsi#L33-L96) checks only matching installed process paths, asks permission to stop owned copies, installs host/TTS/Tera/DirectML/Nemotron files and notices, and does not bundle large model weights. Neural runtimes remain separate processes; runtime pin/archive verification and model weight grants are independent licensing/security checks.

[Uninstall](../../../scripts/slint-installer.nsi#L125-L160) removes app files and opt-in local data/models. Reparse/path containment, malformed command quoting and unexpected DLL leftovers require throwaway native tests; no uninstaller fixture against owner environment is authorized by this source contract.

## Publication and cleanup boundaries

[Version guard](../../../slint-experiment/tests/version_guard.rs#L17-L78) checks Cargo/NSI and macOS plist when selected. `0.38.1-rc.4` in source metadata is not equivalent to published RC4; [speech inventory](../reconciliation/speech-models.md) records observed latest published RC before Nemotron introduction.

[post-release-cleanup](../../../scripts/post-release-cleanup.ps1#L1-L16) defaults preview. With Apply it can **merge eligible green PRs**, close contained PRs, delete older prereleases/contained remote branches and remove rebuildable targets/worktrees; it is not harmless cache housekeeping. [PR match-head checks](../../../scripts/post-release-cleanup.ps1#L59-L124) and [exact ancestry branches/worktrees](../../../scripts/post-release-cleanup.ps1#L147-L240) protect against broad deletion, but executing still needs explicit release-scope owner authority, correct destination/state and native safety procedure. This task never runs it or opens/merges/releases a PR.

## Remaining checks

Exact-SHA targeted build/installer, installed-version parity, model availability/cancel, Slint screenshots/functional hotkeys, update URL/digest failure and signature policy, locked owned processes vs unrelated copies, uninstall/junction behavior and preview/apply audit remain unexecuted. Stable publication requires explicit owner authorization and platform release procedure; recorded source contracts or a docs-only green diff are not substitutes.
