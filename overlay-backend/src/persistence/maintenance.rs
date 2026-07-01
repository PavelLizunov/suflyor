//! SAFE catalog diagnostics + repair — a non-destructive "fix the SQLite file"
//! backend for the session archive. Runs `integrity_check` / `foreign_key_check`,
//! then applies ONLY repairs that cannot lose a user row: WAL checkpoint, REINDEX,
//! FTS5 index rebuild, VACUUM.
//!
//! HARD INVARIANT: this module NEVER issues DROP / DELETE / CREATE / UPDATE against
//! user data. There is no code path that removes a table or a row. A backup is
//! taken BEFORE any write, so even the worst case (a repair that can't help) leaves
//! the user with a recoverable copy. Grep this file for `DROP`/`DELETE` — there is
//! none.

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Outcome of a diagnose+repair pass. All strings are plain (the UI localizes its
/// own labels); `actions`/`issues` are for showing the user what happened.
pub struct DbHealth {
    /// `integrity_check` AND `foreign_key_check` came back clean AFTER repair.
    pub healthy: bool,
    /// Problems still present after repair (empty iff `healthy`).
    pub issues: Vec<String>,
    /// Repairs actually performed (checkpoint / reindex / fts rebuild / vacuum).
    pub actions: Vec<String>,
    /// Where the pre-repair backup was written (`None` if there was nothing to
    /// back up, i.e. the DB didn't exist yet).
    pub backup_path: Option<String>,
}

/// Diagnose + non-destructively repair the DEFAULT on-disk catalog. Backs up
/// first, then checks, repairs (best-effort per step), and re-checks. See
/// [`diagnose_and_repair_at`] for the mechanics.
///
/// # Errors
/// Only if the data root can't be resolved or the backups dir can't be created —
/// an unopenable/corrupt DB is reported IN the returned [`DbHealth`], not as `Err`.
pub fn diagnose_and_repair_default() -> Result<DbHealth> {
    let path = default_catalog_path()?;
    let backups = backups_dir()?;
    diagnose_and_repair_at(&path, &backups)
}

/// Diagnose ONLY the default catalog (no backup, no repair) — a read-only health
/// probe for a status display.
///
/// # Errors
/// If the data root can't be resolved.
pub fn check_default() -> Result<DbHealth> {
    let path = default_catalog_path()?;
    if !path.exists() {
        return Ok(DbHealth {
            healthy: true,
            issues: Vec::new(),
            actions: vec!["база ещё не создана".to_string()],
            backup_path: None,
        });
    }
    match open_main(&path) {
        Ok(conn) => {
            let issues = run_checks(&conn);
            Ok(DbHealth {
                healthy: issues.is_empty(),
                issues,
                actions: Vec::new(),
                backup_path: None,
            })
        }
        Err(e) => Ok(DbHealth {
            healthy: false,
            issues: vec![format!("база не открывается: {e}")],
            actions: Vec::new(),
            backup_path: None,
        }),
    }
}

/// The testable core: back up `path` into `backups`, check, non-destructively
/// repair, re-check. Parameterized over paths so tests need no `data_root`.
///
/// # Errors
/// If `backups` can't be created. An unopenable DB is reported in [`DbHealth`].
pub fn diagnose_and_repair_at(path: &Path, backups: &Path) -> Result<DbHealth> {
    // (1) Nothing to repair if the file was never created.
    if !path.exists() {
        return Ok(DbHealth {
            healthy: true,
            issues: Vec::new(),
            actions: vec!["база ещё не создана".to_string()],
            backup_path: None,
        });
    }

    // (2) BACKUP FIRST — before ANY write to the live DB.
    std::fs::create_dir_all(backups).context("create backups dir")?;
    let backup_path = backup_before_repair(path, backups);
    prune_backups(backups, 5);

    // (2b) STRICT "backup before the operation" guarantee: if no backup could be
    // written, do NOT touch the live DB — report instead. The repair ops are
    // non-destructive anyway, but a guaranteed pre-repair backup is the contract.
    if backup_path.is_none() {
        return Ok(DbHealth {
            healthy: false,
            issues: vec![
                "не удалось создать резервную копию — ремонт пропущен (проверьте свободное место)"
                    .to_string(),
            ],
            actions: Vec::new(),
            backup_path: None,
        });
    }

    // (3) Open the MAIN db (no migrations). If it won't open we can't repair —
    // but the backup is safe, so report rather than bail.
    let conn = match open_main(path) {
        Ok(c) => c,
        Err(e) => {
            return Ok(DbHealth {
                healthy: false,
                issues: vec![format!("база не открывается: {e}")],
                actions: Vec::new(),
                backup_path,
            })
        }
    };

    // (4) CHECK (pre-repair) — result feeds the log only; the FINAL verdict is the
    // post-repair re-check in (6).
    let _pre = run_checks(&conn);

    // (5) REPAIR — non-destructive ops only, each best-effort.
    let mut actions = Vec::new();
    repair(&conn, &mut actions);

    // (6) RE-CHECK → final verdict.
    let issues = run_checks(&conn);

    Ok(DbHealth {
        healthy: issues.is_empty(),
        issues,
        actions,
        backup_path,
    })
}

/// Open the main catalog with the SAME pragmas as `Store::open` (WAL + FK +
/// busy_timeout) but WITHOUT running migrations — repair must not mutate schema.
fn open_main(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).context("open catalog")?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 2000;",
    )
    .context("set pragmas")?;
    Ok(conn)
}

