//! Windows process-lifecycle primitives.
//!
//! Emergency relaunch starts the replacement process before the old one exits.
//! The named mutex makes the child wait before registering hotkeys or showing a
//! second overlay bar.

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
