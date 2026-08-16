//! macOS process-lifecycle primitives.
//!
//! The empty lock file stays on disk: deleting a locked inode would let a
//! second process create and lock a replacement while the first still runs.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

const SINGLETON_LOCK_NAME: &str = "suflyor-overlay-singleton.lock";

/// Keeps the per-user singleton lock alive for the process lifetime.
pub struct SingletonGuard {
    _file: File,
}

fn try_acquire(path: &Path) -> io::Result<Option<SingletonGuard>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(SingletonGuard { _file: file })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

/// Acquire the per-user singleton lock, waiting up to `wait_ms` for a relaunch.
pub fn acquire_singleton(wait_ms: u32) -> Result<SingletonGuard, Box<dyn std::error::Error>> {
    let root = overlay_backend::paths::data_root().ok_or("config directory is unavailable")?;
    std::fs::create_dir_all(&root)?;
    let path = root.join(SINGLETON_LOCK_NAME);
    let deadline = Instant::now() + Duration::from_millis(u64::from(wait_ms));
    loop {
        if let Some(guard) = try_acquire(&path)? {
            return Ok(guard);
        }
        if Instant::now() >= deadline {
            return Err("singleton lock busy (another instance is running)".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn lock_is_exclusive_and_released_on_drop() {
        let path = std::env::temp_dir().join(format!(
            "suflyor-singleton-{}-{}.lock",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let first = try_acquire(&path).unwrap().expect("first lock");
        assert!(try_acquire(&path).unwrap().is_none());
        drop(first);
        assert!(try_acquire(&path).unwrap().is_some());
        let _ = std::fs::remove_file(path);
    }
}
