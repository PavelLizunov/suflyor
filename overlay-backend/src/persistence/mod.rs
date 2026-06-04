//! Session catalog persistence — a REBUILDABLE SQLite index over the append-only
//! JSONL journals ("Phase 2 — SQLite catalog" of
//! `docs/personal-memory-and-session-store-architecture.md`).
//!
//! Two layers, by design:
//! - JSONL ([`crate::journal`]) stays the primary append-only event log — cheap,
//!   crash-proof, human-readable, the source of truth.
//! - SQLite ([`Store`]) is a queryable PROJECTION for the session archive +
//!   search. It can be deleted and rebuilt from the journals with no data loss,
//!   so the live audio / AI pipeline never depends on its speed.
//!
//! Callers see only owned row types ([`Session`] / [`Utterance`] / [`AiTurn`])
//! and [`Store`] — no `rusqlite` types or raw SQL leak out. The JSONL→SQLite
//! indexer + FTS search land in follow-up commits.

mod indexer;
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

/// `%APPDATA%\overlay-mvp\sessions` — where the JSONL journals are written.
#[must_use]
pub fn sessions_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("overlay-mvp").join("sessions"))
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
    index_all(&mut store, &sessions, skip_active)
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
