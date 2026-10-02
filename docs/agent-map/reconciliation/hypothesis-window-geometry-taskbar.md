# Original window C05/C06: bounded taskbar style transitions and monitor centering evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_window_geometry_taskbar_hypotheses.py) inspect frozen Win32 window style manipulation and overlay bar placement logic. They do not invoke Win32 APIs, reconfigure display monitors, or modify taskbar visibility. C05 and C06 remain hypotheses.

## C05 — `set_skip_taskbar` style transitions and un-hiding behavior

In `win32.rs`, [set_skip_taskbar](<../../../slint-experiment/src/win32.rs#L389-L436>) modifies extended window styles (`WS_EX_TOOLWINDOW` and `WS_EX_APPWINDOW`).
Because these styles only take effect when a window is shown, if the target style differs from the current style, the function executes:
1. `ShowWindow(hwnd, SW_HIDE)`;
2. applies the new extended style via `SetWindowLongPtrW`;
3. executes `ShowWindow(hwnd, SW_SHOWNOACTIVATE)` and calls `SetWindowPos` with `HWND_TOPMOST`.

In `window_lifecycle.rs`, [apply_bar_stealth](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L122-L179>) invokes `set_skip_taskbar(hwnd, effective)` directly without checking whether the bar was hidden to tray via `BAR_TRAY_HIDDEN`.

## C06 — initial overlay bar centering on unmeasured rects

In `bar_tray.rs`, [apply_overlay_hwnd](<../../../slint-experiment/src/bin/overlay_host/bar_tray.rs#L371-L430>) centers the bar on the primary monitor using `get_window_rect(hwnd).map(|(_, _, w, _)| w).unwrap_or(0)`.
If the window has not completed its initial layout and reports width `0`, the horizontal position evaluates to `p.left + (p.width() - 0) / 2 = p.left + p.width() / 2`, placing the left edge of the bar at the monitor midpoint instead of centering it.
In contrast, [recenter_when_sized](<../../../slint-experiment/src/bin/overlay_host/bar_tray.rs#L142-L168>) implements an explicit retry loop (up to 12 attempts) waiting until `(cur_w - target_width).abs() <= 24` before calculating and committing the final position.

## Limits

No native Win32 window styling was executed, no multi-monitor configurations were altered, and no taskbar flashing was measured. Original statuses in `candidates.json` remain `hypothesis`.
