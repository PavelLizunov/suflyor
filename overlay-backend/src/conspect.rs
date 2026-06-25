//! Persisted, resumable meeting-summary **conspect** — the durable artifact
//! that makes the map-reduce summary crash-proof and retryable.
//!
//! v0.18.6 (tester summary bugs 2/3/4). The map-reduce summary
//! ([`crate::runtime::run_meeting_summary`]) splits a long transcript into
//! parts, summarises each ("conspectus"), then merges the part summaries into
//! the final recap. Before this module those part summaries lived only in
//! memory: any AI error at the merge step discarded them, the summary window
//! came back empty, and a re-request had nothing to work from — so the model
//! ended up asking the user to paste the conspect text by hand.
//!
//! A `Conspect` is a per-session sidecar at
//! `%APPDATA%\suflyor\conspects\<session_id>.json` that records each part's
//! SOURCE slice + its summary (filled in as the map progresses) + the final
//! recap (filled in when the reduce succeeds). It is written incrementally and
//! atomically, so:
//!
//! - a model error never loses completed parts;
//! - a **retry** re-maps only the parts that failed, then re-runs the cheap
//!   reduce — no re-STT, no re-summarising the parts that already succeeded;
//! - a **re-request** over the same transcript returns the saved recap (or
//!   re-reduces the saved parts) instead of begging for input.
//!
//! Uniform across the LIVE bar-button path and the ARCHIVE re-transcribe path
//! (both have a `session_id`; the archive session's original journal is closed,
//! so a sidecar — not a journal append — is the only home that serves both).

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Sub-directory under the data root holding the per-session sidecars.
const DIR: &str = "conspects";

/// Retention cap — keep the newest N conspect files, pruned on each save. One
/// per session and a few KB–100 KB each; this bounds an otherwise unbounded
/// directory without coupling to the journal/recording retention policies.
const KEEP_NEWEST: usize = 500;

/// One map part: the raw transcript slice it was built from + its conspectus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConspectPart {
    /// The raw (formatted) transcript slice for this part. Retained so a retry
    /// can RE-MAP a part whose summary is missing without re-running STT.
    pub source: String,
    /// The bullet conspectus for this part, or `None` if the map call for it
    /// failed or has not run yet. The reduce uses only the `Some` summaries.
    pub summary: Option<String>,
}

/// The persisted summary state for one session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conspect {
    /// Session key = journal stem (live) or archive row id. Also the filename.
    pub session_id: String,
    /// Unix-ms the conspect was first built (best-effort; 0 if the clock is
    /// unavailable). Diagnostic only — retention prunes by file mtime, not this.
    pub created_ms: u64,
    /// Language the part conspectuses were built in — the reduce prompt's
    /// default when a retry can't re-read config (it normally re-reads it).
    pub is_ru: bool,
    /// Hash of the formatted transcript at build time. A bar re-press reuses a
    /// saved conspect only when this still matches (transcript unchanged);
    /// otherwise it rebuilds from scratch. The explicit error-tile retry
    /// ignores it (the user asked to retry THIS summary).
    pub fingerprint: u64,
    /// `true` = the transcript fit the budget and was summarised in a SINGLE
    /// pass — `parts` then holds exactly one entry whose `source` is the whole
    /// formatted transcript and whose `summary` is unused (the reduce re-runs
    /// the single-pass seed over `source`). `false` = genuine map-reduce.
    pub single_pass: bool,
    /// The parts. For `single_pass` exactly one; otherwise one per map slice.
    pub parts: Vec<ConspectPart>,
    /// The finished recap, set once the reduce succeeds. Lets a re-request /
    /// reopen return instantly with zero AI calls.
    pub final_summary: Option<String>,
}

impl Conspect {
    /// Build a fresh conspect with all part SOURCES recorded up front (summaries
    /// `None`), so even a crash before the first part completes leaves a
    /// resumable artifact on disk.
    #[must_use]
    pub fn new(
        session_id: String,
        is_ru: bool,
        fingerprint: u64,
        single_pass: bool,
        sources: Vec<String>,
    ) -> Self {
        Self {
            session_id,
            created_ms: now_ms(),
            is_ru,
            fingerprint,
            single_pass,
            parts: sources
                .into_iter()
                .map(|source| ConspectPart {
                    source,
                    summary: None,
                })
                .collect(),
            final_summary: None,
        }
    }

