# macOS Gate 0B results — 2026-08-15

Scope: isolated Slint plus native Apple-framework capture prototype. The
Windows product crates, production audio routing, network configuration,
BlackHole, and UAP STT were not modified.

Provenance: stable Windows v0.37 commit
`2470f1988e9e333796237a83f58eaf9a7fc6b9c7`; Gate 0B started from
`22af906807d5daf23aab364ff6cc3afcfa5c7e9b` on branch
`codex/macos-gate-0b`.

## Implemented and verified

- A standalone `experiments/macos-gate0b` crate builds on Windows for cfg
  coverage and on the real Apple Silicon Mac against Xcode 26.6/macOS 26.5.
- Mac targeted checks pass against the locked dependency graph: Rust
  formatting, Clippy with warnings denied, property-list validation, optimized
  app packaging, and strict code-signature verification.
- `Suflyor Gate 0B.app` uses the approved intended development bundle ID
  `com.ninitux.suflyor.dev`, an ad-hoc hardened-runtime signature, the audio
  input entitlement, and both required usage descriptions.
- The app opens as a normal `820x732` AppKit window. Launch itself triggers no
  permission prompt; each request follows a deliberate UI action or explicit
  automation argument.
- Stale Gate 0A permission records for this disposable development identity
  were reset before the final lane. The unchanged Gate 0B app then exercised
  request, denial/settings recovery, explicit Allow, and restart-required
  behavior under its own displayed name.
- Microphone permission uses the public AVFoundation status/request API. On
  this Mac the request reached both the waiting-for-user-decision and allowed
  states without a crash. The available default input then started through
  `AVAudioEngine` on three runs and delivered at least 187,200 frames per run;
  frame counting, peak reporting, and explicit stop were exercised.
- Core Audio Tap creation succeeds through tap ID, tap UID, a private aggregate
  device, and a stereo float format with 8 bytes per frame. The original sound
  is never muted and no virtual audio driver or route change is used.
- While system-audio consent was pending, `AudioDeviceStart` returned
  `MACH_RCV_TIMED_OUT` (`0x10004003`). The app treated that observed timeout as
  a permission/restart action instead of crashing.
- After consent, the same Core Audio Tap path started successfully on three
  consecutive runs. Its sanitized smoke counter advanced beyond 1.7 million
  audio frames on the first final run and beyond 189,000 on each restart.
- Duplicate system-audio starts are rejected while TCC is pending. Quitting
  during that pending HAL call now exits promptly rather than deadlocking.
- A post-grant forced-termination recovery check was exercised on the final
  app. The next run recreated the process-scoped private tap and aggregate,
  started, and passed 189,000 frames, providing evidence that no persistent HAL
  object blocked recovery.
- ScreenCaptureKit uses a one-shot in-memory capture filtered to this app's own
  window. Denial, Settings grant, required restart, and successful capture were
  exercised; the final image was `1640x1464` and was never written to disk.
- Each settings action opens System Settings through the public workspace API
  and directs the user to Privacy & Security. Earlier diagnostic-only deep
  links opened all three target panes while investigating the local TCC state;
  those undocumented anchors are not part of the committed prototype.
- The existing UAP STT server remained one unchanged process after the runs.
- Normal Command-Q exits logged resource release and left no Gate 0B process.
  Quitting while HAL was waiting for permission also exited without a deadlock.

## Pending manual or hardware acceptance

- Acoustic verification with a known physical microphone. The available
  default input delivered frames but remained silent during the automated lane,
  and device identity is intentionally not recorded.
- Sleep/wake recovery while each granted stream is running.
- Grant persistence after ad-hoc rebuilds is recorded as unreliable platform
  behavior and is not a blocker for this free local gate.

## Gate decision

Gate 0B implementation, TCC transitions, capture paths, restart behavior,
repeat start/stop, normal cleanup, and post-crash recovery are proven on the
real Mac. The only remaining acceptance rows are a known physical-microphone
signal and sleep/wake recovery. They are local/manual checks, not paid-service
blockers.
