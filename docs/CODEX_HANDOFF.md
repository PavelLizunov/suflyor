# Codex handoff — macOS port recovery

Updated 2026-08-20. Read `AGENTS.md` before acting. The section immediately
below is authoritative. The older P0n recovery narrative is retained after it
for provenance only and must not override the current state.

Operational note: Windows jobs follow
[`docs/winbrat-recovery.md`](winbrat-recovery.md). Never start a duplicate job
because SSH or a terminal disconnected. macOS builds and live QA run only on
`mm4`; Windows compilation and the repository gate run only on `winbrat`.

## Current operational state — 2026-08-20

- Worktree: `C:\Users\x3d_mutant\Natively\suflyor-macos-mlx-runtime`
- Branch: `codex/macos-mlx-runtime`
- Packaged code commit: `02ac938a46beb1c3fb335fda9f5310f92ef48d8b`
- Packaged code tree: `703ce482d60f90e836f663eac903ff12c7a49b53`
- The work remains local. No push, PR, merge, tag, notarization, or release was
  made.

The verified arm64 app is installed on mm4 at
`/Users/slovn/Applications/Suflyor.app`. The previous installed app was moved
to durable evidence before replacement. A cross-device archive for the
MacBook Air is available at
`C:\suflyor-test-evidence\macos-mlx-package-02ac938-20260820\Suflyor-02ac938-macos-arm64.zip`
(54,385,893 bytes, SHA-256
`1934588a878c20f62bfd93665ba1cc177634b94a935dd7d1150ab08f2bb54621`).
The Air was offline and was not contacted or modified.

### What is working now

- The owner confirmed managed GigaAM/CoreML is working well as the primary STT
  on mm4. The exact managed model remains outside the app bundle.
- Native MLX text uses pinned `LiquidAI/LFM2.5-8B-A1B-MLX-4bit`; native MLX
  Vision uses pinned `mlx-community/Qwen3.5-2B-4bit`. Both verified snapshots
  are installed under mm4 Application Support and are selected/downloaded from
  the macOS Settings UI. Merely choosing a catalog row does not silently change
  the provider; the user explicitly enables it.
- The exact packaged text sidecar passed READY, authentication, health/model
  identity, non-streaming generation, streaming generation, and terminal
  `[DONE]`. The exact packaged Vision sidecar passed the same flow with a valid
  deterministic 64x64 PNG.
- The package includes the required signed `mlx.metallib`, three Swift resource
  bundles, Swift runtime closure, three MLX license notices, Piper, TeraTTS,
  and the host. All four executables are thin arm64. Strict nested and deep app
  signature checks pass; signing is local ad-hoc, not Developer ID/notarized.
- The exact app was launched through LaunchServices as an `LSUIElement` and
  left running with its direct TeraTTS child. Configuration and secrets were
  not read or rewritten during package/live acceptance.

### Validation and operational limits

- Exact final Windows validation on Winbrat is green: focused packaging guard
  8/8 and full `scripts/ci.ps1` exit 0 for commit `02ac938a`.
- Full macOS compile-seam validation was green for parent `d82eea86`; `02ac938a`
  differs only by signing/verifying `mlx.metallib` and its structural guard.
  On `02ac938a`, the focused guard is 8/8 green and the real package build,
  independent artifact audit, packaged-model live smoke, and LaunchServices
  launch are green.
- Run macOS Cargo and Swift builds with one job. Unbounded Clippy previously
  spawned nine compilers, exhausted RAM/swap, and filled the disk. Regenerable
  debug targets were removed; models, evidence, installed apps, source, locks,
  and unrelated worktrees were preserved.
- Audible TTS quality/latency and every Settings page still require owner UI
  testing. Screen capture and menu-bar automation from SSH remain constrained
  by macOS TCC. Do not weaken TCC or treat process liveness as audible QA.
- A pre-existing launchd service `com.mlx.vlm` runs Python `mlx_vlm.server` on
  loopback port 8082 with the old `mlx-community/gemma-4-e4b-it-4bit` model. It
  was deliberately preserved. At the final audit it was idle and almost all of
  its roughly 5.5 GiB charged footprint was compressed/swapped, but stopping it
  can break the legacy external Vision route and launchd may restart it. Do not
  stop or disable it without explicit owner authorization.

## Historical P0n recovery state (superseded)

