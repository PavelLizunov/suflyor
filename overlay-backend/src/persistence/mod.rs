//! Session catalog persistence — a REBUILDABLE SQLite index over the append-only
//! JSONL journals ("Phase 2 — SQLite catalog" of
//! `docs/personal-memory-and-session-store-architecture.md`).
//!
//! Two layers, by design:
//! - JSONL ([`crate::journal`]) stays the primary append-only event log — cheap,
//!   crash-proof, human-readable, the source of truth.
//! - SQLite ([`Store`]) is a queryable PROJECTION for the session archive +
//!   search, so the live audio / AI pipeline never depends on its speed. It is
//!   rebuilt by re-indexing the journals — but NOTE (fs-audit #2): the indexer
//!   is additive (it never deletes session rows on its own), so once a journal
//!   is pruned from disk under journal retention, its catalog row becomes the
//!   LAST surviving copy of that session's transcript + AI turns. The catalog
//!   therefore deliberately preserves archive history PAST the raw-journal
//!   retention; manually deleting `catalog.sqlite` and rebuilding would drop
//!   those pruned-journal sessions (their journals are already gone). This is a
//!   feature, not drift — the ~few-MB catalog is the long-term searchable
//!   history while the bulky raw journals/audio rotate out.
//!
//! Callers see only owned row types ([`Session`] / [`Utterance`] / [`AiTurn`])
//! and [`Store`] — no `rusqlite` types or raw SQL leak out. The JSONL→SQLite
//! indexer + FTS search land in follow-up commits.

mod indexer;
pub mod maintenance;
mod migrations;
pub mod models;
mod sqlite_store;

pub use indexer::{index_all, index_journal_file, IndexStats};
pub use models::{
    AiTurn, MemoryCandidate, MemoryItem, NewMemoryCandidate, NewMemoryItem, SearchHit, Session,
    Utterance,
};
pub use sqlite_store::Store;

use anyhow::{Context, Result};
use std::path::PathBuf;

/// `%APPDATA%\suflyor\sessions` (legacy `overlay-mvp` until migrated) — where the
/// JSONL journals are written. Routed through [`crate::paths::data_root`] so it
/// stays in lock-step with the writer's `journal::sessions_dir`.
#[must_use]
pub fn sessions_dir() -> Option<PathBuf> {
    crate::paths::data_root().map(|r| r.join("sessions"))
}

/// One-call startup entry point: open the default catalog and idempotently index
/// every finished JSONL session under the default `sessions/` dir. `skip_active`
/// is the live session id to skip (its file is still being written), or `None` at
/// app launch. Meant to run OFF the hot path (the caller spawns it on a detached
/// thread) so the live audio / AI pipeline never waits on it.
pub fn reindex_default(skip_active: Option<&str>) -> Result<IndexStats> {
    let db = Store::default_path().context("resolve catalog path")?;
    let sessions = sessions_dir().context("resolve sessions dir")?;
    let mut store = Store::open(&db)?;
    let stats = index_all(&mut store, &sessions, skip_active)?;
    // v0.22.x — recompute every session's headline model from what its AI turns
    // actually ran on (a turnless session → NULL → "—"). Best-effort: a stale
    // model is cosmetic, never fail the catalog refresh over it. Idempotent.
    if let Err(e) = store.backfill_session_models() {
        log::warn!("reindex: session-model backfill failed: {e:#}");
    }
    Ok(stats)
}

/// Open the default on-disk catalog (the same `catalog.sqlite` the startup
/// indexer writes) for the UI READ paths — the session-archive list + FTS
/// search. Opening runs the migrations + WAL setup, so the caller holds the
/// returned [`Store`] for one browse session (reusing it across list / search /
/// detail queries) instead of reopening per query. `Err` if the OS config dir
/// can't be resolved or the open fails — the caller surfaces an "archive
/// unavailable" state rather than crashing. The archive is read-only: it never
/// writes, so it can race the indexer harmlessly under WAL.
pub fn open_default_store() -> Result<Store> {
    let db = Store::default_path().context("resolve catalog path")?;
    Store::open(&db)
}
