//! AppKit window behavior for Slint surfaces on macOS.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_int, c_void};

extern "C" {
    fn suflyor_macos_configure_floating_window(view: *mut c_void) -> c_int;
    fn suflyor_macos_begin_window_drag(view: *mut c_void) -> c_int;
}

fn appkit_view(window: &slint::Window) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    let slint_handle = window.window_handle();
    let handle = slint_handle.window_handle()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(appkit) => Ok(appkit.ns_view.as_ptr()),
        other => Err(format!("expected an AppKit window handle, got {other:?}").into()),
    }
}

/// Apply the floating-overlay behavior validated by the Gate 0A prototype.
pub fn configure_floating(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>> {
    let configured = unsafe { suflyor_macos_configure_floating_window(appkit_view(window)?) };
    if configured == 1 {
        Ok(())
    } else {
        Err("AppKit view has no window".into())
    }
}

/// Begin a native AppKit drag from the current left-button press event.
pub fn begin_drag(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>> {
    let started = unsafe { suflyor_macos_begin_window_drag(appkit_view(window)?) };
    if started == 1 {
        Ok(())
    } else {
        Err("native window drag requires a left-button press".into())
    }
}