- Worktree: `C:\Users\x3d_mutant\Natively\suflyor-macos-port-recovery`
- Branch: `codex/macos-port-recovery`
- Code commit: `d85e1c245b909cb1a5afe1e36ba7ef247d8c1cc8`
- Parent/base: `ce6739ba303037780dd5b584748c15cc192dc744`
- Validated code tree: `ff8f822f73cb3ca66ed1e0f30d8385a21c7b76ae`
- The work is local. No push, PR, merge, tag, notarization, or release was made.
- The shared `overlay-mvp` checkout and its unrelated dirty worktrees were not
  cleaned, reset, or used for builds.

The immutable validation snapshot is synthetic commit
`4047d8433c070312e7da61041eee91d5601ddd5f` (P0n). Its tree is byte-for-byte
identical to code commit `d85e1c245`.

- Bundle:
  `C:\Users\x3d_mutant\Natively\worker-logs\macos-port-recovery-20260819\macos-p0n-final-20260819.bundle`
- Bundle SHA-256:
  `29e84468fb495b93ffea1580113f60eff5287fd27e92caee7c079b15d07ca6a4`
- Bundle length: `15,144,107` bytes
- Recovery archive:
  `C:\Users\x3d_mutant\Natively\suflyor-macos-recovery-20260819`

## What the branch audit established

The previous large macOS handoff was not a reliable description of this
branch. It was an uncommitted document from another dirty worktree and mixed
states from sibling branches.

- `codex/qwen-macos-capture-watchdog` at `8d4e722` is a separate 44-commit
  local-only line. It remains preserved on Windows and in the recovery
  artifacts.
- Most of its useful modules had already been copied into the current branch,
  but its entrypoint wiring was not. Restoring that entrypoint wholesale would
  regress the product: most hotkeys were stubs, Archive and Palette were empty,
  Settings was partial, and capture results were discarded.
- The mature shared runtime already contained the real Windows/macOS product
  flows. The repair therefore kept it and added the missing native macOS
  adapters.
- Recent branches, worktrees, stashes, reflogs, and unreachable commits were
  audited. No additional macOS production behavior was found outside the
  preserved Qwen line. Do not delete the Qwen worktree or recovery artifacts
  during routine cleanup.

## Canonical runtime — do not regress this again

`slint-experiment/src/bin/overlay_host.rs` is the thin entrypoint. On Windows
and macOS it explicitly includes the canonical root file
`slint-experiment/src/bin/overlay_host_windows.rs`. The filename is historical;
the file now owns the shared production runtime for both platforms.

The following seven Qwen slices were unreferenced and never compiled:

- `macos_session.rs`
- `macos_text_ask.rs`
- `macos_settings.rs`
- `macos_tile_manager.rs`
- `macos_palette.rs`
- `macos_help.rs`
- `macos_archive.rs`

They and the accidental duplicate
`src/bin/overlay_host/overlay_host_windows.rs` were deleted only after their
unreachability and preservation in backups were proven. Do not recreate a
second macOS runtime beside the shared one.

## Implemented recovery

### Native lifecycle and windows

- The macOS status-item guard now lives through the event loop instead of being
  dropped immediately.
- Hide/Show synchronizes the bar state and menu title; Quit shuts down the host
  and managed sidecars.
- AppKit view IDs, floating/raise behavior, window rectangles, logical-point
  positioning, and selected-monitor placement replace fake `HWND(1)` success.
- Screen-share stealth is honest: public macOS capture exclusion is reported as
  unsupported instead of claiming that a no-op succeeded.
- Bar, auxiliary windows, tiles, lock popup, and maximize/restore use the native
  geometry path. The broken unconditional main-screen centering was removed.

### Capture, OCR, clipboard, and hotkeys

- CoreGraphics enumerates active displays, cursor position, and global display
  bounds; ScreenCaptureKit captures the display under the cursor.
- Suflyor is excluded from its own F8 frozen frame. Capture fails closed if the
  app or its windows cannot be excluded.
- Apple Vision provides local macOS OCR off the UI thread, including exact BGRA
  validation, autorelease handling, automatic language detection, localized
  empty/error states, and an `OCR · Apple Vision` source label.
- macOS clipboard read/clear and trusted Command+C implement Shift+Alt+1 with
  bounded modifier release and text-clipboard restoration on failure.
- All 13 production shortcuts remain owned by the mature shared dispatcher.

### Sessions, STT, and platform policy

- Rapid manual Start/Stop and watchdog work share one lifecycle lock and exact
  intent generations, preventing stale tasks from reversing a newer action.
