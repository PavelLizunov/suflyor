//! Win32 HWND helpers for Slint windows on Windows.
//!
//! Extracted from the Phase 0.5 / Phase 1 Day 1 overlay-spike. Provides
//! the migrate-target API for Phase 1+ window management:
//!
//! - `grab_hwnd(window) -> Result<HWND, _>` — two-step raw-window-handle
//!   unwrap. Must be called AFTER the first event-loop tick, otherwise
//!   slint returns `HandleError::NotSupported`.
//! - `make_transparent_overlay(hwnd)` — applies WS_EX_TRANSPARENT +
//!   WS_EX_TOOLWINDOW + DWM frame extension + DWM blur-behind region.
//!   Result on Windows 11: window background composites with per-pixel
//!   alpha, click events pass through, no taskbar entry. Confirmed
//!   visually 2026-05-27 in commit e46df21.
//! - `set_always_on_top(hwnd, bool)` — toggles HWND_TOPMOST.
//! - `set_stealth(hwnd, bool)` — toggles WDA_EXCLUDEFROMCAPTURE; window
//!   becomes invisible to Print Screen + Teams/Meet screen-share.
//!
//! Multi-monitor positioning via EnumDisplayMonitors + pick_monitor
//! is used by overlay_host to place tiles on the correct display
//! (respects the user's portrait-secondary setup).

#![allow(clippy::missing_errors_doc)]

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmExtendFrameIntoClientArea, DWM_BB_BLURREGION, DWM_BB_ENABLE,
    DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

/// Extract a raw Win32 HWND from any Slint window. Requires the
/// `raw-window-handle-06` feature on `slint`.
///
/// Two-step: `slint::Window::window_handle()` returns slint's wrapper
/// `slint::WindowHandle`, which implements `raw_window_handle::
/// HasWindowHandle`. That yields the real `raw_window_handle::WindowHandle`
/// from which we pull the Win32 HWND.
///
/// IMPORTANT: must be called AFTER the first event-loop iteration.
/// `slint::Timer::single_shot(Duration::from_millis(200), ...)` after
/// `window.show()` / `window.run()` is the reliable pattern. Calling
/// earlier returns `HandleError::NotSupported` because winit realizes
/// the native window lazily.
pub fn grab_hwnd(window: &slint::Window) -> Result<HWND, Box<dyn std::error::Error>> {
    let slint_handle = window.window_handle();
    let raw = slint_handle.window_handle()?;
    match raw.as_raw() {
        RawWindowHandle::Win32(w32) => Ok(HWND(w32.hwnd.get() as *mut _)),
        other => Err(format!("not a Win32 window handle: {other:?}").into()),
    }
}

/// Apply the overlay flag combination + DWM transparency wiring.
///
/// After this call:
/// - WS_EX_TRANSPARENT — mouse/touch events pass through to underlying windows
/// - WS_EX_TOOLWINDOW — no taskbar / Alt-Tab entry
/// - DwmExtendFrameIntoClientArea with margins=-1 — DWM frame covers whole client area
/// - DwmEnableBlurBehindWindow with empty region — flags window for per-pixel alpha compositing
///
/// Combined with Slint's `Window { background: transparent; }` declaration,
/// this yields a true transparent overlay on Windows 11. Confirmed
/// visually 2026-05-27.
pub fn make_transparent_overlay(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>> {
    let before = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let added = WS_EX_TRANSPARENT.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, before | added);
    }

    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins)? };

    let h_rgn = unsafe { CreateRectRgn(0, 0, -1, -1) };
    let bb = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
        fEnable: true.into(),
        hRgnBlur: h_rgn,
        fTransitionOnMaximized: false.into(),
    };
    unsafe {
        let result = DwmEnableBlurBehindWindow(hwnd, &bb);
        let _ = DeleteObject(h_rgn.into());
        result?;
    }

    Ok(())
}

