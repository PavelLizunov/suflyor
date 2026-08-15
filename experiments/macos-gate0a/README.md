# macOS Gate 0A prototype

This is a disposable feasibility spike, not the production macOS port. It has
no AI, STT, audio, persistence, updater, secrets, or dependency on the existing
UAP STT service.

It exercises only the first windowing risks:

- a Slint surface packaged as `com.ninitux.suflyor.dev`;
- AppKit floating-window behavior across ordinary windows, Spaces, and a
  foreign fullscreen app;
- interactive chips and a draggable tile;
- a normal Settings window with keyboard focus;
- recovery after hiding the overlay through a native status item;
- Retina/display-coordinate logging without machine identifiers;
- an explicit statement that external capture exclusion is unsupported.

Build and package on the Mac:

```zsh
cd ~/Developer/suflyor/experiments/macos-gate0a
./scripts/build-app.sh
open "target/Suflyor Gate 0A.app"
```

The script uses ad-hoc signing by default. Once a valid Apple Development
identity exists, set `SIGN_IDENTITY` to that identity and rebuild. Ad-hoc
signing can prove technical feasibility but cannot close the development-signing
acceptance item.

For the embedded Slint MCP visual audit, build the debug binary with
`SLINT_EMIT_DEBUG_INFO=1 cargo build --features ui-mcp`, then launch that exact
binary with `SLINT_EMIT_DEBUG_INFO=1` and `SLINT_MCP_PORT=9123`.

Manual acceptance matrix:

1. The bar and tile stay above an ordinary app and remain clickable.
2. Dragging each title area moves the corresponding window.
3. The Settings chip opens a normal window; the text field accepts typing.
4. Hide the overlay, then restore it from the Suflyor status item.
5. Repeat on another Space and beside a foreign fullscreen app.
6. Record Stage Manager behavior when available.
7. Compare the logged logical frames and scale factors with System Settings.
8. Confirm no UI or documentation claims external capture exclusion.

Gate 0A remains incomplete until the development-signed `.app` and every
applicable manual row above have sanitized evidence from the real Mac.
