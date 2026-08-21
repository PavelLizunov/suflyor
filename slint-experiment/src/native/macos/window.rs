//! AppKit window behavior for Slint surfaces on macOS.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_int, c_void};

extern "C" {
    fn suflyor_macos_configure_floating_window(view: *mut c_void) -> c_int;
    fn suflyor_macos_begin_window_drag(view: *mut c_void) -> c_int;
    fn suflyor_macos_raise_window_key_front(view: *mut c_void) -> c_int;
    fn suflyor_macos_get_window_rect(
        view: *mut c_void,
        out_x: *mut i32,
        out_y: *mut i32,
        out_width: *mut i32,
        out_height: *mut i32,
    ) -> c_int;
}

fn appkit_view(window: &slint::Window) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    let slint_handle = window.window_handle();
    let handle = slint_handle.window_handle()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(appkit) => Ok(appkit.ns_view.as_ptr()),
        other => Err(format!("expected an AppKit window handle, got {other:?}").into()),
    }
}

fn view_from_id(view_id: isize) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    if view_id == 0 {
        Err("AppKit view id is null".into())
    } else {
        Ok(view_id as *mut c_void)
    }
}

/// Stable identity of the AppKit view while the Slint window is alive.
pub fn view_id(window: &slint::Window) -> Result<isize, Box<dyn std::error::Error>> {
    Ok(appkit_view(window)? as isize)
}

/// Apply the floating-overlay behavior validated by the Gate 0A prototype.
pub fn configure_floating(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>> {
    configure_floating_by_id(view_id(window)?)
}

/// Apply floating behavior to a previously resolved AppKit view id.
pub fn configure_floating_by_id(view_id: isize) -> Result<(), Box<dyn std::error::Error>> {
    let configured = unsafe { suflyor_macos_configure_floating_window(view_from_id(view_id)?) };
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

/// Activate the app and make the window key and front, so reopening a
/// surface while another app is active regains keyboard focus.
pub fn raise_key_front(window: &slint::Window) -> Result<(), Box<dyn std::error::Error>> {
    raise_key_front_by_id(view_id(window)?)
}

/// Raise a window through a previously resolved AppKit view id.
pub fn raise_key_front_by_id(view_id: isize) -> Result<(), Box<dyn std::error::Error>> {
    let raised = unsafe { suflyor_macos_raise_window_key_front(view_from_id(view_id)?) };
    if raised == 1 {
        Ok(())
    } else {
        Err("AppKit view has no window".into())
    }
}

/// Read an AppKit window frame in CoreGraphics global screen coordinates.
pub fn window_rect_by_id(
    view_id: isize,
) -> Result<(i32, i32, i32, i32), Box<dyn std::error::Error>> {
    let mut x = 0;
    let mut y = 0;
    let mut width = 0;
    let mut height = 0;
    let read = unsafe {
        suflyor_macos_get_window_rect(
            view_from_id(view_id)?,
            &mut x,
            &mut y,
            &mut width,
            &mut height,
        )
    };
    if read == 1 && width > 0 && height > 0 {
        Ok((x, y, width, height))
    } else {
        Err("AppKit view has no measurable window".into())
    }
}
