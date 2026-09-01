# Suflyor v0.38.1-rc.1

This release candidate focuses on faster, shorter, and better-grounded automatic answers from the managed MLX text model on Apple Silicon.

## Improvements

- Managed MLX automatic tiles use a compact prompt while preserving response language, approved context, live coaching, and prompt-injection boundaries.
- Relevant entries from the embedded knowledge base ground automatic answers, with focused coverage for Kubernetes readiness/liveness probes, Linux load average, and etcd snapshot recovery.
- The managed MLX automatic-tile output budget is reduced to 384 tokens. Non-MLX automatic tiles retain their 4096-token budget; manual asks and re-asks retain their existing separate budgets.
- Automatic-tile cache identity now includes the effective transcript, meeting context, coaching state, and MLX prompt profile, preventing stale answers after those inputs change.

## Installers

- Apple Silicon macOS 14.2 or newer: `Suflyor-0.38.1-rc.1-macos-arm64.dmg`.
- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned. The macOS package is ad-hoc signed and unnotarized; follow the installation and permissions guide included in the DMG.

## Verification

The changed prompt, cache, token-policy, and Swift tokenizer seams are covered by focused tests. The prerelease also includes a short MacBook retest checklist for the affected managed-MLX automatic-answer scenarios.

This is a prerelease test build for MacBook acceptance before a stable maintenance release.
