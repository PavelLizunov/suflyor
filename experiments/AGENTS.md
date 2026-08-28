# experiments — macOS Feasibility Experiments Guide

## Non-Production Status

- All projects under `experiments/` (`macos-gate0a`, `macos-gate0b`, etc.) are **disposable feasibility spikes and prototypes**.
- **Not Production Code**: Code in `experiments/` is not part of Suflyor's production codebase and is not bundled into release binaries (`overlay-host`, `overlay-backend`, `suflyor-tts`, `suflyor-teratts`, `suflyor-wsola`).
- **No Production Dependencies**: Production crates must never import or depend on crates inside `experiments/`.

## Platform Routing & Isolation

- **Production Platforms**: Windows is the primary product target and the main crates now contain an active macOS port. Linux is unsupported.
- **Historical Experiment Scope**: These spikes established isolated macOS capabilities on Apple Silicon hardware before selected patterns moved into production. Do not treat them as the current macOS runtime.
- **Isolation Rules**:
  - Code under `experiments/` stays isolated within `experiments/` and does not alter Windows release builds, CI gates, or production audio/AI pipelines.
  - macOS experiments maintain `cfg` conditional compilation checks so `cargo check` and static analysis can run on Windows without requiring macOS SDKs.

## Current macOS Experiments

1. **`experiments/macos-gate0a/`** — AppKit Windowing & Slint Surface Prototype
   - Validates floating window behavior across ordinary windows, Spaces, and foreign fullscreen applications using AppKit and Slint.
   - Includes status bar menu item, draggable tiles, settings window focus, and Retina scale factor logging.
   - Uses bundle ID `com.ninitux.suflyor.dev` with local ad-hoc code signing.
   - Note: External screen capture exclusion is explicitly unsupported on macOS.

2. **`experiments/macos-gate0b/`** — Native Capture & TCC Permission Prototype
   - Validates macOS permission flows (TCC) and audio/video capture without production backend dependencies.
   - Exercises microphone input via AVFoundation, system audio tap via Core Audio Taps and private aggregate device, and single-window capture via ScreenCaptureKit.
   - Uses public workspace APIs for System Settings navigation and handles HAL permission timeouts cleanly without deadlocks.

## Promotion Criteria

For any experimental capability or pattern to be promoted from `experiments/` into production architecture:

1. **Formal Architecture Review (SDD)**: Must undergo research, Micro-Spec review, and explicit approval before any platform abstractions or macOS code enter main crates.
2. **Hardware Acceptance Evidence**: Must pass all manual acceptance rows on real Apple Silicon Mac hardware, including physical microphone audio capture, sleep/wake stream recovery, and multi-display windowing.
3. **Zero Impact on Windows Target**: Must maintain 100% feature and performance parity on Windows with zero cross-platform regressions or build bloat.
4. **Clean Abstraction Boundary**: Platform-specific logic must be isolated behind modular platform interfaces (`target_os` routing) rather than scattering `#[cfg]` across business logic.
5. **Security & Entitlements Compliance**: TCC permission handling, hardened runtime entitlements, and process teardown must comply with platform distribution policies and release safety rules.
6. **Passing Quality Gates**: Run the experiment's own scripts (for example `experiments/macos-gate0b/scripts/check.sh`) and the applicable macOS repository gate on `mac-worker`; do not invent a root `scripts/check.sh`.
