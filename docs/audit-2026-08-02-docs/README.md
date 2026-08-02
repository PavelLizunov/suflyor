# Documentation audit — 2026-08-02

Scope: public README, showcase images, GitHub About/topics, contributor and
security guidance, and post-release documentation maintenance.

## Confirmed and corrected

- Removed the stale claim that the overlay bar displays cost.
- Replaced the unverified “7–8+ hours” summary threshold with the implemented
  map-reduce behaviour.
- Replaced drifting knowledge-base counts with non-numeric wording.
- Corrected `Settings → About` to the existing `Settings → Updates` page.
- Documented the updater's fail-closed SHA-256 release-digest verification and
  the lack of Authenticode signing.
- Added supported-version and disclosure expectations to `SECURITY.md`.
- Added Slint MCP, before/after, branch, gate, and sensitive-data requirements
  to `CONTRIBUTING.md`.
- Added Linux and macOS as roadmap-only platforms; Windows remains the only
  supported platform.

## Visual evidence

- Before: `docs/showcase/settings-interface.png` showed language/theme controls
  but no speech model.
- After: [`settings-stt-local.png`](../showcase/settings-stt-local.png) shows
  `Local Whisper (whisper.cpp)` and `whisper-large-v3-turbo`. Only a loopback
  URL is visible; keys, private paths, and conversation data are absent.

The exact debug binary was built from master commit `f3dad30` with Slint debug
info and captured through the embedded Slint MCP server at 720×600.

## Automation

`.github/workflows/docs-after-release.yml` runs after a published release,
weekly, or on demand. It deterministically updates the README release marker on
an automation branch and requests review through a PR. If repository policy
blocks Actions-created PRs, it opens an issue with the branch comparison link.
It never pushes directly to `master` and never generates feature prose.

## External state

- GitHub About is English.
- Fifteen repository topics describe current supported functionality; Linux and
  macOS are intentionally not topics because they are roadmap-only.
- Homepage remains empty because there is no separate public product site.
- Slint Showcase submission is outside this docs correction and still requires
  completion of the external form with owner contact/legal details.

Large Qwen JSONL, prompts, MCP logs, and before/after artifacts remain outside
the repository under `ai-worker-results`.
