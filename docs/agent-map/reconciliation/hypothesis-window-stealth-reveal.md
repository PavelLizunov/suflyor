# Original window C01/C03: bounded window reveal and stealth fallback evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_window_stealth_reveal_confirmed.py) inspect frozen window lifecycle, stealth application, and fallback realization logic. They do not invoke Win32 APIs or manipulate physical display servers. C01 and C03 remain confirmed mechanisms.

## C01 — `present_window_stealth_aware_at` unconditional reveal vs tray menu abort

In `window_lifecycle.rs`, [present_window_stealth_aware_at](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L291-L380>) attempts to apply stealth via `apply_stealth_one(hwnd, true)`. However, `do_reveal` unconditionally proceeds to move the window on-screen via `move_window_pos_only` and `set_platform_window_position`, regardless of whether `apply_stealth_one` succeeded.
In contrast, [open_tray_menu](<../../../slint-experiment/src/bin/overlay_host/bar_tray.rs#L249-L270>) explicitly checks `if global_stealth() && set_stealth(hwnd, true).is_err() { return false; }` and hides the window in its fallback if stealth application fails.

## C03 — bar fallback reveal under stealth vs aux window parking

In `bar_tray.rs`, [apply_overlay_hwnd](<../../../slint-experiment/src/bin/overlay_host/bar_tray.rs#L454-L476>) defines a last-ditch fallback for the main overlay bar when HWND grabbing repeatedly fails:
it disables active stealth (`set_global_stealth_effective(false)`, `o.set_stealth_active(false)`) and brings the bar on-screen at `(primary.left + 60, primary.top + 24)` even under stealth, to prevent the user from being locked out of the primary control surface.
In contrast, the auxiliary window fallback in [window_lifecycle.rs](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L350-L365>) checks `if global_stealth()` and deliberately keeps the window parked off-screen at `(-32000, -32000)` to prevent screen leakage.

## Limits

No physical display servers or screen recorders were run, and no Win32 display affinity failures were simulated live. Original statuses in `candidates.json` remain `confirmed`.
