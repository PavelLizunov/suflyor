use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const KEEP_LAST_SESSIONS: usize = 100;
pub const MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024; // 500 MB

pub fn prune_old_sessions(dir: &Path, keep: usize) -> Result<usize> {
    prune_old_sessions_with_size_cap(dir, keep, MAX_TOTAL_BYTES)
}

pub fn prune_old_sessions_with_size_cap(dir: &Path, keep: usize, max_bytes: u64) -> Result<usize> {
    let mut entries: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    for e in std::fs::read_dir(dir).context("read sessions dir")? {
        let Ok(e) = e else { continue };
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        let size = meta.len();
        entries.push((mtime, size, path));
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));

    let mut deleted = 0usize;
    for (_, _, path) in entries.iter().skip(keep) {
        if let Err(e) = std::fs::remove_file(path) {
            log::warn!("failed to prune old session {}: {e}", path.display());
        } else {
            deleted += 1;
        }
    }

    if max_bytes > 0 && entries.len() > deleted {
        let remaining_count = entries.len().saturating_sub(deleted);
        let mut total: u64 = entries.iter().take(remaining_count).map(|e| e.1).sum();
        if total > max_bytes {
            for (_, size, path) in entries.iter().take(remaining_count).rev() {
                if total <= max_bytes {
                    break;
                }
                if let Err(e) = std::fs::remove_file(path) {
                    log::warn!("failed to prune session for size {}: {e}", path.display());
                } else {
                    total = total.saturating_sub(*size);
                    deleted += 1;
                }
            }
        }
    }

    Ok(deleted)
}