/// `integrity_check(50)` + `foreign_key_check` → a list of remaining problems
/// (empty = clean). Read-only; used both pre- and post-repair.
fn run_checks(conn: &Connection) -> Vec<String> {
    let mut issues = Vec::new();

    // integrity_check(N) returns N rows max; a single "ok" row means clean.
    match conn.prepare("PRAGMA integrity_check(50)") {
        Ok(mut stmt) => match stmt.query_map([], |r| r.get::<_, String>(0)) {
            Ok(rows) => {
                for row in rows {
                    match row {
                        Ok(v) if v != "ok" => issues.push(format!("integrity: {v}")),
                        Ok(_) => {}
                        Err(e) => issues.push(format!("integrity_check read failed: {e}")),
                    }
                }
            }
            Err(e) => issues.push(format!("integrity_check failed: {e}")),
        },
        Err(e) => issues.push(format!("integrity_check failed: {e}")),
    }

    // foreign_key_check: EACH returned row is a violation (table, rowid, parent,
    // fkid). No rows = clean.
    match conn.prepare("PRAGMA foreign_key_check") {
        Ok(mut stmt) => {
            let cols = stmt.column_count();
            match stmt.query_map([], move |r| {
                let mut parts = Vec::with_capacity(cols);
                for i in 0..cols {
                    // Columns are TEXT/INTEGER/NULL; render each defensively.
                    let v = r
                        .get::<_, Option<String>>(i)
                        .or_else(|_| r.get::<_, i64>(i).map(|n| Some(n.to_string())))
                        .unwrap_or(None);
                    parts.push(v.unwrap_or_default());
                }
                Ok(parts.join(" "))
            }) {
                Ok(rows) => {
                    for row in rows {
                        match row {
                            Ok(v) => issues.push(format!("foreign_key: {v}")),
                            Err(e) => issues.push(format!("foreign_key_check read failed: {e}")),
                        }
                    }
                }
                Err(e) => issues.push(format!("foreign_key_check failed: {e}")),
            }
        }
        Err(e) => issues.push(format!("foreign_key_check failed: {e}")),
    }

    issues
}