- The watchdog is intentionally fail-safe Stop-only. On a proven stalled or
  vanished capture it finalizes audio/journal/debrief once, preserves the full
  transcript, shows a local error, and asks the user to press Start. It does not
  silently create a new session or split the journal.
- UAP is another project. Its raw-PCM backend, setup fields, tests, and UI
  strings were removed from Suflyor. The earlier mistaken external loopback
  edit was fully rolled back and verified; this branch does not manage the UAP
  service or its configuration.
- macOS exposes Cloud (Groq Whisper) and External Whisper only. GigaAM and the
  broken all-in-one local installer are hidden on macOS. A manually supplied
  native `whisper-server` remains supported.
- The Windows-only updater surface is hidden on macOS until a signed macOS
  update artifact/channel exists.

### Packaging and gates

- The package requires `overlay-host`, `suflyor-tts`, and `suflyor-teratts`,
  uses one target directory, requires the icon, lints `Info.plist`, and signs
  nested executables before the app bundle.
- `LSUIElement=true`; bundle ID is `com.ninitux.suflyor.macos`; minimum macOS is
  14.2.
- The macOS gate now runs the portable crates, overlay-host tests, all 24 safe
  non-GUI macOS integration guards, Piper/WSOLA/TeraTTS checks, and strict
  Clippy. The two window-showing GUI tests remain live-QA-only.
- Windows large local-AI fixtures are marked sparse before `set_len`, preventing
  the full gate from consuming many gigabytes of physical disk.
- TeraTTS shared dependencies are in `[dependencies]`; only `cpal` remains
  non-Windows-specific. This is the final P0n repair that made Windows TeraTTS
  Clippy/tests compile.

## Exact verification

### macOS — mm4

Validated synthetic commit `4047d843`, tree `ff8f822f`:

- `cargo check --locked --bin overlay-host`: green
- full `scripts/git-gate-macos.sh`: green
- 38 test suites; 1,006 passed; 0 failed; 5 expected ignored
- Evidence:
  `/Users/slovn/Developer/suflyor-test-evidence/macos-p0n-final-20260819-1730`
- `cargo-check.log` SHA-256:
  `dccad65a474a0e64d7c677fc96c87f30265067841005a793f4d82605fea6fe77`
- `git-gate-macos.log` SHA-256:
  `ca509674c365d81a4566bf7bd9bf6c07351c807472656484187f89648cbf42c0`
- `manifest.txt` SHA-256:
  `3f1333169119fc9a7fa161f9611276b1a7b885b6bf7e131be00d4f1468cd045a`

### Windows — Winbrat

The same commit/tree passed one non-duplicated full `scripts/ci.ps1` run:

- final line: `All gating layers green.`
- Slint, UI-MCP, backend, WSOLA, Piper, TeraTTS, and i18n layers: green
- TeraTTS: strict Clippy green; tests 70/70
- Evidence:
  `C:\suflyor-test-evidence\macos-port-p0n-final\p0n-final-full-ci.log`
  with sibling `.exit.txt` and `.manifest.json`
- `p0n-final-full-ci.log` SHA-256:
  `3c7c532f82b2bfb13ab677752e5bed8160a86d8b861a802e2a4e27ea341c8e09`
- `p0n-final-full-ci.exit.txt` SHA-256:
  `13bf7b3039c63bf5a50491fa3cfd8eb4e699d1ba1436315aef9cbe5711530354`
- `p0n-final-full-ci.manifest.json` SHA-256:
  `1b2fdd7d57b11b25445a55ed97825fbf2c6e10d49e8a43c4805fd7b222208af1`

### Exact P0n app bundle

Built from the same P0n tree and launched through LaunchServices:

- App:
  `/Users/slovn/Developer/suflyor-macos-backend-seam/slint-experiment/target/Suflyor.app`
- App size: 88,040 KiB
- `overlay-host`: 35,600,512 bytes; SHA-256
  `2afbb3338b6756790081a88cdffc5d98d4103f1d3b7a2278498a558a1ec453c9`
- `suflyor-tts`: 26,456,048 bytes; SHA-256
  `75f34b134231e5c644f4cffe80825900d0875c9a472fdc740568aaf292d72acd`
- `suflyor-teratts`: 27,333,248 bytes; SHA-256
  `278850d515bd68e701341a08efbccd384a75fa333258d1f351fb58c07e464ecd`
