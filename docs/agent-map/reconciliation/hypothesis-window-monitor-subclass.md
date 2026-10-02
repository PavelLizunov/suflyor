# Original window C07/C10: bounded monitor picking and subclass unhook evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_window_monitor_subclass_hypotheses.py) inspect frozen Win32 monitor selection, tile monitor pin encoding, and window subclassing logic. They do not query physical display hardware, install Win32 subclasses, or destroy live windows. C07 and C10 remain hypotheses.

## C07 — monitor selection fallback and monitor pin sentinel encoding

In `win32.rs`, [pick_monitor](<../../../slint-experiment/src/win32.rs#L866-L885>) requires an explicit primary monitor (`monitors.iter().find(|m| m.is_primary).copied()?`). If no primary monitor is reported by the system, it returns `None`.
In `window_lifecycle.rs`, [TILE_MONITOR_PIN](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L190-L215>) packs coordinates into an `AtomicI64` using `(left as i64 << 32) | (top as u32)`.
Because `TILE_MONITOR_AUTO` is defined as `i64::MIN`, when `left == -2147483648` (`i32::MIN`) and `top == 0`, `(i32::MIN << 32) | 0` evaluates to `i64::MIN`, colliding with the `AUTO` sentinel value.

## C10 — native window subclassing and registry unhook boundaries

In `win32.rs`, [install_stealth_cursor_guard](<../../../slint-experiment/src/win32.rs#L98-L115>) invokes Win32 `SetWindowSubclass(hwnd, Some(stealth_cursor_subclass_proc), ...)` to monitor cursor messages. The crate imports `DefSubclassProc` and `SetWindowSubclass`, but never imports or calls `RemoveWindowSubclass`.
In `window_lifecycle.rs`, [WindowRegistry](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L502-L662>) maintains `RefCell` collections of open auxiliary and tile window handles. It provides methods to broadcast stealth, scheme, and opacity, but does not implement `Drop` or provide explicit window unhook/destruction callbacks.

## Limits

No physical monitor hotplug events were simulated, no virtual desktop configuration changes were made, and no native HWND message queues were hooked. Original statuses in `candidates.json` remain `hypothesis`.