/// Toggle HWND_TOPMOST. `true` puts the window above all non-topmost
/// windows; `false` reverts to the normal Z-order.
///
/// Slint's `always-on-top: true;` property does the same thing
/// declaratively. Use this helper for runtime toggling (e.g. a
/// Settings switch).
pub fn set_always_on_top(hwnd: HWND, on: bool) -> Result<(), Box<dyn std::error::Error>> {
    let insert_after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
    unsafe {
        SetWindowPos(
            hwnd,
            Some(insert_after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        )?;
    }
    Ok(())
}

/// Toggle WDA_EXCLUDEFROMCAPTURE — the window becomes invisible in
/// Print Screen, screen recording, and Teams/Meet screen share.
///
/// This is the "stealth" toggle in the existing overlay-mvp Settings
/// → Stealth panel. WDA_EXCLUDEFROMCAPTURE is the Win32 mechanism
/// behind it. Requires Windows 10 build 2004 / Windows 11.
pub fn set_stealth(hwnd: HWND, on: bool) -> Result<(), Box<dyn std::error::Error>> {
    let affinity = if on { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
    unsafe {
        SetWindowDisplayAffinity(hwnd, affinity)?;
    }
    Ok(())
}

/// Bounds of a display monitor in screen-coordinate space.
#[derive(Debug, Clone, Copy)]
pub struct MonitorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub is_primary: bool,
}

impl MonitorRect {
    #[must_use]
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    #[must_use]
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    #[must_use]
    pub fn is_landscape(&self) -> bool {
        self.width() >= self.height()
    }
}

/// Enumerate all attached display monitors with their bounds + primary flag.
///
/// Uses `EnumDisplayMonitors` (Win32). Consumed by `pick_monitor`
/// below + by overlay_host's `apply_tile_hwnd_with_monitor` helper
/// to choose a tile-spawn display + call `move_window` for placement.
pub fn enum_monitors() -> Vec<MonitorRect> {
    use std::cell::RefCell;
    use windows::core::BOOL;
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    // Win32 constant — equals MONITORINFOF_PRIMARY. Hardcoded to avoid
    // a feature-flag dependency that differs across windows-rs versions.
    const MONITORINFOF_PRIMARY: u32 = 0x0000_0001;

    thread_local! {
        static MONITORS: RefCell<Vec<MonitorRect>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "system" fn callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _lprect: *mut RECT,
        _lparam: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(hmonitor, &mut info) }.as_bool() {
            let rect = MonitorRect {
                left: info.rcMonitor.left,
                top: info.rcMonitor.top,
                right: info.rcMonitor.right,
                bottom: info.rcMonitor.bottom,
                is_primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
            };
            MONITORS.with(|m| m.borrow_mut().push(rect));
        }
        true.into()
    }

    MONITORS.with(|m| m.borrow_mut().clear());
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(callback), LPARAM(0));
    }
    MONITORS.with(|m| m.borrow().clone())
}

/// Position a window at the given screen coordinates with the given size.
/// Used by `apply_tile_hwnd_with_monitor` in overlay_host to drive
/// freshly-spawned tile windows onto the chosen display.
pub fn move_window(
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            w,
            h,
            windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
        )?;
    }
    Ok(())
}

/// Pick a target monitor for a new tile. Mirrors the heuristic in
/// `src-tauri/src/tile.rs::pick_monitor` — default to primary unless
/// a non-primary monitor is landscape AND at least as wide as primary.
///
/// User memory `[[user-setup-monitors]]`: primary 1920x1080 landscape,
/// secondary 1200x1920 PORTRAIT at x=-1200. The "first non-primary"
/// default in earlier versions put tiles invisibly off-screen. This
/// helper preserves the fix.
#[must_use]
pub fn pick_monitor(monitors: &[MonitorRect]) -> Option<MonitorRect> {
    let primary = monitors.iter().find(|m| m.is_primary).copied()?;
    let upgrade = monitors
        .iter()
        .filter(|m| !m.is_primary && m.is_landscape() && m.width() >= primary.width())
        .copied()
        .next();
    Some(upgrade.unwrap_or(primary))
}