- All three executables are thin arm64 and link only system frameworks/libraries.
- `Info.plist`, strict nested/deep signatures, and the audio-input entitlement
  were verified.
- LaunchServices reports `ApplicationType=UIElement`, registered and ready.
- At verification time after the exact relaunch, the host/child PIDs were
  `41971` / `41975`.
- Evidence:
  `/Users/slovn/Developer/suflyor-test-evidence/macos-p0n-package-20260819-1740`
- `build-macos-app.log` SHA-256:
  `9beb6a3dde7d93962cf99d312102ca46ca17f56b239da5c62316f26fcbf1e8fe`
- `package-verification.txt` SHA-256:
  `5de4b453c6b656a4b4ed1d64f75f9b9f046390b7555e803e960fcea366fbbc8c`
- `launchservices-full.txt` SHA-256:
  `eea095c494e3ff6cfe757b719028ae68be19cdf2f854743c0a9e3a654acd32d7`

This is an ad-hoc local signature. Distribution still requires the owner's
Developer ID, notarization, and explicit release authorization.

## Live QA completed

- The status item exposes Hide/Show and Quit. Hide removes the bar; Show
  restores it; F1 still works while the bar is hidden.
- F1 Help, F4 Palette, and F7 Archive open and close distinct floating windows.
- F6 on an empty runtime creates the expected local feedback tile. F3 on a
  fresh runtime creates no false request. Plain F8 with Vision off creates its
  safe local notice.
- A filtered direct launch reported successful registration of all 13 hotkeys,
  including Ctrl+F8 and Shift+Alt+1/2/3.
- Registration is not a functional pass for all 13 shortcuts. Live behavior was
  completed only for F1, fresh-state F3, F4, F6, F7, and plain F8; the remaining
  modified/capture shortcuts still require the physical/TCC checks below.
- Native Quit removed the host and its direct Tera child within the first
  two-second poll; no bundle process remained. Relaunch succeeded afterward.
- Direct TeraTTS protocol/synthesis reached `READY`, `STARTED`, `PLAYING`, and
  `DONE`, with no `FAILED` or `REJECTED` and 160,885 synthesized samples.
- Only one 1920x1200 display was connected during live QA.

## Human/TCC acceptance still required

Do not relabel these items as passed from static guards:

1. Visual screenshots were blocked from the SSH session by macOS display/TCC
   context. Confirm the bar, status item, absence of a Dock icon, English/Russian
   Settings, lock popup, and tile maximize/restore from the physical desktop.
2. With Suflyor Screen Recording and Accessibility permissions, physically test
   Ctrl+F8 and Shift+Alt+2 Apple Vision OCR on asymmetric English/Russian text,
   own-window exclusion, orientation, and clipboard-preserving Shift+Alt+1.
3. BlackHole capture from SSH produced a valid but fully silent WAV despite a
   successful Tera synthesis. Confirm Piper and Tera audibly, plus
   Shift+Alt+3 pause/resume, from the physical audio session.
4. Repeat display-under-cursor capture and placement with a real second display,
   including a negative-origin layout; no second display was attached here.
5. Exercise rapid Start/Stop with the intended microphone/system-audio TCC and
   local STT setup. The concurrency and watchdog state machines are covered by
   tests, but the device/TCC path needs real hardware acceptance.
6. Functionally exercise Shift+F8, F9, and Shift+F9 with an explicitly chosen
   offline/local test setup. They were not invoked here because the live config
   was not read or changed and an external/cloud request could incur cost.

## Product limits, not hidden TODOs

- Suflyor excludes itself from its own F8 capture. It cannot promise invisibility
  in Zoom/Meet/other screen-sharing applications through a public macOS API;
  the UI must report stealth unavailable rather than fake success.
- One-click managed local AI/STT is not offered on macOS until the project owns
  native Whisper artifacts and a portable installer. External Whisper and Groq
  are the supported macOS STT paths today.
- Capture watchdog recovery stops safely and preserves data; resuming is manual
  until recorder/journal continuation can be made transactional.
- The macOS updater is intentionally unavailable until a signed update channel
  exists.

## Next action

The code is committed and both platform gates are green. The next owner action
is the physical visual/audio/TCC checklist above. After that, push this
`codex/macos-port-recovery` branch and open a PR if desired. Do not push directly
to `master`, publish a release, create a tag, or notarize/publish artifacts
without explicit owner authorization.
