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
    GetWindowLongPtrW, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_SHOWNOACTIVATE, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
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
    apply_transparency(hwnd, /* click_through */ true)
}

/// Apply the same transparency wiring as `make_transparent_overlay`
/// but WITHOUT `WS_EX_TRANSPARENT`, so the window accepts clicks +
/// drag interaction. Use for tile windows where the user needs to
/// click buttons (pin, close) and drag the chrome row. Phase E6 —
/// fixes user complaint "тайлы нельзя двигать": tiles inherited
/// click-through from make_transparent_overlay and silently
/// swallowed every TouchArea press.
pub fn make_transparent_tile(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>> {
    apply_transparency(hwnd, /* click_through */ false)
}

fn apply_transparency(hwnd: HWND, click_through: bool) -> Result<(), Box<dyn std::error::Error>> {
    let before = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let target = if click_through {
        // Overlay bar — clicks pass through everywhere except
        // explicit TouchAreas (Slint engine).
        before | WS_EX_TRANSPARENT.0 as isize | WS_EX_TOOLWINDOW.0 as isize
    } else {
        // Tiles — explicitly CLEAR WS_EX_TRANSPARENT (Slint's
        // frameless+transparent-background setup sets it implicitly
        // on Windows). Without this AND-NOT, tiles silently swallow
        // every click → drag/buttons never fire. Phase E6 v6 root
        // cause of "тайлы нельзя двигать".
        (before | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_TRANSPARENT.0 as isize)
    };
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target);
    }
    // Phase E6 v7 diagnostic — verify the ex_style actually changed.
    // Logs the before/after bits so we can confirm WS_EX_TRANSPARENT
    // (0x20) is cleared for tiles and set for overlay. If Slint
    // re-applies WS_EX_TRANSPARENT later we'll see it diverge.
    let after = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let transparent_bit = WS_EX_TRANSPARENT.0 as isize;
    eprintln!(
        "[overlay-host] apply_transparency: click_through={} before=0x{:x} target=0x{:x} after=0x{:x} \
         transparent_bit_after={}",
        click_through,
        before,
        target,
        after,
        (after & transparent_bit) != 0,
    );

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

/// Show or hide the window's TASKBAR button. Wired to the stealth toggle so the
/// overlay also disappears from the taskbar (a screen-share viewer shouldn't see
/// "suflyor" in the taskbar). WS_EX_TOOLWINDOW/APPWINDOW only affect the taskbar
/// at show-time, so we do a brief hide -> restyle -> show-no-activate and
/// re-assert topmost. Only the TOOLWINDOW/APPWINDOW bits are touched; all other
/// ex-style bits (layered / transparent / etc.) are preserved.
pub fn set_skip_taskbar(hwnd: HWND, skip: bool) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let before = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let tool = WS_EX_TOOLWINDOW.0 as isize;
        let app = WS_EX_APPWINDOW.0 as isize;
        let after = if skip {
            (before | tool) & !app
        } else {
            (before & !tool) | app
        };
        if after == before {
            return Ok(());
        }
        let _ = ShowWindow(hwnd, SW_HIDE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, after);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        // hide/show can drop topmost — re-assert it without stealing focus.
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )?;
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

/// Move the window to (x, y) without changing its size. Used by tile
/// placement so Slint's natural sizing (per-monitor DPI scale aware)
/// stays intact while we just position the window. Fixes Phase E6
/// bug: setting `move_window(..., 460, 360)` with raw pixel sizes
/// forces the window smaller than Slint's logical-size render canvas
/// on HiDPI monitors → text overflows the dark fill area.
pub fn move_window_pos_only(hwnd: HWND, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::UI::WindowsAndMessaging::{SWP_NOSIZE, SWP_NOZORDER};
    unsafe {
        SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOZORDER | SWP_NOSIZE)?;
    }
    Ok(())
}

