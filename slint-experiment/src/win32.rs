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
    DwmEnableBlurBehindWindow, DwmExtendFrameIntoClientArea, DwmIsCompositionEnabled,
    DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE, WDA_EXCLUDEFROMCAPTURE,
    WDA_NONE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
    WS_SYSMENU,
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

/// Force-hide `window` at the Win32 level (`ShowWindow(SW_HIDE)`). Slint's own
/// `Window::hide()` does NOT reliably clear an overlay window that lives on a
/// NON-PRIMARY monitor — its pixels can linger until that monitor repaints —
/// but an explicit `SW_HIDE` does (the same call `hide_own_windows` uses for
/// the F8 capture-hide). The tile close paths call this in ADDITION to
/// `hide()` so "close all" actually clears tiles the user moved to a second
/// screen. No-op if the HWND can't be resolved.
pub fn force_hide(window: &slint::Window) {
    if let Ok(hwnd) = grab_hwnd(window) {
        // SAFETY: a live top-level window owned by this process; SW_HIDE only
        // toggles visibility — no lifetime or threading hazard.
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
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

/// True if DWM desktop composition is active. Per-pixel-alpha transparency
/// (`DwmEnableBlurBehindWindow`, above) REQUIRES this — without it the overlay
/// renders OPAQUE (the transparent margins show as black) no matter what we wire.
/// It can be OFF on RDP / remote sessions, some VMs without a virtual GPU, or
/// with a very old GPU driver. Logged at startup so a "transparency doesn't work"
/// report is diagnosable from the log instead of guessed. NOTE: this is NOT the
/// Windows "Transparency effects" toggle (that only gates acrylic/Mica, a
/// different API).
#[must_use]
pub fn composition_enabled() -> bool {
    unsafe {
        DwmIsCompositionEnabled()
            .map(|b| b.as_bool())
            .unwrap_or(false)
    }
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
    // Diagnostic only — gate behind debug builds so normal (release) window
    // creation isn't spammed with this per-window line (it also bypasses the
    // timestamped file log). cfg! keeps it compiled, so `after`/`transparent_bit`
    // stay "used"; the format work is just skipped at runtime in release.
    if cfg!(debug_assertions) {
        eprintln!(
            "[overlay-host] apply_transparency: click_through={} before=0x{:x} target=0x{:x} after=0x{:x} \
             transparent_bit_after={}",
            click_through,
            before,
            target,
            after,
            (after & transparent_bit) != 0,
        );
    }

    // V0.8.4 — kill the ghost caption buttons. Slint's `no-frame: true` leaves
    // WS_CAPTION | WS_SYSMENU | WS_MAXIMIZEBOX | WS_MINIMIZEBOX on the HWND; once
    // we extend the DWM frame into the client area (below), DWM paints the
    // caption's close/min/max glyphs faintly in the top-right corner — the
    // "еле заметный крестик" the user kept reporting on the bar (and it was on
    // tiles too). Clearing the sys-menu + min/max bits drops those non-client
    // buttons while leaving WS_CAPTION/WS_THICKFRAME, so the DWM frame extension
    // is unchanged. Verified live: the × disappears, transparency + rounded
    // corners stay intact. Alt+F4 also goes away — fine, every overlay window
    // has its own close affordance (the bar's X chip, the tile's X button).
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let no_buttons = style
            & !(WS_SYSMENU.0 as isize | WS_MAXIMIZEBOX.0 as isize | WS_MINIMIZEBOX.0 as isize);
        if no_buttons != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, no_buttons);
            // SWP_FRAMECHANGED forces a non-client recompute so the removed
            // buttons stop being drawn immediately (not on the next frame).
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
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

/// (x, y, width, height) of the ENTIRE virtual desktop spanning all monitors.
/// The origin can be NEGATIVE (the user's portrait secondary sits at x = -1200),
/// so callers must never assume (0,0). Physical pixels.
#[must_use]
pub fn virtual_screen_bounds() -> (i32, i32, i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

/// Current mouse-cursor position in physical virtual-screen coordinates.
#[must_use]
pub fn cursor_pos() -> (i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

/// Capture a screen rectangle (physical virtual-screen coords) as TOP-DOWN
/// BGRA bytes (4 bytes/pixel; `len == w*h*4`) via a GDI BitBlt of the desktop
/// DC. NOTE: GDI capture IGNORES WDA_EXCLUDEFROMCAPTURE, so any of our own
/// windows inside the rect WILL appear — hide them first (`hide_own_windows`).
pub fn capture_rect_bgra(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    if w <= 0 || h <= 0 {
        return Err(format!("invalid capture size {w}x{h}").into());
    }
    let buf_len = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("capture size overflow")?;
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("GetDC(screen) failed".into());
        }
        let mem = CreateCompatibleDC(Some(screen));
        if mem.is_invalid() {
            let _ = ReleaseDC(None, screen);
            return Err("CreateCompatibleDC failed".into());
        }
        let bmp = CreateCompatibleBitmap(screen, w, h);
        if bmp.is_invalid() {
            let _ = DeleteDC(mem);
            let _ = ReleaseDC(None, screen);
            return Err("CreateCompatibleBitmap failed".into());
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        let blt = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY);
        // Deselect the bitmap from the DC BEFORE reading its bits: GetDIBits
        // requires the bitmap NOT be selected into any DC (documented contract).
        SelectObject(mem, old);
        // Skip the large (full-virtual-desktop) GetDIBits copy + buffer alloc
        // entirely when BitBlt failed — but still run the GDI cleanup below on
        // every path.
        let result: Result<Vec<u8>, Box<dyn std::error::Error>> = if blt.is_err() {
            Err("BitBlt failed".into())
        } else {
            let mut buf = vec![0u8; buf_len];
            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // negative => top-down rows
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let lines = GetDIBits(
                mem,
                bmp,
                0,
                h as u32,
                Some(buf.as_mut_ptr().cast::<c_void>()),
                &mut bi,
                DIB_RGB_COLORS,
            );
            if lines == 0 {
                Err("GetDIBits returned 0 scanlines".into())
            } else {
                Ok(buf)
            }
        };
        // Free the remaining GDI objects on all paths.
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, screen);
        result
    }
}

/// Hide every visible top-level window owned by THIS process; returns each
/// hidden window's HWND (as `isize`) PLUS whether it was topmost, so
/// `show_windows` can restore BOTH visibility and the always-on-top band —
/// keeps our bar/tiles out of a GDI screenshot without dropping them behind
/// other windows afterward.
#[must_use]
pub fn hide_own_windows() -> Vec<(isize, bool)> {
    use std::cell::RefCell;
    use windows::core::BOOL;
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOPMOST,
    };
    thread_local! {
        static HIDDEN: RefCell<Vec<(isize, bool)>> = const { RefCell::new(Vec::new()) };
    }
    unsafe extern "system" fn cb(hwnd: HWND, _l: LPARAM) -> BOOL {
        let pid = std::process::id();
        let mut wpid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut wpid)) };
        if wpid == pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            let was_topmost = (ex & (WS_EX_TOPMOST.0 as isize)) != 0;
            let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
            HIDDEN.with(|h| h.borrow_mut().push((hwnd.0 as isize, was_topmost)));
        }
        true.into()
    }
    HIDDEN.with(|h| h.borrow_mut().clear());
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(0));
    }
    // Force a DWM compositor flush so the windows we just hid are gone from the
    // next composited frame BEFORE the caller's GDI BitBlt reads the screen.
    // Without it, on a busy frame the just-hidden bar/tiles could still be
    // captured into the (possibly cloud) vision screenshot — GDI BitBlt ignores
    // WDA_EXCLUDEFROMCAPTURE, so this hide-then-flush is the only thing keeping
    // our own overlay out of the outbound image.
    {
        use windows::Win32::Graphics::Dwm::DwmFlush;
        let _ = unsafe { DwmFlush() };
    }
    HIDDEN.with(|h| h.borrow().clone())
}

