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
  stationary, and ignore-cycle behavior. The mutually exclusive legacy
  fullscreen-auxiliary behavior is used only before macOS 26. AppKit readback
  on macOS 26 returned level 25 and collection behavior `0x40051`.
- Before the interaction check, the foreign fullscreen application was the
  frontmost process and both Suflyor windows were present in that fullscreen
  Space. Chips work after the Slint/Winit app is active.
- The tile moved by exactly +150 logical points horizontally and +100
  vertically after a physical title-area drag.
- Hiding the bar removes it from the on-screen window list. The native status
  item restores it in both an ordinary Space and the foreign fullscreen Space.
- With Stage Manager temporarily enabled, the bar and tile remained present
  beside the foreign fullscreen application and status-item recovery changed
  the on-screen Suflyor window count from three back to four. The original
  disabled Stage Manager setting was restored after the check.
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

- Only one physical display was connected, so cross-display movement remains
  unverified.
- The signed-in Apple ID is a free Personal Team. The Certificates portal is
  unavailable without paid Apple Developer Program enrollment, and the owner
  explicitly chose not to purchase that enrollment. No certificate was
  revoked. The strict-valid ad-hoc signature is the accepted local Gate 0
  development path; it does not provide Developer ID distribution or Apple
  notarization.
- External-capture exclusion is unsupported and the prototype says so in the
  UI; it makes no concealment claim.
- When another application is frontmost, the first click on the Slint/Winit
  floating window is still an activation click; the chip acts on the next
  click. Adding the nonactivating style to Winit's ordinary `NSWindow` had no
  effect. Reparenting its view into a real `NSPanel` was rejected because Winit
  aborted in its occlusion-state delegate. Production click-through therefore
  needs a backend-supported panel/window creation path, not a runtime reparent
  or global mouse-event workaround.
- Slint MCP can inspect and focus the normal Settings window, but its
  `take_screenshot` call for that window aborts the QA build inside the
  third-party `imgref` screenshot path (`stride > 0`). The packaged release
  application did not reproduce this QA-tool-only failure.

## Gate decision

The isolated native-windowing approach is technically viable for the tested
single-display, Spaces, fullscreen, Stage Manager visibility, focus, drag,
recovery, Retina, packaging, and local ad-hoc-signing surfaces. Gate 0A is
accepted as a local technical-feasibility result. It does not validate public
Developer ID/notarized distribution, backend-supported inactive-window
click-through, or cross-display behavior.
