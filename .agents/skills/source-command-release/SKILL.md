---
name: source-command-release
description: Build, verify, publish, and clean up Suflyor RC or stable releases under the repository-wide release policy.
---

# Suflyor release

Use this repository instruction whenever any agent prepares or publishes a
Suflyor release. `AGENTS.md` remains authoritative.

## 1. Version and gate

Update both `slint-experiment/Cargo.toml` and
`scripts/slint-installer.nsi` (`PRODUCT_VERSION`) to the same SemVer. RCs use
`X.Y.Z-rc.N`.

Run `powershell scripts/git-gate-native.ps1 manual`. Use the selected tier for
an RC; a version-only bump does not force Full. Stable releases always require
`powershell scripts/ci.ps1`. Build/test on Winbrat or required GitHub CI, never
on the owner's workstation when it is not an authorized test host.

For changed UI, complete `.agents/skills/slint-mcp-ui-audit/SKILL.md`. Build the
installer with `powershell scripts/build-slint-release.ps1 -Installer`, and
record its SHA-256, size, embedded version, install smoke, and required runtime
evidence.

## 2. Publish policy

- **RC prerelease:** standing owner authorization applies. When its selected
  gate and evidence are green, publish the next RC without asking again.
  Verify the exact tag, prerelease flag, and installer asset; do not mark it
  Latest.
- **Stable release:** show the evidence and stop until the owner explicitly
  authorizes that stable version with `релизь`. Never infer stable approval
  from RC authorization.

Commit only intended paths on a `codex/<task>` branch, use a PR, and never push
a release directly from `master`.

## 3. Mandatory finish hook

Publishing is not complete until both commands succeed:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/post-release-cleanup.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/post-release-cleanup.ps1 -Apply
```

Verify afterward that no actionable PR remains, merged remote branches are
gone, only the newest published prerelease is retained, dirty/unproven work remains
untouched, and inactive rebuildable targets were removed. GitHub repository
setting `delete_branch_on_merge` must remain enabled. Include the cleanup
result and every deliberately preserved item in the release summary.
