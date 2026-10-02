# Original window C02/C04: bounded stealth parking and registry walk evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_window_stealth_lifecycle_hypotheses.py) inspect frozen window presentation, off-screen parking, and stealth registry dispatch logic. They do not invoke Win32 APIs, create windows, or modify display affinity. C02 and C04 remain hypotheses.

## C02 — off-screen parking before show and stealth application timing

In `window_lifecycle.rs`, [present_window_stealth_aware_at](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L291-L308>) calls `set_platform_window_position(win.window(), -32000, -32000)` before `win.show()` to park the window off-screen prior to initial frame rendering.
In [do_reveal](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L315-L350>), `grab_hwnd` realizes the handle, applies window decorations, and calls `apply_stealth_one(hwnd, true)` when `global_stealth()` is true **before** moving the window on-screen via `move_window_pos_only`.
In [fallback_reveal](<../../../overlay-backend/src/credentials.rs#L1-L10>), if HWND realization fails completely after retries while stealth is active, the function logs a diagnostic and returns early, keeping the window safely parked off-screen rather than revealing it unexcluded.

## C04 — WindowRegistry stealth iteration scope and tray menu handling

In [apply_stealth](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L538-L565>), the registry walks open tiles, settings, palette, text ask, wizard, help, recovery dialog, transcript, archive, and lock menu windows.
It does not hold or walk references to `TrayMenuWindow` or the full-screen capture overlay.
In `bar_tray.rs`, [open_tray_menu](<../../../slint-experiment/src/bin/overlay_host/bar_tray.rs#L196-L260>) parks the tray menu window at `(-32000, -32000)`, calls `show()`, and applies `set_stealth(hwnd, true)` inside the realization closure before repositioning to the target coordinates `(x, y)`.

## Limits

No native display capture was run, no DWM window creation races were triggered, and no HWND parking glitches were observed on live monitors. Original statuses in `candidates.json` remain `hypothesis`.