    /// The non-empty part conspectuses, in order — the exact input a reduce-only
    /// re-run needs. Empty when nothing has been (successfully) mapped yet.
    #[must_use]
    pub fn usable_summaries(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|p| p.summary.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// True when at least one part has a usable conspectus (map-reduce) — i.e. a
    /// reduce can run without re-mapping. Always false for an empty single-pass
    /// (single-pass reduces from `parts[0].source`, not from a summary).
    #[must_use]
    pub fn has_usable_parts(&self) -> bool {
        !self.usable_summaries().is_empty()
    }

    /// Indices of parts still missing a summary (a retry re-maps these).
    #[must_use]
    pub fn missing_part_indices(&self) -> Vec<usize> {
        self.parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.summary.as_deref().map(str::trim).unwrap_or("").is_empty())
            .map(|(i, _)| i)
            .collect()
    }
}

/// Best-effort unix-ms (0 when the clock is before the epoch / unavailable).
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

/// Stable hash of the formatted transcript — the reuse key for a bar re-press.
#[must_use]
pub fn fingerprint(formatted: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    formatted.hash(&mut h);
    h.finish()
}

/// The conspects directory under the resolved data root, if resolvable.
fn conspects_dir() -> Option<PathBuf> {
    crate::paths::data_root().map(|root| root.join(DIR))
}

/// Reject anything that isn't a plain filename component (defence-in-depth — the
/// session id is a journal stem / archive id, already path-safe, but never let a
/// `..` / separator escape the conspects dir).
fn safe_stem(session_id: &str) -> Option<String> {
    let s = session_id.trim();
    if s.is_empty() || s.contains(['/', '\\', ':']) || s.contains("..") {
        return None;
    }
    // Must be a bare filename component (no parent dirs / specials).
    if Path::new(s).file_name() != Some(std::ffi::OsStr::new(s)) {
        return None;
    }
    Some(s.to_owned())
}

/// Persist the conspect atomically (tmp + rename, NTFS-atomic like config save),
/// then prune the directory to [`KEEP_NEWEST`]. Returns whether it was written;
/// a failure is logged (never panics) and degrades to the in-memory-only
/// behaviour rather than blocking the summary.
pub fn save(c: &Conspect) -> bool {
    match conspects_dir() {
        Some(dir) => save_in(&dir, c).is_ok(),
        None => {
            log::warn!("conspect save skipped: data root unresolved");
            false
        }
    }
}

/// Pure save (test seam): operate on an explicit directory.
fn save_in(dir: &Path, c: &Conspect) -> anyhow::Result<()> {
    let stem =
        safe_stem(&c.session_id).ok_or_else(|| anyhow::anyhow!("unsafe conspect session id"))?;
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{stem}.json"));
    let tmp = dir.join(format!("{stem}.json.tmp"));
    let bytes = serde_json::to_vec_pretty(c)?;
    std::fs::write(&tmp, &bytes)?;
    // Atomic replace — same proven pattern as config::save (MoveFileEx replace).
    std::fs::rename(&tmp, &path)?;
    prune_in(dir, KEEP_NEWEST);
    Ok(())
}

/// Load a session's conspect, or `None` if absent / unreadable / corrupt
/// (treated as "no conspect" — the caller rebuilds).
#[must_use]
pub fn load(session_id: &str) -> Option<Conspect> {
    let dir = conspects_dir()?;
    load_in(&dir, session_id)
}

/// Pure load (test seam).
fn load_in(dir: &Path, session_id: &str) -> Option<Conspect> {
    let stem = safe_stem(session_id)?;
    let path = dir.join(format!("{stem}.json"));
    let bytes = std::fs::read(&path).ok()?;
    match serde_json::from_slice::<Conspect>(&bytes) {
        Ok(c) => Some(c),
        Err(e) => {
            log::warn!("conspect {stem}: parse failed ({e}); ignoring");
            None
        }
    }
}

/// True if a session has a saved conspect sidecar — a CHEAP path check (no read /
/// parse). Used to gate the "re-create summary?" confirm (F3): a present file
/// means a summary was already built, so overwriting needs confirmation. A
/// corrupt-but-present file still counts (the user saw a summary).
#[must_use]
pub fn exists(session_id: &str) -> bool {
    conspects_dir().is_some_and(|dir| exists_in(&dir, session_id))
}

/// Pure existence check (test seam).
fn exists_in(dir: &Path, session_id: &str) -> bool {
    match safe_stem(session_id) {
        Some(stem) => dir.join(format!("{stem}.json")).exists(),
        None => false,
    }
}

/// Delete a session's conspect sidecar (and any stale `.tmp`). Idempotent +
/// safe-stem guarded; returns true if the main `.json` was removed.
pub fn delete(session_id: &str) -> bool {
    match conspects_dir() {
        Some(dir) => delete_in(&dir, session_id),
        None => false,
    }
}

