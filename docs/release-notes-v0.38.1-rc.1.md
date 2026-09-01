# Suflyor v0.38.1-rc.1

This release candidate focuses on faster, shorter, and better-grounded automatic answers from the managed MLX text model on Apple Silicon.

## Improvements

- Managed MLX automatic tiles use a compact prompt while preserving response language, approved context, live coaching, and prompt-injection boundaries.
- Relevant entries from the embedded knowledge base ground automatic answers, with focused coverage for Kubernetes readiness/liveness probes, Linux load average, and etcd snapshot recovery.
- The managed MLX automatic-tile output budget is reduced to 384 tokens. Manual asks, re-asks, non-MLX providers, and their 4096-token budget are unchanged.
- Automatic-tile cache identity now includes the effective transcript, meeting context, coaching state, and MLX prompt profile, preventing stale answers after those inputs change.

## Installers

- Apple Silicon macOS 14.2 or newer: `Suflyor-0.38.1-rc.1-macos-arm64.dmg`.
- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned. The macOS package is ad-hoc signed and unnotarized; follow the installation and permissions guide included in the DMG.

## Verification

The exact production prompt renderer and managed Swift/Metal sidecar were exercised with synthetic Kubernetes, Linux, and etcd questions. The accepted matrix completed all three answers with `finish_reason=stop`, no visible reasoning tags, factual grounding in all three cases, average first-token latency of 0.865 seconds, and average total latency of 1.804 seconds.

This is a prerelease test build for MacBook acceptance before a stable maintenance release.
