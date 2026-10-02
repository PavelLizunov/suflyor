# Original window C08/C09: bounded DWM transparency and style persistence evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_window_dwm_transparency_hypotheses.py) inspect frozen Win32 transparency wiring, extended styles, and topmost toggling logic. They do not invoke Win32 APIs, create DWM windows, or manipulate screen rendering. C08 and C09 remain hypotheses.

## C08 — DWM frame extension, blur-behind composition, and caption button removal

In `win32.rs`, [apply_transparency](<../../../slint-experiment/src/win32.rs#L196-L280>) wires per-pixel alpha transparency for Windows DWM:
1. Calls `set_skip_taskbar(hwnd, true)` to ensure `WS_EX_TOOLWINDOW` is set and `WS_EX_APPWINDOW` is cleared;
2. Sets extended style using `transparency_exstyle`: setting `WS_EX_TRANSPARENT` for the click-through overlay bar (`make_transparent_overlay`) and clearing it for interactive tiles (`make_transparent_tile`);
3. Clears caption button bits (`WS_SYSMENU`, `WS_MAXIMIZEBOX`, `WS_MINIMIZEBOX`) from `GWL_STYLE` and calls `SetWindowPos` with `SWP_FRAMECHANGED` to eliminate ghost min/max/close glyphs in extended DWM frames;
4. Extends the DWM frame across the entire client area via `DwmExtendFrameIntoClientArea(hwnd, margins=[-1,-1,-1,-1])`;
5. Enables DWM blur-behind with an empty bounding region `CreateRectRgn(0, 0, -1, -1)` to activate per-pixel alpha composition without calling `SetLayeredWindowAttributes`.

## C09 — Topmost Z-order toggle and style persistence coherence

In `win32.rs`, [set_always_on_top](<../../../slint-experiment/src/win32.rs#L301-L315>) executes `SetWindowPos` with `HWND_TOPMOST` or `HWND_NOTOPMOST` using flags `SWP_NOMOVE | SWP_NOSIZE`.
While `set_skip_taskbar` re-asserts `HWND_TOPMOST` when toggling taskbar presence, it does not re-apply `apply_transparency`. Extended styles and DWM frame extension are applied on initial realization (`apply_overlay_hwnd` or `present_window_stealth_aware_at`) and rely on Windows DWM to maintain client frame attributes across Z-order changes.

## Limits

No native Windows DWM rendering glitches or transparency regressions were observed live, no Winit event-loop restyle races were triggered, and no compositor hooks were installed. Original statuses in `candidates.json` remain `hypothesis`.