/// Pure delete (test seam).
fn delete_in(dir: &Path, session_id: &str) -> bool {
    let Some(stem) = safe_stem(session_id) else {
        return false;
    };
    let _ = std::fs::remove_file(dir.join(format!("{stem}.json.tmp")));
    std::fs::remove_file(dir.join(format!("{stem}.json"))).is_ok()
}

/// Keep the newest `keep` `*.json` conspects by mtime (falling back to name
/// order when mtime is unreadable); delete the rest. Best-effort, never panics.
fn prune_in(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let mtime = p
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            (mtime, p)
        })
        .collect();
    if files.len() <= keep {
        return;
    }
    // Newest first; drop everything past `keep`.
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (_, path) in files.into_iter().skip(keep) {
        if let Err(e) = std::fs::remove_file(&path) {
            log::debug!("conspect prune: could not remove {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn sample(id: &str) -> Conspect {
        Conspect::new(
            id.to_owned(),
            true,
            fingerprint("formatted transcript"),
            false,
            vec!["part one source".into(), "part two source".into()],
        )
    }

    #[test]
    fn save_load_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let mut c = sample("session_123");
        c.parts[0].summary = Some("- topic A\n- decision B".into());
        c.final_summary = Some("final recap".into());
        save_in(tmp.path(), &c).unwrap();
        let back = load_in(tmp.path(), "session_123").expect("loads back");
        assert_eq!(back, c);
    }

    #[test]
    fn load_missing_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_in(tmp.path(), "nope").is_none());
    }

    #[test]
    fn exists_in_reflects_file_presence() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!exists_in(tmp.path(), "session_123")); // absent → false
        save_in(tmp.path(), &sample("session_123")).unwrap();
        assert!(exists_in(tmp.path(), "session_123")); // present → true
        assert!(!exists_in(tmp.path(), "other")); // a different id → false
        assert!(!exists_in(tmp.path(), "../escape")); // unsafe stem → false
    }

    #[test]
    fn usable_summaries_skips_none_and_blank() {
        let mut c = sample("s");
        c.parts[0].summary = Some("real conspectus".into());
        c.parts[1].summary = Some("   ".into()); // blank → not usable
        assert_eq!(c.usable_summaries(), vec!["real conspectus".to_string()]);
        assert!(c.has_usable_parts());
        assert_eq!(c.missing_part_indices(), vec![1]);
    }

    #[test]
    fn fingerprint_is_stable_and_distinguishes() {
        assert_eq!(fingerprint("abc"), fingerprint("abc"));
        assert_ne!(fingerprint("abc"), fingerprint("abd"));
    }

    #[test]
    fn safe_stem_rejects_traversal_and_separators() {
        assert!(safe_stem("session_2026").is_some());
        assert!(safe_stem("../escape").is_none());
        assert!(safe_stem("a/b").is_none());
        assert!(safe_stem("a\\b").is_none());
        assert!(safe_stem("c:evil").is_none());
        assert!(safe_stem("   ").is_none());
    }

    #[test]
    fn delete_in_removes_json_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("S1.json"), b"{}").unwrap();
        assert!(delete_in(dir, "S1")); // removed
        assert!(!dir.join("S1.json").exists());
        assert!(!delete_in(dir, "S1")); // already gone → false, no panic
        assert!(!delete_in(dir, "../escape")); // unsafe id rejected
    }

    #[test]
    fn prune_keeps_newest_n() {
        let tmp = tempfile::tempdir().unwrap();
        // Write 5 conspects; keep 2.
        for i in 0..5 {
            let c = sample(&format!("s{i}"));
            save_in(tmp.path(), &c).unwrap();
        }
        prune_in(tmp.path(), 2);
        let remaining = std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .count();
        assert_eq!(remaining, 2, "prune keeps exactly the newest 2");
    }

    #[test]
    fn incremental_save_overwrites_same_session() {
        let tmp = tempfile::tempdir().unwrap();
        let mut c = sample("dup");
        save_in(tmp.path(), &c).unwrap();
        c.parts[0].summary = Some("now mapped".into());
        save_in(tmp.path(), &c).unwrap();
        // One file, latest content.
        let n = std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .count();
        assert_eq!(n, 1, "same session id keeps a single sidecar");
        assert_eq!(
            load_in(tmp.path(), "dup").unwrap().parts[0]
                .summary
                .as_deref(),
            Some("now mapped")
        );
    }
}
