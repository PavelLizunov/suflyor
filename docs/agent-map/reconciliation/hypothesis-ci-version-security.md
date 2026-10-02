# Original CI C12/C13: bounded security and version evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_ci_version_security_hypotheses.py) inspect frozen workflow, installer, and build source. They do not run Actions, Cargo, or NSIS. C12 remains a hypothesis; C13 remains confirmed.

## C12 — security jobs do not gate the required check

[Gate dependencies](<../../../.github/workflows/ci.yml#L146-L167>) include `changes` and `rust`, not macOS, gitleaks, cargo-deny, or CodeQL. [Security workflow](<../../../.github/workflows/security.yml#L1-L40>) exists separately and contains the scan jobs. This source split does not prove branch protection ignores that workflow.

## C13 — versions match, but build does not synchronize them

Current [Cargo version](<../../../slint-experiment/Cargo.toml#L1-L10>) and [installer version](<../../../scripts/slint-installer.nsi#L10-L23>) are both `0.38.1-rc.4`. [Release build](<../../../scripts/build-slint-release.ps1#L130-L160>) invokes makensis without `/DPRODUCT_VERSION` or an Info.plist read. [Native gate](<../../../scripts/git-gate-native.ps1#L69-L170>) selects crate manifests and runs `version_guard` through the Slint manifest; the NSIS path itself is not a crate selector.

The current match is therefore a snapshot, not an enforced synchronization mechanism. No installer or Cargo test was executed.

## Limits

No GitHub Actions, branch-protection query, Cargo test, or NSIS build was run. Original statuses and 39/75/5 remain unchanged.
