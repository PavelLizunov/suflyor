# macOS Gate 0B capture prototype

This disposable app validates native macOS capture and TCC behavior without
using the production backend, AI, STT, persistence, BlackHole, or the existing
UAP STT service. It uses only Apple frameworks and the existing Slint UI stack.

The prototype provides explicit controls for:

- microphone permission plus default-input start/stop and frame counters;
- a private Core Audio Tap plus private aggregate device for system audio;
- a ScreenCaptureKit screenshot of this app's own window, retained only as
  dimensions and never written to disk;
- opening System Settings through the public workspace API, with the user then
  choosing Privacy & Security;
- explicit normal-exit teardown when HAL is responsive, with process cleanup
  as the fallback if macOS is still blocking inside a permission request.

Build and package on the Mac:

```zsh
cd ~/Developer/suflyor/experiments/macos-gate0b
./scripts/build-app.sh
open "target/Suflyor Gate 0B.app"
```

The targeted static/package gate is `./scripts/check.sh`. It does not trigger
TCC prompts or require repeating the full Windows test suite.

The bundle identifier is the approved future development identity
`com.ninitux.suflyor.dev`. The local package is ad-hoc signed because the owner
does not plan to purchase Apple Developer Program enrollment. Rebuilds may
invalidate remembered TCC decisions; this is recorded rather than hidden.

No permission prompt is triggered at launch. Every prompt follows a deliberate
button click. A real physical microphone is still required to close the
microphone-capture row; a virtual device is not accepted as a substitute.

For repeatable TCC lanes, `open ... --args --auto-system`, `--auto-screen`,
`--auto-microphone`, or `--auto-microphone-input` invokes the matching UI
action one second after the window opens. These flags exist only for sanitized
logging and repeatable checks.

Manual TCC rows are Allow, Deny, Deny -> System Settings -> Allow, Revoke,
RestartRequired, sleep/wake, and cleanup after forced termination. Raw captures
or account/machine identifiers must not be committed.