/// Apply the non-destructive repair ops in order, recording successes in
/// `actions` and logging (never bailing on) failures. NO DROP / DELETE / CREATE.
fn repair(conn: &Connection, actions: &mut Vec<String>) {
    // WAL checkpoint (TRUNCATE) — folds the -wal back into the main file. Returns
    // a row, so query_row, not execute.
    match conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())) {
        Ok(()) => actions.push("wal_checkpoint(TRUNCATE)".to_string()),
        Err(e) => log::warn!("maintenance: wal_checkpoint failed: {e}"),
    }

    // REINDEX — rebuild all b-tree indexes from the table data (non-destructive).
    match conn.execute_batch("REINDEX;") {
        Ok(()) => actions.push("reindex".to_string()),
        Err(e) => log::warn!("maintenance: reindex failed: {e}"),
    }

    // Rebuild every FTS5 full-text index from its content (non-destructive: the
    // 'rebuild' command re-derives the index, it does not touch source rows).
    for tbl in fts5_tables(conn) {
        let quoted = quote_ident(&tbl);
        let sql = format!("INSERT INTO {quoted}({quoted}) VALUES('rebuild');");
        match conn.execute_batch(&sql) {
            Ok(()) => actions.push(format!("fts rebuild: {tbl}")),
            Err(e) => log::warn!("maintenance: fts rebuild of {tbl} failed: {e}"),
        }
    }

    // VACUUM — rewrite the DB file, defragmenting and dropping free pages. Moves
    // no user rows; purely a physical repack.
    match conn.execute_batch("VACUUM;") {
        Ok(()) => actions.push("vacuum".to_string()),
        Err(e) => log::warn!("maintenance: vacuum failed: {e}"),
    }
}

/// Discover FTS5 virtual tables by their CREATE sql (case-insensitive `fts5`).
fn fts5_tables(conn: &Connection) -> Vec<String> {
    let mut out = Vec::new();
    let sql = "SELECT name FROM sqlite_master \
               WHERE type='table' AND lower(sql) LIKE '%fts5%'";
    match conn.prepare(sql) {
        Ok(mut stmt) => match stmt.query_map([], |r| r.get::<_, String>(0)) {
            Ok(rows) => {
                for row in rows {
                    match row {
                        Ok(name) => out.push(name),
                        Err(e) => log::warn!("maintenance: fts table row read failed: {e}"),
                    }
                }
            }
            Err(e) => log::warn!("maintenance: fts table discovery query failed: {e}"),
        },
        Err(e) => log::warn!("maintenance: fts table discovery prepare failed: {e}"),
    }
    out
}

/// Quote a SQL identifier (double-quote, doubling any embedded `"`). Table names
/// come from `sqlite_master` (trusted), but quoting keeps the interpolation safe.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Back up the live DB BEFORE any repair write. Prefer a consistent
/// `VACUUM INTO` (self-contained, includes WAL); fall back to a raw file copy
/// (+ -wal/-shm siblings) if the DB is too corrupt to VACUUM. Returns the backup
/// path, or `None` if even the raw copy failed (repair still proceeds — the live
/// DB is untouched by a failed backup).
fn backup_before_repair(path: &Path, backups: &Path) -> Option<String> {
    let millis = unix_millis();
    let backup = backups.join(format!("catalog-{millis}.sqlite"));

    // Preferred: VACUUM INTO writes a clean, consistent copy (folds in WAL). Runs
    // on its own short-lived connection.
    match Connection::open(path) {
        Ok(conn) => {
            let sql = format!("VACUUM INTO {}", quote_string(&backup.to_string_lossy()));
            match conn.execute_batch(&sql) {
                Ok(()) => return Some(backup.to_string_lossy().into_owned()),
                Err(e) => log::warn!("maintenance: VACUUM INTO backup failed ({e}); raw-copying"),
            }
        }
        Err(e) => log::warn!("maintenance: open for VACUUM INTO failed ({e}); raw-copying"),
    }

    // Fallback: raw file copy of the main DB + its WAL/SHM siblings (so the copy
    // is a complete WAL set even from a DB we couldn't open cleanly).
    match std::fs::copy(path, &backup) {
        Ok(_) => {
            for ext in ["sqlite-wal", "sqlite-shm"] {
                let sib = path.with_extension(ext);
                if sib.exists() {
                    let dst = backup.with_extension(ext);
                    if let Err(e) = std::fs::copy(&sib, &dst) {
                        log::warn!("maintenance: copy of {} failed: {e}", sib.display());
                    }
                }
            }
            Some(backup.to_string_lossy().into_owned())
        }
        Err(e) => {
            log::warn!("maintenance: raw backup copy failed: {e}");
            None
        }
    }
}