/// Re-show windows hidden by `hide_own_windows` without stealing focus, and
/// re-assert HWND_TOPMOST for those that were topmost — hide/show drops the
/// always-on-top band, so without this the bar/tiles fall behind the foreground
/// window after a capture (same fix as `apply_transparency`).
pub fn show_windows(hwnds: &[(isize, bool)]) {
    for &(h, was_topmost) in hwnds {
        let hwnd = HWND(h as *mut std::ffi::c_void);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            if was_topmost {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

/// Bring a window to the foreground + give it keyboard focus so its FocusScope
/// receives key events (e.g. Esc on the capture overlay). Unlike the always-on-
/// top bar (which avoids activation to not steal focus), the capture overlay is
/// modal and SHOULD take focus.
pub fn focus_window(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Force a window VISIBLE at the Win32 level and bring it to the foreground.
/// Used when re-opening a REUSED overlay window (Settings) that may have been
/// hidden OUT FROM UNDER Slint by `hide_own_windows()` — the F8 / capture-chip
/// flow hides every app window via `SW_HIDE`, but Slint's own visibility state
/// is unchanged, so a later `window().show()` is a no-op and the window would
/// stay invisible with no way back. `SW_SHOW` bypasses that stale state, so the
/// gear ALWAYS brings Settings back. WDA stealth + DWM alpha persist across
/// show/hide, so this does not un-stealth the window.
pub fn reveal_window(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_SHOW};
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
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

/// v0.17.1 — ask DWM to round this window's corners (Windows 11). A frameless
/// Slint window with an OPAQUE background (the archive / settings / palette,
/// which can't use per-pixel-alpha rounding like the transparent-overlay
/// tiles) otherwise shows hard square corners; an inner `border-radius` only
/// rounds the FILL, leaving the window's own square edges. `DWMWCP_ROUND`
/// clips the actual window region at the OS level — all four corners, no
/// content change. No-op on Windows 10 (the attribute is silently ignored).
pub fn set_round_corners(hwnd: HWND) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE};
    // DWMWCP_ROUND = 2 (the standard, larger-radius rounding).
    const DWMWCP_ROUND: u32 = 2;
    unsafe {
        let pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(pref).cast(),
            std::mem::size_of::<u32>() as u32,
        );
    }
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

// ===== V0.8.0 (Поток B) — single-instance guard for the emergency restart =====
//
// The ⟳ chip relaunches the app to recover from a hung local AI. The OLD
// process spawns the NEW one (with `--relaunch`) then quits. For a brief window
// both run. Two live instances are bad: they'd both register the F3/F4/F6/F8/F9
// global hotkeys (the 2nd registration fails) AND show two overlay bars — under
// stealth the 2nd bar could flash on the stream before WDA. So the relaunched
// child WAITS for the parent to fully exit (releasing this named mutex) before
// it registers hotkeys + shows its bar.
//
// Mechanism: a named mutex `Global\suflyor-overlay-singleton`. The process that
// holds it is "the live instance". On a normal launch acquisition is immediate.
// On `--relaunch`, `wait_for_singleton` blocks until the parent drops it (parent
// quits → handle closed → mutex released → WAIT_OBJECT_0 or WAIT_ABANDONED).

/// Owns the single-instance named mutex for this process's lifetime; releasing
/// it (drop / process exit) lets a waiting relaunch child proceed. Keep the
/// returned guard alive in `main` (e.g. `let _singleton = ...;`).
pub struct SingletonGuard {
    handle: windows::Win32::Foundation::HANDLE,
}

impl Drop for SingletonGuard {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::ReleaseMutex;
        unsafe {
            let _ = ReleaseMutex(self.handle);
            let _ = CloseHandle(self.handle);
        }
    }
}

const SINGLETON_MUTEX_NAME: windows::core::PCWSTR =
    windows::core::w!("Global\\suflyor-overlay-singleton");

/// Acquire the single-instance mutex, blocking up to `wait_ms` for any prior
/// holder (a still-exiting parent on `--relaunch`) to release it. Returns the
/// guard on success. `Err` means the wait timed out (another instance is still
/// alive) — the caller should bail rather than run a second bar.
///
/// `wait_ms = 0` = try-once (normal launch: acquire immediately or report busy).
pub fn acquire_singleton(wait_ms: u32) -> Result<SingletonGuard, Box<dyn std::error::Error>> {
    use windows::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
    unsafe {
        // CreateMutexW returns a handle to the existing mutex if one exists
        // (initial-owner=false: we don't own it until WaitForSingleObject).
        let handle = CreateMutexW(None, false, SINGLETON_MUTEX_NAME)?;
        if handle.is_invalid() {
            return Err("CreateMutexW returned invalid handle".into());
        }
        let wait = WaitForSingleObject(handle, wait_ms);
        // WAIT_OBJECT_0 = acquired; WAIT_ABANDONED = prior owner died without
        // releasing (we still own it now — fine for our use). Anything else
        // (timeout) = another instance is alive: close our ref and fail.
        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
            Ok(SingletonGuard { handle })
        } else {
            use windows::Win32::Foundation::CloseHandle;
            let _ = CloseHandle(handle);
            Err("singleton mutex busy (another instance is running)".into())
        }
    }
}