/// Read the actual physical window rect (x, y, w, h) for placement
/// math. Returns dimensions in screen coordinates (raw OS pixels).
/// Used by the tile-spawn poll Timer to know each tile's real size
/// after Slint's HiDPI-aware layout settles, so the right-edge
/// alignment math uses the true width.
pub fn get_window_rect(hwnd: HWND) -> Result<(i32, i32, i32, i32), Box<dyn std::error::Error>> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut r = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut r)?;
    }
    Ok((r.left, r.top, r.right - r.left, r.bottom - r.top))
}

/// Work area (monitor bounds MINUS the taskbar) of the monitor that most
/// contains `hwnd`, as a `MonitorRect`. Used by `toggle_tile_maximize` to
/// keep a maximized tile fully on-screen AND clear of the taskbar (so the
/// tile's bottom row — e.g. the follow-up input — stays reachable).
/// `is_primary` is meaningless here and always false.
#[must_use]
pub fn work_area_for_window(hwnd: HWND) -> Option<MonitorRect> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    unsafe {
        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            Some(MonitorRect {
                left: info.rcWork.left,
                top: info.rcWork.top,
                right: info.rcWork.right,
                bottom: info.rcWork.bottom,
                is_primary: false,
            })
        } else {
            None
        }
    }
}

// Phase E6 v22 — manual cursor-delta drag state.
//
// REPLACES the old WM_NCLBUTTONDOWN system-drag. That approach
// entered a Windows MODAL drag loop (SendMessageW blocks until
// mouse-up), and the modal loop CONSUMED the mouse-up event before
// Slint could see it. Slint's TouchArea then stayed stuck in the
// "pressed" state forever → every subsequent click was treated as
// a drag → bar + tiles became unclickable. User: "после того как
// кликнул на :: idle вся зона стала drag и больше ничего не
// кликается; вызванный тайл завис, двигается но ничего не
// прожимается".
//
// New model: no modal loop. We track the cursor delta ourselves.
//   drag_begin(hwnd) on pointer-down  → record cursor + window pos
//   drag_update(hwnd) on pointer-move → move window by the delta
// Slint sees the real mouse-up normally, so TouchArea state stays
// consistent and clicks keep working after a drag.

use std::cell::Cell;

thread_local! {
    /// (cursor_start_x, cursor_start_y, window_start_x, window_start_y).
    /// Set on drag_begin, read on drag_update. UI-thread-only so a
    /// thread-local Cell is sufficient (no cross-thread sharing).
    static DRAG_ANCHOR: Cell<Option<(i32, i32, i32, i32)>> = const { Cell::new(None) };
}

/// Begin a manual window drag — capture the cursor + window origin so
/// subsequent `drag_update` calls can move the window by the delta.
/// Call from the drag-handle TouchArea's pointer-event(down).
pub fn drag_begin(hwnd: HWND) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
    let mut cursor = POINT::default();
    let mut rect = RECT::default();
    unsafe {
        if GetCursorPos(&mut cursor).is_err() {
            return;
        }
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return;
        }
    }
    DRAG_ANCHOR.with(|a| a.set(Some((cursor.x, cursor.y, rect.left, rect.top))));
}

/// Continue a manual window drag — move the window so its origin
/// tracks the cursor by the same delta seen since `drag_begin`.
/// Call from the drag-handle TouchArea's `moved` callback (guarded
/// by `self.pressed` on the Slint side). No-op if no drag is active.
pub fn drag_update(hwnd: HWND) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SWP_NOSIZE, SWP_NOZORDER};
    let Some((cx0, cy0, wx0, wy0)) = DRAG_ANCHOR.with(Cell::get) else {
        return;
    };
    let mut cursor = POINT::default();
    unsafe {
        if GetCursorPos(&mut cursor).is_err() {
            return;
        }
        let nx = wx0 + (cursor.x - cx0);
        let ny = wy0 + (cursor.y - cy0);
        let _ = SetWindowPos(hwnd, None, nx, ny, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
    }
}

/// Clear drag anchor. Optional — `drag_begin` overwrites it anyway —
/// but calling on pointer-up keeps the state tidy.
pub fn drag_end() {
    DRAG_ANCHOR.with(|a| a.set(None));
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