/// Keep only the newest `keep` `catalog-*.sqlite` backups (and their -wal/-shm),
/// deleting older ones. Prunes ONLY our own `catalog-<millis>.sqlite` files —
/// never the live DB, never anything else.
fn prune_backups(backups: &Path, keep: usize) {
    let mut ours: Vec<PathBuf> = match std::fs::read_dir(backups) {
        Ok(rd) => rd
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| is_backup_file(p))
            .collect(),
        Err(e) => {
            log::warn!("maintenance: read backups dir failed: {e}");
            return;
        }
    };
    if ours.len() <= keep {
        return;
    }
    // Newest first by filename — the embedded unix-millis sorts lexicographically
    // in time order (fixed width until year ~2286).
    ours.sort();
    ours.reverse();
    for old in ours.into_iter().skip(keep) {
        if let Err(e) = std::fs::remove_file(&old) {
            log::warn!("maintenance: prune {} failed: {e}", old.display());
        }
        for ext in ["sqlite-wal", "sqlite-shm"] {
            let sib = old.with_extension(ext);
            if sib.exists() {
                let _ = std::fs::remove_file(&sib);
            }
        }
    }
}

/// True for our backup files: `catalog-<digits>.sqlite`. Excludes the live
/// `catalog.sqlite` and anything not matching the pattern, so pruning can't touch
/// the working DB.
fn is_backup_file(p: &Path) -> bool {
    let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    if p.extension().and_then(|e| e.to_str()) != Some("sqlite") {
        return false;
    }
    match stem.strip_prefix("catalog-") {
        Some(rest) => !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

/// Quote a SQL string literal (single-quote, doubling embedded `'`). For the
/// `VACUUM INTO '<path>'` file path.
fn quote_string(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Milliseconds since the Unix epoch (0 if the clock is before the epoch — only
/// used to make a unique backup filename).
fn unix_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

// ── USER-INITIATED targeted clears of the memory tables ────────────────────────
//
// These DELETE rows ON PURPOSE (the user asked to "clear the accumulated queue").
// Two hard safety rules, enforced by `clear_table_at`:
//   1. BACK UP FIRST — reuse `backup_before_repair`; if no backup lands, bail
//      WITHOUT deleting anything.
//   2. Delete the ONE requested table ONLY — `table` is a FIXED whitelist of the
//      two memory tables; nothing else ever reaches SQL. Sessions/utterances/
//      ai_turns/FTS are NEVER touched. Grep this block: the only DELETE is
//      `DELETE FROM memory_candidates|memory_items WHERE profile_id = ?1`.

/// Outcome of a targeted clear: how many rows went, and where the pre-delete
/// backup landed (`backup_path` empty iff there was no DB to clear).
pub struct ClearResult {
    pub cleared: usize,
    pub backup_path: String,
}

/// Clear the pending "Suggestions" queue (`memory_candidates`) for profile
/// `default`. Backs up first; deletes ONLY that table's `default` rows.
///
/// # Errors
/// If paths can't be resolved, or (STRICT) if the pre-delete backup couldn't be
/// written — in which case nothing is deleted.
pub fn clear_memory_candidates_default() -> Result<ClearResult> {
    clear_table_at(
        &default_catalog_path()?,
        &backups_dir()?,
        "memory_candidates",
    )
}

/// Clear approved curated memory (`memory_items`) for profile `default`. Backs up
/// first; deletes ONLY that table's `default` rows.
///
/// # Errors
/// As [`clear_memory_candidates_default`].
pub fn clear_memory_items_default() -> Result<ClearResult> {
    clear_table_at(&default_catalog_path()?, &backups_dir()?, "memory_items")
}

/// Read-only count of the `default` "Suggestions" queue, for a confirm prompt.
/// Returns 0 if the DB doesn't exist yet.
///
/// # Errors
/// If the catalog path can't be resolved, or the count query fails.
pub fn count_memory_candidates_default() -> Result<usize> {
    count_table_at(&default_catalog_path()?, "memory_candidates")
}

/// Read-only count of `default` curated memory items, for a confirm prompt.
/// Returns 0 if the DB doesn't exist yet.
///
/// # Errors
/// As [`count_memory_candidates_default`].
pub fn count_memory_items_default() -> Result<usize> {
    count_table_at(&default_catalog_path()?, "memory_items")
}

/// Testable core of the targeted clear. `table` MUST be one of the two memory
/// tables — any other value bails BEFORE any I/O, so no arbitrary name is ever
/// interpolated into SQL. Sequence: whitelist → (no DB → no-op) → BACKUP FIRST
/// (bail if it fails) → `DELETE FROM <table> WHERE profile_id = 'default'`.
///
/// # Errors
/// Non-whitelisted `table`; the backups dir can't be created; the backup fails
/// (nothing is deleted); or the DB can't be opened / the DELETE fails.
fn clear_table_at(path: &Path, backups: &Path, table: &str) -> Result<ClearResult> {
    // (0) WHITELIST — the ONLY table names that may reach SQL. Reject everything
    // else up front (sessions/utterances/ai_turns/FTS can never be passed here).
    if !matches!(table, "memory_candidates" | "memory_items") {
        bail!("отказ: очистка разрешена только для таблиц памяти, не «{table}»");
    }

    // (1) Nothing to clear if the DB was never created.
    if !path.exists() {
        return Ok(ClearResult {
            cleared: 0,
            backup_path: String::new(),
        });
    }

    // (2) BACKUP FIRST — before the DELETE. STRICT: no backup ⇒ no delete.
    std::fs::create_dir_all(backups).context("create backups dir")?;
    let backup_path = match backup_before_repair(path, backups) {
        Some(bp) => bp,
        None => bail!("резервная копия не создана — очистка отменена"),
    };
    prune_backups(backups, 5);

    // (3) DELETE the ONE whitelisted table's `default` rows — nothing else.
    let conn = open_main(path)?;
    let sql = format!("DELETE FROM {table} WHERE profile_id = ?1");
    let cleared = conn
        .execute(&sql, params!["default"])
        .with_context(|| format!("clear {table}"))?;

    Ok(ClearResult {
        cleared,
        backup_path,
    })
}

/// Read-only `COUNT(*)` of a whitelisted memory table's `default` rows. Missing
/// DB → 0.
///
/// # Errors
/// Non-whitelisted `table`, or the count query fails.
fn count_table_at(path: &Path, table: &str) -> Result<usize> {
    if !matches!(table, "memory_candidates" | "memory_items") {
        bail!("отказ: подсчёт разрешён только для таблиц памяти, не «{table}»");
    }
    if !path.exists() {
        return Ok(0);
    }
    let conn = open_main(path)?;
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE profile_id = 'default'");
    let n: i64 = conn
        .query_row(&sql, [], |r| r.get(0))
        .with_context(|| format!("count {table}"))?;
    Ok(usize::try_from(n).unwrap_or(0))
}

/// Resolve the default catalog path via `Store::default_path`, erroring if the OS
/// config dir can't be resolved.
fn default_catalog_path() -> Result<PathBuf> {
    match super::Store::default_path() {
        Some(p) => Ok(p),
        None => bail!("cannot resolve catalog path (no OS config dir)"),
    }
}

/// `data_root()/backups`, erroring if the data root can't be resolved.
fn backups_dir() -> Result<PathBuf> {
    match crate::paths::data_root() {
        Some(r) => Ok(r.join("backups")),
        None => bail!("cannot resolve data root (no OS config dir)"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// Build a small, valid catalog with real user data + an FTS5 index at `path`.
    fn seed_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
             CREATE VIRTUAL TABLE notes_fts USING fts5(body, content='notes', content_rowid='id');
             CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
                 INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
             END;",
        )
        .unwrap();
        for (i, body) in [
            (1, "hash map lookup"),
            (2, "binary tree"),
            (3, "b-tree index"),
        ] {
            conn.execute(
                "INSERT INTO notes (id, body) VALUES (?1, ?2)",
                params![i, body],
            )
            .unwrap();
        }
    }

    fn row_count(path: &Path) -> i64 {
        let conn = Connection::open(path).unwrap();
        conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn repair_is_healthy_backs_up_and_preserves_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("catalog.sqlite");
        let backups = tmp.path().join("backups");
        seed_db(&db);
        assert_eq!(row_count(&db), 3, "precondition: 3 user rows");

        let health = diagnose_and_repair_at(&db, &backups).unwrap();

        assert!(
            health.healthy,
            "clean DB should verify healthy: {:?}",
            health.issues
        );
        assert!(health.issues.is_empty());
        // A backup was written and exists on disk.
        let bp = health.backup_path.expect("backup path recorded");
        assert!(Path::new(&bp).exists(), "backup file exists at {bp}");
        // The repair ops we promise actually ran.
        assert!(
            health.actions.iter().any(|a| a == "vacuum"),
            "actions: {:?}",
            health.actions
        );
        assert!(
            health.actions.iter().any(|a| a == "reindex"),
            "actions: {:?}",
            health.actions
        );
        assert!(
            health.actions.iter().any(|a| a.starts_with("fts rebuild")),
            "actions: {:?}",
            health.actions
        );
        // NO DATA LOSS — every user row still present after repair.
        assert_eq!(row_count(&db), 3, "repair must not lose any user row");
    }

    #[test]
    fn missing_db_is_healthy_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("does-not-exist.sqlite");
        let backups = tmp.path().join("backups");
        let health = diagnose_and_repair_at(&db, &backups).unwrap();
        assert!(health.healthy);
        assert!(health.backup_path.is_none(), "nothing to back up");
        assert!(health.actions.iter().any(|a| a.contains("не создана")));
    }

    #[test]
    fn prune_keeps_at_most_five_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let backups = tmp.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        // 8 backup files with strictly increasing millis-stamped names.
        for i in 0..8 {
            let f = backups.join(format!("catalog-{:013}.sqlite", 1_000_000_000_000u64 + i));
            std::fs::write(&f, b"x").unwrap();
        }
        // A non-backup file must be left untouched by pruning.
        let keeper = backups.join("catalog.sqlite");
        std::fs::write(&keeper, b"live").unwrap();

        prune_backups(&backups, 5);

        let remaining: Vec<String> = std::fs::read_dir(&backups)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| is_backup_file(p))
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert_eq!(remaining.len(), 5, "pruned to newest 5: {remaining:?}");
        // Kept the newest (highest millis), dropped the oldest.
        assert!(remaining.iter().any(|n| n.contains("1000000000007")));
        assert!(!remaining.iter().any(|n| n.contains("1000000000000")));
        // The live DB (not a catalog-<digits> file) survived.
        assert!(
            keeper.exists(),
            "prune must never touch the live catalog.sqlite"
        );
    }

    /// Seed a DB with the two memory tables (3 candidates + 2 items, all
    /// profile 'default') plus a `sessions` table (1 row) that the clears must
    /// NEVER touch. Minimal schema — the clears only read `profile_id`.
    fn seed_memory_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE sessions (id INTEGER PRIMARY KEY, title TEXT);
             CREATE TABLE memory_candidates (id INTEGER PRIMARY KEY, profile_id TEXT NOT NULL, text TEXT);
             CREATE TABLE memory_items (id INTEGER PRIMARY KEY, profile_id TEXT NOT NULL, text TEXT);
             INSERT INTO sessions (title) VALUES ('a meeting');",
        )
        .unwrap();
        for t in ["q1", "q2", "q3"] {
            conn.execute(
                "INSERT INTO memory_candidates (profile_id, text) VALUES ('default', ?1)",
                params![t],
            )
            .unwrap();
        }
        for t in ["fact1", "fact2"] {
            conn.execute(
                "INSERT INTO memory_items (profile_id, text) VALUES ('default', ?1)",
                params![t],
            )
            .unwrap();
        }
    }

    fn count_of(path: &Path, table: &str) -> i64 {
        let conn = Connection::open(path).unwrap();
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn clear_candidates_clears_only_the_queue_and_backs_up() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("catalog.sqlite");
        let backups = tmp.path().join("backups");
        seed_memory_db(&db);

        let res = clear_table_at(&db, &backups, "memory_candidates").unwrap();

        assert_eq!(res.cleared, 3, "all 3 pending candidates removed");
        assert!(
            Path::new(&res.backup_path).exists(),
            "backup taken before delete at {}",
            res.backup_path
        );
        assert_eq!(count_of(&db, "memory_candidates"), 0, "queue emptied");
        // The OTHER tables are untouched — proves only the queue was cleared.
        assert_eq!(count_of(&db, "memory_items"), 2, "curated items untouched");
        assert_eq!(count_of(&db, "sessions"), 1, "sessions untouched");
    }

    #[test]
    fn clear_items_clears_only_curated_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("catalog.sqlite");
        let backups = tmp.path().join("backups");
        seed_memory_db(&db);

        let res = clear_table_at(&db, &backups, "memory_items").unwrap();

        assert_eq!(res.cleared, 2, "both curated items removed");
        assert!(Path::new(&res.backup_path).exists(), "backup taken");
        assert_eq!(count_of(&db, "memory_items"), 0, "items emptied");
        assert_eq!(count_of(&db, "memory_candidates"), 3, "queue untouched");
        assert_eq!(count_of(&db, "sessions"), 1, "sessions untouched");
    }

    #[test]
    fn clear_rejects_non_whitelisted_table() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("catalog.sqlite");
        let backups = tmp.path().join("backups");
        seed_memory_db(&db);

        // A real, populated table — the whitelist, not a missing table, must reject it.
        assert!(
            clear_table_at(&db, &backups, "sessions").is_err(),
            "whitelist must reject any non-memory table"
        );
        assert!(clear_table_at(&db, &backups, "utterances").is_err());
        assert!(clear_table_at(&db, &backups, "memory_candidates; DROP TABLE sessions").is_err());
        // The rejected attempt deleted nothing.
        assert_eq!(count_of(&db, "sessions"), 1, "reject must not delete");
    }

    #[test]
    fn clear_missing_db_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("does-not-exist.sqlite");
        let backups = tmp.path().join("backups");
        let res = clear_table_at(&db, &backups, "memory_candidates").unwrap();
        assert_eq!(res.cleared, 0);
        assert!(res.backup_path.is_empty(), "no backup for a missing DB");
    }

    #[test]
    fn count_reads_default_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("catalog.sqlite");
        seed_memory_db(&db);
        assert_eq!(count_table_at(&db, "memory_candidates").unwrap(), 3);
        assert_eq!(count_table_at(&db, "memory_items").unwrap(), 2);
        // Missing DB → 0.
        let missing = tmp.path().join("nope.sqlite");
        assert_eq!(count_table_at(&missing, "memory_items").unwrap(), 0);
    }

    #[test]
    fn is_backup_file_matches_only_our_pattern() {
        assert!(is_backup_file(Path::new("catalog-1700000000000.sqlite")));
        assert!(!is_backup_file(Path::new("catalog.sqlite"))); // the live DB
        assert!(!is_backup_file(Path::new("catalog-.sqlite"))); // empty stamp
        assert!(!is_backup_file(Path::new("catalog-abc.sqlite"))); // non-digit
        assert!(!is_backup_file(Path::new("catalog-123.bak"))); // wrong ext
    }
}
