//! Native macOS menu-bar recovery for the production overlay.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_int, c_void};
use std::marker::PhantomData;
use std::rc::Rc;

extern "C" {
    fn suflyor_macos_status_install(
        view: *mut c_void,
        on_quit: extern "C" fn(),
        on_visibility: extern "C" fn(bool),
    ) -> c_int;
    fn suflyor_macos_status_remove();
}

extern "C" fn quit_through_event_loop() {
    let _ = slint::quit_event_loop();
}

fn appkit_view(window: &slint::Window) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    let slint_handle = window.window_handle();
    let handle = slint_handle.window_handle()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(appkit) => Ok(appkit.ns_view.as_ptr()),
        other => Err(format!("expected an AppKit window handle, got {other:?}").into()),
    }
}

/// Owns the process-wide native status item.
pub struct StatusItemGuard {
    _main_thread: PhantomData<Rc<()>>,
}

impl Drop for StatusItemGuard {
    fn drop(&mut self) {
        unsafe { suflyor_macos_status_remove() };
    }
}

/// Install synchronously once the host's startup timer can see the AppKit view.
pub fn install(
    window: &slint::Window,
    on_visibility: extern "C" fn(bool),
) -> Result<StatusItemGuard, Box<dyn std::error::Error>> {
    let result = unsafe {
        suflyor_macos_status_install(appkit_view(window)?, quit_through_event_loop, on_visibility)
    };
    if result == 1 {
        Ok(StatusItemGuard {
            _main_thread: PhantomData,
        })
    } else {
        Err("native status item is unavailable".into())
    }
}
