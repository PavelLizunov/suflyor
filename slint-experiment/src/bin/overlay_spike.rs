//! Phase 0.5 spike — HWND-poking transparent always-on-top overlay.
//!
//! The migration plan's #1 risk is whether Slint+winit+skia honor the
//! same overlay invariants the existing Tauri/WebView2 build does:
//!
//! - Background transparency (window.background = transparent + WS_EX_LAYERED)
//! - Always-on-top (Slint's always-on-top property OR Win32 SetWindowPos HWND_TOPMOST)
//! - No-frame (Slint's no-frame property)
//! - Click-through (WS_EX_TRANSPARENT — Win32 only, no Slint property)
//! - Hidden from screen capture (WDA_EXCLUDEFROMCAPTURE via SetWindowDisplayAffinity)
//!   — stretch; not exercised in this spike to keep it minimal.
//!
//! This binary spawns the OverlaySpike window, grabs its raw HWND via
//! `raw-window-handle 0.6`, applies the three Win32 EX flags, prints
//! the before/after GetWindowLongPtrW result to stderr, then closes
//! itself after 8 s so the smoke script can screenshot + verify.
//!
//! Run: `cargo run --bin overlay-spike` from `slint-experiment/`.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
};

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use ui::OverlaySpike;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = OverlaySpike::new()?;

    // Deferred HWND grab — slint::Window::window_handle() returns
    // HandleError::NotSupported until the native window is realized,
    // which happens during the first event-loop iteration. A 200 ms
    // single-shot timer that grabs after run() starts is reliable.
    let weak = window.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let Some(w) = weak.upgrade() else { return };
        match grab_hwnd(&w) {
            Ok(hwnd) => apply_and_log(hwnd),
            Err(e) => eprintln!("[overlay-spike] FAILURE: grab_hwnd: {e}"),
        }
    });

    // Auto-close timer so the smoke script doesn't have to manage a
    // separate kill step. Slint's Timer fires on the UI thread.
    let weak_close = window.as_weak();
    slint::Timer::single_shot(Duration::from_secs(8), move || {
        if let Some(w) = weak_close.upgrade() {
            eprintln!("[overlay-spike] 8 s elapsed, hiding window.");
            let _ = w.hide();
        }
    });

    window.run()?;
    Ok(())
}

fn apply_and_log(hwnd: HWND) {
    eprintln!("[overlay-spike] HWND = {:?}", hwnd.0);

    let before = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    eprintln!("[overlay-spike] EX style before: 0x{:x}", before as usize);

    // Variant choice: WS_EX_LAYERED triggers GDI-style alpha which Slint's
    // skia/winit rendering doesn't paint into, leaving the layered surface
    // opaque black. The Tauri/WebView2 version achieves transparency via
    // DWM compositing rather than WS_EX_LAYERED — winit's default window
    // creation already enables DWM compositing, and Slint's
    // `background: transparent` should drive per-pixel alpha through it.
    //
    // So this spike adds ONLY:
    //   - WS_EX_TRANSPARENT  (click-through; doesn't affect rendering)
    //   - WS_EX_TOOLWINDOW   (no taskbar entry; doesn't affect rendering)
    //
    // And RELIES on Slint's `background: transparent;` + winit default
    // compositing for actual transparency. If the window still renders
    // opaque, the migration needs to investigate winit::WindowBuilder
    // ::with_transparent(true) wiring inside Slint's backend.
    let added = WS_EX_TRANSPARENT.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
    let new_style = before | added;
    let prev = unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style) };
    eprintln!(
        "[overlay-spike] SetWindowLongPtrW returned: 0x{:x}",
        prev as usize
    );

    let after = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    eprintln!("[overlay-spike] EX style after:  0x{:x}", after as usize);

    let layered = (after as u32) & WS_EX_LAYERED.0 != 0;
    let transparent = (after as u32) & WS_EX_TRANSPARENT.0 != 0;
    let toolwindow = (after as u32) & WS_EX_TOOLWINDOW.0 != 0;
    eprintln!(
        "[overlay-spike] flags now: LAYERED={layered}, TRANSPARENT={transparent}, TOOLWINDOW={toolwindow}"
    );

    if transparent && toolwindow {
        eprintln!(
            "[overlay-spike] SUCCESS: TRANSPARENT + TOOLWINDOW flags stuck. \
             Visible transparency depends on Slint's winit-transparent wiring (see screenshot)."
        );
    } else {
        eprintln!("[overlay-spike] FAILURE: required flags did not apply.");
    }
}

/// Extract a raw Win32 HWND from a Slint window. Requires the
/// `raw-window-handle-06` feature on slint.
///
/// Two-step: slint::Window::window_handle() returns slint::WindowHandle
/// (slint's wrapper); slint::WindowHandle implements raw_window_handle::
/// HasWindowHandle, which yields the raw_window_handle::WindowHandle.
fn grab_hwnd(window: &OverlaySpike) -> Result<HWND, Box<dyn std::error::Error>> {
    let slint_handle = window.window().window_handle();
    let raw = slint_handle.window_handle()?;
    match raw.as_raw() {
        RawWindowHandle::Win32(w32) => Ok(HWND(w32.hwnd.get() as *mut _)),
        other => Err(format!("not a Win32 window handle: {other:?}").into()),
    }
}
