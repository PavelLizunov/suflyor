# O5: Native Windows HWND, Stealth & Multi-Monitor Geometry

## 1. Windows HWND Manipulation (`slint-experiment/src/win32.rs` & `src/native/`)
- **Two-Step HWND Extraction**: Winit and Slint lazily instantiate native Win32 window handles. Calling `window_handle()` immediately upon window instantiation yields `HandleError::NotSupported`. Suflyor enforces a retry-based schedule via `realize_with_retries` (evaluating at ~33ms, +80ms, and +400ms) to ensure window realization.
- **Transparency & DWM Frame Extension**:
  - `make_transparent_overlay`: Sets `WS_EX_TRANSPARENT` for click-through behavior on the main bar overlay.
  - `make_transparent_tile`: Clears `WS_EX_TRANSPARENT` so user interactions (buttons, dragging) work as intended.
  - DWM blur-behind is activated with an empty region (`MARGINS { all: -1 }`), hinting the Windows DWM compositor to enable per-pixel alpha blending.
  - Ghost caption buttons (`WS_SYSMENU`, `WS_MAXIMIZEBOX`, `WS_MINIMIZEBOX`) are stripped from `GWL_STYLE` to prevent faint gray artifacts on frameless windows.
- **Taskbar Skipping**: `WS_EX_TOOLWINDOW` is set while `WS_EX_APPWINDOW` is cleared to prevent floating overlays from polluting the Windows taskbar.

## 2. Stealth Mode (`WDA_EXCLUDEFROMCAPTURE`)
- **Display Affinity**: Invokes `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` across all application surfaces (bar, tiles, palette, settings).
- **Readback Verification**: Every affinity change reads back the setting via `GetWindowDisplayAffinity(hwnd)`. If readback does not match, a stealth fault is raised and displayed in the UI.
- **Cursor Guard Subclassing**: Win32 window subclassing intercepts `WM_SETCURSOR` to force the arrow cursor, preventing the system cursor from blinking or revealing window positions during screen-sharing sessions.
- **GDI BitBlt Caveat**: GDI `BitBlt` ignores `WDA_EXCLUDEFROMCAPTURE`. Before internal screen captures, Suflyor explicitly hides its own windows (`SW_HIDE`), flushes the compositor via `DwmFlush()`, grabs the frame, and restores visibility.

## 3. Multi-Monitor Coordinate Math & Geometry
- **Monitor Orientation Awareness**: The user environment typically features a primary landscape display (1920x1080 at `x=0`) and a secondary portrait display (1200x1920 at `x=-1200`).
- **`pick_monitor` Policy**: Suflyor defaults tile placement to the primary monitor unless a non-primary monitor is both **landscape and wider than or equal to** the primary monitor width. This prevents tiles from landing invisibly on portrait screens.
- **Virtual Desktop Origin**: Coordinate calculations account for negative coordinate spaces (`SM_XVIRTUALSCREEN`) rather than assuming `(0, 0)`.
- **First-Frame Flicker Prevention**: Windows are parked off-screen at `(-32000, -32000)` before `.show()`, decorated with DWM properties, and only then moved into their final viewport position.

## 4. macOS Platform Seams (`slint-experiment/macos/` & `src/native/macos/`)
- **Native AppKit Bridge**: Objective-C wrappers (`libsuflyor_appkit.a`) manage window levels (`NSPopUpMenuWindowLevel`) and activation policies (`NSApplicationActivationPolicyAccessory`).
- **Coordinate Inversion**: Flips AppKit's bottom-left origin to top-down screen coordinates via `CGRectGetMaxY(primary) - NSMaxY(frame)`.
- **Honest Unsupported Stealth**: macOS has no OS-wide window exclusion API equivalent to `WDA_EXCLUDEFROMCAPTURE`. Consequently, `stealth_supported()` returns `false`, and `set_stealth()` fails with `Err(Unsupported)`. Self-exclusion is instead performed via ScreenCaptureKit's `SCContentFilter` during internal captures.
