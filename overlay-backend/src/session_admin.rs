//! Hard-delete every artifact of an archived session — the user-initiated
//! "delete from archive" (ТЗ2a). The SQLite catalog deliberately outlives
//! journal/audio retention (see the `persistence` module docs), so an explicit
//! user delete must reach BOTH the on-disk artifacts AND the catalog row.
//!
//! Order is load-bearing: filesystem artifacts FIRST, the catalog row LAST. The
//! catalog row is what lists a session in the archive, so it is removed ONLY once
//! every file is gone — a locked file leaves the row, the user sees an error, and
//! a retry (idempotent: a missing artifact is success) finishes the job. That is
//! the "no half-deleted session" + "retry completes it" contract.

use crate::persistence::Store;
use anyhow::{bail, Result};
use std::path::Path;

/// A session id is a journal file stem (e.g. `2026-06-04_10-00-00_ab12`). Reject
/// anything that could escape the data dirs (defense-in-depth before we join it
/// onto `sessions/` / `recordings/`).
fn is_safe_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && !session_id.contains('/')
        && !session_id.contains('\\')
        && !session_id.contains("..")
        // The id must be EXACTLY its own file name. This rejects "." (which would
        // make `<recordings_dir>.join(".")` resolve to the recordings ROOT and
        // `remove_dir_all` wipe every session's audio) and any other component
        // trickery. Mirrors conspect::safe_stem's round-trip guard.
        && Path::new(session_id).file_name() == Some(std::ffi::OsStr::new(session_id))
}

/// Remove `<sessions_dir>/<id>.jsonl` and `<recordings_dir>/<id>/`, idempotently
/// (a missing artifact is success). Both are attempted even if the first errors;
/// the first real IO error (e.g. a file held open) is returned, so the caller can
/// leave the catalog row in place and retry. Test seam for
/// [`delete_session_everywhere`].
///
/// # Errors
/// The first non-`NotFound` filesystem error encountered.
pub fn delete_session_files_in(
    sessions_dir: &Path,
    recordings_dir: &Path,
    session_id: &str,
) -> Result<()> {
    if !is_safe_id(session_id) {
        bail!("unsafe session id: {session_id:?}");
    }
    let mut first_err: Option<anyhow::Error> = None;

    let jsonl = sessions_dir.join(format!("{session_id}.jsonl"));
    match std::fs::remove_file(&jsonl) {
        Ok(()) => log::info!("session-delete: removed journal {}", jsonl.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            log::warn!("session-delete: journal {} failed: {e}", jsonl.display());
            first_err.get_or_insert_with(|| anyhow::anyhow!("delete journal: {e}"));
        }
    }

    let rec = recordings_dir.join(session_id);
    match std::fs::remove_dir_all(&rec) {
        Ok(()) => log::info!("session-delete: removed recordings {}", rec.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            log::warn!("session-delete: recordings {} failed: {e}", rec.display());
            first_err.get_or_insert_with(|| anyhow::anyhow!("delete recordings: {e}"));
        }
    }

    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Hard-delete EVERY artifact of `session_id`: the journal `.jsonl`, the
/// `recordings/<id>/` audio, the conspect sidecar, and the catalog row (cascading
/// to its utterances / ai_turns / FTS rows). Idempotent. Filesystem first,
/// catalog last (see module docs).
///
/// The caller MUST ensure this is not the live/active session — a session still
/// being written is not guarded here.
///
/// # Errors
/// The first filesystem error (leaving the catalog row intact for a retry), or a
/// catalog-delete error.
pub fn delete_session_everywhere(store: &mut Store, session_id: &str) -> Result<()> {
    if !is_safe_id(session_id) {
        bail!("unsafe session id: {session_id:?}");
    }
    let sessions_dir = crate::journal::sessions_dir()?;
    let recordings_dir = crate::recorder::recordings_dir()?;
    delete_session_files_in(&sessions_dir, &recordings_dir, session_id)?;
    // Conspect + debrief sidecars: self-guarded + idempotent, carry no data the
    // journal lacks — best-effort, never block the catalog delete.
    crate::conspect::delete(session_id);
    crate::conspect::delete_debrief(session_id);
    // Catalog row LAST — only reached when every FS artifact above is gone.
    store.delete_session(session_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn rejects_unsafe_ids() {
        let d = std::env::temp_dir();
        for bad in ["../escape", "a/b", "a\\b", "", ".", "..", "./x"] {
            assert!(
                delete_session_files_in(&d, &d, bad).is_err(),
                "expected reject: {bad:?}"
            );
        }
    }

    #[test]
    fn deletes_jsonl_and_recordings_then_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let sessions = tmp.path().join("sessions");
        let recordings = tmp.path().join("recordings");
        std::fs::create_dir_all(&sessions).unwrap();
        std::fs::create_dir_all(recordings.join("S1")).unwrap();
        std::fs::write(sessions.join("S1.jsonl"), b"{}").unwrap();
        std::fs::write(recordings.join("S1").join("mic.wav"), b"x").unwrap();

        delete_session_files_in(&sessions, &recordings, "S1").unwrap();
        assert!(!sessions.join("S1.jsonl").exists());
        assert!(!recordings.join("S1").exists());

        // Retry on a fully-cleaned session is still Ok (idempotent).
        delete_session_files_in(&sessions, &recordings, "S1").unwrap();
    }

    #[test]
    fn missing_artifacts_are_ok() {
        let tmp = tempfile::tempdir().unwrap();
        delete_session_files_in(
            &tmp.path().join("sessions"),
            &tmp.path().join("recordings"),
            "S2",
        )
        .unwrap();
    }
}