// ===== Read-aloud helpers: copy-the-selection + clipboard text =====

/// Read the current clipboard as UTF-8 text (None if empty / not text).
pub fn clipboard_read_text() -> Option<String> {
    clipboard_win::get_clipboard_string()
        .ok()
        .filter(|s| !s.is_empty())
}

/// Replace the clipboard text — used to restore the user's clipboard after a
/// programmatic copy.
pub fn clipboard_write_text(text: &str) {
    let _ = clipboard_win::set_clipboard_string(text);
}

/// Plant an empty sentinel so a subsequent Ctrl+C tells us whether anything was
/// actually selected (still empty afterwards = nothing was highlighted).
pub fn clipboard_clear() {
    let _ = clipboard_win::set_clipboard_string("");
}

/// Synthesize Ctrl+C to the FOREGROUND window. The overlay is click-through and
/// never focused, so the keystroke lands in whatever app the user is looking at,
/// copying their current selection. All four key events go in ONE `SendInput` so
/// Ctrl is reliably released (a stuck Ctrl would mangle the user's next keys).
///
/// Each event carries its real hardware SCAN CODE (via `MapVirtualKeyW`), not
/// just the virtual key: some apps — notably Telegram Desktop and other Qt
/// builds — read the scan code and IGNORE a synthetic keystroke whose `wScan`
/// is 0, so a bare-vk Ctrl+C copied nothing there. Right-Alt is an extended key,
/// flagged so its scan code is interpreted correctly.
pub fn send_ctrl_c() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_CONTROL, VK_LMENU,
        VK_LSHIFT, VK_MENU, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    };
    let vk_c = VIRTUAL_KEY(0x43); // 'C'
    let ev = |vk: VIRTUAL_KEY, up: bool| {
        let scan = unsafe { MapVirtualKeyW(u32::from(vk.0), MAPVK_VK_TO_VSC) } as u16;
        let mut flags = if up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        };
        if vk == VK_RMENU {
            flags |= KEYEVENTF_EXTENDEDKEY; // right-Alt is an extended key
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    };
    // CRITICAL: the Shift+Alt that fired the hotkey are still physically DOWN, so
    // a bare Ctrl+C injection reads as Shift+Alt+Ctrl+C (NOT copy). Release both
    // modifiers (all L/R variants) first, in the same atomic SendInput batch, so
    // the Ctrl+C lands clean and actually copies the selection.
    let inputs = [
        ev(VK_LMENU, true),
        ev(VK_RMENU, true),
        ev(VK_MENU, true),
        ev(VK_LSHIFT, true),
        ev(VK_RSHIFT, true),
        ev(VK_SHIFT, true),
        ev(VK_CONTROL, false),
        ev(vk_c, false),
        ev(vk_c, true),
        ev(VK_CONTROL, true),
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
