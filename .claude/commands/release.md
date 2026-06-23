---
description: Build + self-gate a release, SHOW the evidence, then STOP — never auto-publish (waits for an explicit «релизь»).
---

Cut a release of suflyor. This is a HARD-STOP checklist: do every step, present
the evidence, and WAIT. Do NOT run `gh release` until the user explicitly says
«релизь». A green gate ≠ a verified UI.

## 1. Version (two files, must match)
Pick the new SemVer (ask if unclear) and bump it in BOTH:
- `slint-experiment/Cargo.toml` → `version = "X.Y.Z"`
- `scripts/slint-installer.nsi` → `!define PRODUCT_VERSION "X.Y.Z"`

A mismatch ships a binary whose version ≠ the installer's — the auto-updater
will misbehave.

## 2. Gate — all five layers, ALL THREE crates
- clippy `-D warnings` + `cargo fmt --check` + full `cargo test` for
  `overlay-backend`, `slint-experiment`, AND `suflyor-tts`.
- Adversarial review-agent on the diff (0 critical / 0 important to ship).
- Live smoke: build release, launch `overlay-host.exe`, read the boot log
  (hotkeys registered + DWM on), eyeball the changed UI surface.

## 3. Build the installer
- `pwsh scripts/build-slint-release.ps1 -Installer`
- Confirm `slint-experiment/target/release/bundle/suflyor-slint-setup.exe`
  exists; note its size + that the binary reports the new version.

## 4. SHOW the user — do NOT publish
Present version, gate results, review verdict, smoke result, installer size, as
EVIDENCE ("here's what passed, look at X") — never "all green, releasing".
**STOP. Wait for «релизь».**

## 5. Only after «релизь»
- Commit, EXCLUDING the user's WIP: `.claude/hooks/git-gate.ps1`,
  `scripts/ci.ps1`, `docs/feature-requests.md` (and never `nini-context-backup.txt`).
- `git push` + `gh release create vX.Y.Z <installer> --title … --notes-file …`
  via **PowerShell** (the Bash tool's network to GitHub is flaky — retry on reset).
- Verify: tag present, not draft/prerelease, asset attached, marked Latest.

Commit footer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`
