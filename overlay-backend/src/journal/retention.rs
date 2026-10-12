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
    Ok(prune_entries(entries, keep, max_bytes, |path| {
        std::fs::remove_file(path)
    }))
}

fn prune_entries(
    mut entries: Vec<(SystemTime, u64, PathBuf)>,
    keep: usize,
    max_bytes: u64,
    mut remove: impl FnMut(&Path) -> std::io::Result<()>,
) -> usize {
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));

    let mut deleted = 0usize;
    for (_, _, path) in entries.iter().skip(keep) {
        if let Err(e) = remove(path) {
            log::warn!("failed to prune old session {}: {e}", path.display());
        } else {
            deleted += 1;
        }
    }

    entries.truncate(keep);

    if max_bytes > 0 {
        let mut total: u64 = entries.iter().map(|e| e.1).sum();
        if total > max_bytes {
            for (_, size, path) in entries.iter().rev() {
                if total <= max_bytes {
                    break;
                }
                if let Err(e) = remove(path) {
                    log::warn!("failed to prune session for size {}: {e}", path.display());
                } else {
                    total = total.saturating_sub(*size);
                    deleted += 1;
                }
            }
        }
    }

    deleted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_count_prune_does_not_delete_recent_sessions_for_size() {
        let entries = [0, 10, 20, 100, 1]
            .into_iter()
            .enumerate()
            .map(|(i, size)| {
                (
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(5 - i as u64),
                    size,
                    PathBuf::from(format!("{i}.jsonl")),
                )
            })
            .collect();
        let mut attempted = Vec::new();
        let deleted = prune_entries(entries, 3, 50, |path| {
            attempted.push(path.to_path_buf());
            if path == Path::new("4.jsonl") {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            } else {
                Ok(())
            }
        });
        assert_eq!(deleted, 1);
        assert_eq!(attempted, [PathBuf::from("3.jsonl"), PathBuf::from("4.jsonl")]);
    }
}
