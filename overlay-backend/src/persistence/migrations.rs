//! Declarative schema migrations for the catalog. The schema lives in immutable
//! SQL files under `overlay-backend/migrations/`, embedded here via `include_str!`
//! and versioned with SQLite's `PRAGMA user_version` (per the architecture doc).
//!
//! Rules: applied migration files are NEVER edited — a change is a NEW file + a
//! new entry below. Each migration runs in its own transaction, so a failure
//! rolls back and leaves the DB at the last good version (the on-disk backup is
//! taken by [`super::sqlite_store::Store::open`] BEFORE migrating).

use anyhow::{Context, Result};
use rusqlite::Connection;

/// The newest schema version this build knows how to produce.
pub(crate) const LATEST_VERSION: i32 = 4;

/// Ordered `(target_user_version, sql)` migrations. Index = order applied.
const MIGRATIONS: &[(i32, &str)] = &[
    (1, include_str!("../../migrations/0001_session_catalog.sql")),
    (2, include_str!("../../migrations/0002_fts.sql")),
    (3, include_str!("../../migrations/0003_memory.sql")),
    (
        4,
        include_str!("../../migrations/0004_utterance_audio_ms.sql"),
    ),
];

/// Apply every migration newer than the DB's current `user_version`, each in its
/// own transaction, bumping `user_version` on success. Idempotent: a DB already
/// at the latest version is left untouched. Returns the version after migrating.
pub(crate) fn run_migrations(conn: &mut Connection) -> Result<i32> {
    let current: i32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .context("read user_version")?;
    let mut at = current;
    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction().context("begin migration tx")?;
        tx.execute_batch(sql)
            .with_context(|| format!("apply migration {version}"))?;
        // `user_version` takes a literal, not a bound param; `version` is a
        // trusted in-crate constant, so the format is safe.
        tx.execute_batch(&format!("PRAGMA user_version = {version};"))
            .with_context(|| format!("bump user_version to {version}"))?;
        tx.commit()
            .with_context(|| format!("commit migration {version}"))?;
        at = *version;
    }
    Ok(at)
}
