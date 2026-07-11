//! Persistent session-name sidecar — a single, atomically-written JSON map of
//! `session_id → human title`, at `%APPDATA%\suflyor\session_names.json`.
//!
//! v0.22.0. The live overlay bar shows the auto-generated name from RAM; THIS
//! persists it so the archive can title past sessions and the user can rename
//! them — deliberately WITHOUT a SQLite migration. Low-risk by construction:
//! one small file, written atomically (tmp + rename), pruned to the newest
//! [`MAX_ENTRIES`]. The session id is a journal stem / archive id (already
//! path-safe) and is used only as a map KEY here, so there is no path-traversal
//! surface — nothing is ever joined onto a filesystem path from it.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FILE: &str = "session_names.json";

/// Cap on stored titles. Far above a heavy user's session count; the file is a
/// few tens of KB at the cap. On overflow the OLDEST (by `ts`) are dropped.
const MAX_ENTRIES: usize = 2000;

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    name: String,
    /// unix-ms when the title was set — used only to pick survivors on prune.
    ts: u128,
}

type Map = HashMap<String, Entry>;

fn file_path_in(root: &Path) -> PathBuf {
    root.join(FILE)
}

fn load_in(root: &Path) -> Map {
    std::fs::read_to_string(file_path_in(root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_in(root: &Path, map: &Map) -> std::io::Result<()> {
    let json = serde_json::to_string(map)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let final_path = file_path_in(root);
    let tmp = final_path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &final_path)
}

fn prune(map: &mut Map) {
    if map.len() <= MAX_ENTRIES {
        return;
    }
    let mut by_recency: Vec<(String, u128)> = map.iter().map(|(k, e)| (k.clone(), e.ts)).collect();
    by_recency.sort_by_key(|(_, ts)| std::cmp::Reverse(*ts)); // newest first
    let keep: std::collections::HashSet<String> = by_recency
        .into_iter()
        .take(MAX_ENTRIES)
        .map(|(k, _)| k)
        .collect();
    map.retain(|k, _| keep.contains(k));
}

/// Set/replace a session's title (atomically), pruning to the newest entries.
/// `ts` is unix-ms. Empty `session_id` is a no-op (ephemeral / test sessions).
pub fn set_in(root: &Path, session_id: &str, name: &str, ts: u128) -> std::io::Result<()> {
    if session_id.is_empty() {
        return Ok(());
    }
    let mut map = load_in(root);
    map.insert(
        session_id.to_string(),
        Entry {
            name: name.to_string(),
            ts,
        },
    );
    prune(&mut map);
    write_in(root, &map)
}

/// The human title for a session, if one was set. An empty stored title reads
/// as `None` (so clearing the rename field reverts the archive to the time).
#[must_use]
pub fn get_in(root: &Path, session_id: &str) -> Option<String> {
    load_in(root)
        .get(session_id)
        .map(|e| e.name.clone())
        .filter(|n| !n.is_empty())
}

/// Convenience: set against `%APPDATA%\suflyor`. Best-effort — a failed write is
/// logged, not propagated (the live name in RAM is the source of truth).
pub fn set(session_id: &str, name: &str, ts: u128) {
    if let Some(root) = crate::paths::data_root() {
        if let Err(e) = set_in(&root, session_id, name, ts) {
            log::warn!("session_names: persist failed: {e}");
        }
    }
}

/// Convenience: read against `%APPDATA%\suflyor`.
#[must_use]
pub fn get(session_id: &str) -> Option<String> {
    crate::paths::data_root().and_then(|root| get_in(&root, session_id))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests assert directly; runtime code stays strict"
)]
mod tests {
    use super::*;

    #[test]
    fn set_then_get_round_trips() {
        let dir = std::env::temp_dir().join(format!("sn_rt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        set_in(&dir, "sess-1", "Обзор функций", 100).unwrap();
        assert_eq!(get_in(&dir, "sess-1").as_deref(), Some("Обзор функций"));
        assert_eq!(get_in(&dir, "missing"), None);
        // Replace.
        set_in(&dir, "sess-1", "Новое имя", 200).unwrap();
        assert_eq!(get_in(&dir, "sess-1").as_deref(), Some("Новое имя"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn whitespace_only_title_reads_as_none() {
        let dir = std::env::temp_dir().join(format!("sn_ws_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        set_in(&dir, "sess-blank", "    ", 100).unwrap();
        assert_eq!(get_in(&dir, "sess-blank"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_id_is_noop() {
        let dir = std::env::temp_dir().join(format!("sn_empty_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        set_in(&dir, "", "ghost", 1).unwrap();
        assert!(!file_path_in(&dir).exists(), "no file written for empty id");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_loads_empty() {
        let dir = std::env::temp_dir().join(format!("sn_miss_{}", std::process::id()));
        assert_eq!(get_in(&dir, "anything"), None);
    }

    #[test]
    fn prune_keeps_newest_by_ts() {
        let mut map: Map = HashMap::new();
        for i in 0..(MAX_ENTRIES + 50) {
            map.insert(
                format!("s{i}"),
                Entry {
                    name: format!("n{i}"),
                    ts: i as u128, // higher i = newer
                },
            );
        }
        prune(&mut map);
        assert_eq!(map.len(), MAX_ENTRIES);
        // The newest (highest ts) must survive; the oldest must be gone.
        assert!(map.contains_key(&format!("s{}", MAX_ENTRIES + 49)));
        assert!(!map.contains_key("s0"));
    }
}
