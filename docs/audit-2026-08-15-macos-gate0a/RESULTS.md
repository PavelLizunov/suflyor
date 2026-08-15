# macOS Gate 0A results — 2026-08-15

Scope: isolated Slint/AppKit feasibility prototype only. No production AI,
STT, audio, persistence, updater, secrets, Tailscale routing, or the existing
UAP STT process was changed.

## Passed on the real Apple Silicon Mac

- Xcode 26.6 and the macOS 26.5 SDK compile Objective-C and Metal-backed Slint.
- The release binary packages as `Suflyor Gate 0A.app` with bundle identifier
  `com.ninitux.suflyor.dev`.
- Strict code-signature verification passes with an ad-hoc signature.
- The floating bar and tile use AppKit status-window level and remain visible
  beside an independently launched foreign fullscreen application.
- On macOS 26, `CanJoinAllApplications` is combined with all-Spaces,
  fullscreen-auxiliary, stationary, and ignore-cycle behavior. AppKit readback
  returned level 25 and collection behavior `0x40151`.
- Before the interaction check, the foreign fullscreen application was the
  frontmost process and both Suflyor windows were present in that fullscreen
  Space. One physical pointer press switched the Test chip and its status text;
  no activation click was lost.
- The tile moved by exactly +150 logical points horizontally and +100
  vertically after a physical title-area drag.
- Hiding the bar removes it from the on-screen window list. The native status
  item restores it in both an ordinary Space and the foreign fullscreen Space.
- Settings opens as a normal AppKit window. Its text field accepted keyboard
  focus and the synthetic value `Focus accepted on macOS`.
- Slint MCP reported one bar, one tile, and one Settings window; no duplicate
  bar registration remained. The bar and tile screenshots at Retina 2x had no
  clipping or overlap.
- Display logging reported one 1280x800 logical display at scale 2.0 and did
  not include display identifiers.
- The existing UAP STT server remained a single unchanged process after the
  run.

## Pending or explicitly unsupported

- Stage Manager was disabled, so its live scenario was not executed.
- Only one physical display was connected, so cross-display movement remains
  unverified.
- Xcode account sign-in is complete, but Apple Development signing remains
  blocked by the remote certificate/pending-request state. No remote
  certificate was revoked. Ad-hoc signing proves packaging only.
- External-capture exclusion is unsupported and the prototype says so in the
  UI; it makes no concealment claim.
- Slint MCP can inspect and focus the normal Settings window, but its
  `take_screenshot` call for that window aborts the QA build inside the
  third-party `imgref` screenshot path (`stride > 0`). The packaged release
  application did not reproduce this QA-tool-only failure.

## Gate decision

The isolated native-windowing approach is technically viable for the tested
single-display, Spaces, fullscreen, pointer, focus, drag, recovery, Retina,
packaging, and ad-hoc-signing surfaces. Gate 0A is not fully closed until Apple
Development signing is available and the applicable Stage Manager and
second-display checks are recorded.
