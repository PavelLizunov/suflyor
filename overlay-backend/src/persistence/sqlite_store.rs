//! The SQLite session catalog — connection, schema lifecycle, and the
//! idempotent (re)index + read queries. A REBUILDABLE index over the JSONL
//! journals: drop the file and re-index from the raw events with no loss.
//!
//! Not shared across threads: the indexer opens its own `Store` on a detached
//! worker (a `rusqlite::Connection` is `!Sync`), so the live audio / AI pipeline
//! never blocks on SQLite (architecture-doc invariant).

use anyhow::{Context, Result};
use rusqlite::{params, Connection, Row};
use std::path::{Path, PathBuf};

use super::migrations;
use super::models::{AiTurn, Session, Utterance};

/// The catalog handle. Owns one SQLite connection.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Default catalog location: `%APPDATA%\overlay-mvp\catalog.sqlite`, next to
    /// the `sessions/` JSONL dir. `None` if the OS config dir can't be resolved.
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("overlay-mvp").join("catalog.sqlite"))
    }

    /// Open (creating if absent) the on-disk catalog: ensure the parent dir,
    /// enable WAL + foreign keys, back up the file if a schema migration is
    /// pending, then migrate to [`migrations::LATEST_VERSION`].
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create catalog dir")?;
        }
        let preexisting = path.exists();
        let mut conn = Connection::open(path).context("open catalog")?;
        // WAL keeps reads non-blocking while the indexer writes; foreign keys
        // power the ON DELETE CASCADE that makes re-indexing clean.
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .context("set pragmas")?;
        if preexisting {
            let current: i32 = conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .context("read user_version")?;
            if current < migrations::LATEST_VERSION {
                // Recoverable backup BEFORE migrating (doc rule). Checkpoint WAL
                // first so the copied main file is self-contained. Best-effort:
                // a failed backup must not block opening a usable catalog.
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                let bak = path.with_extension("sqlite.bak");
                if let Err(e) = std::fs::copy(path, &bak) {
                    log::warn!("catalog pre-migration backup failed: {e}");
                }
            }
        }
        migrations::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// In-memory catalog (tests + ephemeral use): same schema, no file, no WAL.
    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory().context("open in-memory catalog")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .context("enable foreign_keys")?;
        migrations::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Idempotently (re)index one session: replace its row plus ALL its
    /// utterances / ai_turns in a single transaction. Re-running with the same
    /// `session.id` overwrites — so re-indexing a JSONL never duplicates rows
    /// (ON DELETE CASCADE clears the children when the session row is removed).
    pub fn replace_session(
        &mut self,
        session: &Session,
        utterances: &[Utterance],
        ai_turns: &[AiTurn],
    ) -> Result<()> {
        let tx = self.conn.transaction().context("begin index tx")?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", params![session.id])
            .context("clear prior session rows")?;
        tx.execute(
            "INSERT INTO sessions (id, journal_path, started_at_ms, finished_at_ms, status, \
             ai_model, transcript_lines, ai_turns_count, total_cost_microcents, indexed_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                session.id,
                session.journal_path,
                session.started_at_ms,
                session.finished_at_ms,
                session.status,
                session.ai_model,
                session.transcript_lines,
                session.ai_turns_count,
                session.total_cost_microcents,
                session.indexed_at_ms,
            ],
        )
        .context("insert session")?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO utterances (session_id, unix_ms, source, text) VALUES (?1, ?2, ?3, ?4)")
                .context("prepare utterance insert")?;
            for u in utterances {
                stmt.execute(params![u.session_id, u.unix_ms, u.source, u.text])
                    .context("insert utterance")?;
            }
        }
        {
            let mut stmt = tx
                .prepare("INSERT INTO ai_turns (session_id, unix_ms, purpose, model, question, \
                          answer, latency_ms, attached_screenshot) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
                .context("prepare ai_turn insert")?;
            for t in ai_turns {
                stmt.execute(params![
                    t.session_id,
                    t.unix_ms,
                    t.purpose,
                    t.model,
                    t.question,
                    t.answer,
                    t.latency_ms,
                    t.attached_screenshot,
                ])
                .context("insert ai_turn")?;
            }
        }
        tx.commit().context("commit index tx")?;
        Ok(())
    }

    /// All sessions, newest first (by `started_at_ms`, NULLs last). Powers the
    /// archive list.
    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, journal_path, started_at_ms, finished_at_ms, status, ai_model, \
                 transcript_lines, ai_turns_count, total_cost_microcents, indexed_at_ms \
                 FROM sessions ORDER BY started_at_ms DESC NULLS LAST, id DESC",
            )
            .context("prepare list_sessions")?;
        let rows = stmt
            .query_map([], row_to_session)
            .context("query list_sessions")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map session row")?);
        }
        Ok(out)
    }

    /// One session by id, or `None` if it isn't indexed.
    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, journal_path, started_at_ms, finished_at_ms, status, ai_model, \
                 transcript_lines, ai_turns_count, total_cost_microcents, indexed_at_ms \
                 FROM sessions WHERE id = ?1",
            )
            .context("prepare get_session")?;
        let mut rows = stmt
            .query_map(params![id], row_to_session)
            .context("query get_session")?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("map session row")?)),
            None => Ok(None),
        }
    }

    /// Utterance count for a session (cheap; used by the session-detail view +
    /// tests).
    pub fn count_utterances(&self, session_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM utterances WHERE session_id = ?1",
                params![session_id],
                |r| r.get(0),
            )
            .context("count utterances")
    }

    /// AI-turn count for a session.
    pub fn count_ai_turns(&self, session_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM ai_turns WHERE session_id = ?1",
                params![session_id],
                |r| r.get(0),
            )
            .context("count ai_turns")
    }

    /// The set of session ids already in the catalog — lets the indexer skip
    /// immutable, already-indexed journals.
    pub fn indexed_session_ids(&self) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM sessions")
            .context("prepare indexed ids")?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .context("query indexed ids")?;
        let mut set = std::collections::HashSet::new();
        for r in rows {
            set.insert(r.context("map id")?);
        }
        Ok(set)
    }

    /// All utterances for a session, time-ordered (the session-detail view).
    pub fn session_utterances(&self, session_id: &str) -> Result<Vec<Utterance>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, unix_ms, source, text FROM utterances \
                 WHERE session_id = ?1 ORDER BY unix_ms ASC, id ASC",
            )
            .context("prepare session_utterances")?;
        let rows = stmt
            .query_map(params![session_id], row_to_utterance)
            .context("query session_utterances")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map utterance")?);
        }
        Ok(out)
    }

    /// All AI turns for a session, time-ordered.
    pub fn session_ai_turns(&self, session_id: &str) -> Result<Vec<AiTurn>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, unix_ms, purpose, model, question, answer, latency_ms, \
                 attached_screenshot FROM ai_turns WHERE session_id = ?1 ORDER BY unix_ms ASC, id ASC",
            )
            .context("prepare session_ai_turns")?;
        let rows = stmt
            .query_map(params![session_id], row_to_ai_turn)
            .context("query session_ai_turns")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map ai_turn")?);
        }
        Ok(out)
    }
}

/// Map a `sessions` row (column order matches the SELECTs above) to a [`Session`].
fn row_to_session(row: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        journal_path: row.get(1)?,
        started_at_ms: row.get(2)?,
        finished_at_ms: row.get(3)?,
        status: row.get(4)?,
        ai_model: row.get(5)?,
        transcript_lines: row.get(6)?,
        ai_turns_count: row.get(7)?,
        total_cost_microcents: row.get(8)?,
        indexed_at_ms: row.get(9)?,
    })
}

/// Map an `utterances` row (column order matches the SELECTs above).
fn row_to_utterance(row: &Row) -> rusqlite::Result<Utterance> {
    Ok(Utterance {
        session_id: row.get(0)?,
        unix_ms: row.get(1)?,
        source: row.get(2)?,
        text: row.get(3)?,
    })
}

/// Map an `ai_turns` row (column order matches the SELECTs above).
fn row_to_ai_turn(row: &Row) -> rusqlite::Result<AiTurn> {
    Ok(AiTurn {
        session_id: row.get(0)?,
        unix_ms: row.get(1)?,
        purpose: row.get(2)?,
        model: row.get(3)?,
        question: row.get(4)?,
        answer: row.get(5)?,
        latency_ms: row.get(6)?,
        attached_screenshot: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn sample_session(id: &str) -> Session {
        Session {
            id: id.to_string(),
            journal_path: format!("C:/sessions/{id}.jsonl"),
            started_at_ms: Some(1_000),
            finished_at_ms: Some(2_000),
            status: "completed".into(),
            ai_model: Some("gemma-4-E4B".into()),
            transcript_lines: 2,
            ai_turns_count: 1,
            total_cost_microcents: 0,
            indexed_at_ms: 3_000,
        }
    }

    fn utt(id: &str, ms: i64, source: &str, text: &str) -> Utterance {
        Utterance {
            session_id: id.into(),
            unix_ms: ms,
            source: source.into(),
            text: text.into(),
        }
    }

    #[test]
    fn open_in_memory_migrates_to_latest() {
        let store = Store::open_in_memory().unwrap();
        let v: i32 = store
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, migrations::LATEST_VERSION);
    }

    #[test]
    fn replace_session_round_trips() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("2026-06-04_10-00-00_ab12");
        let utts = vec![
            utt(&s.id, 1, "mic", "what is a hash map"),
            utt(&s.id, 2, "system", "interviewer speaks"),
        ];
        let turns = vec![AiTurn {
            session_id: s.id.clone(),
            unix_ms: 3,
            purpose: "live_ask".into(),
            model: "gemma-4-E4B".into(),
            question: "what is a hash map".into(),
            answer: "a key-value structure".into(),
            latency_ms: Some(120),
            attached_screenshot: false,
        }];
        store.replace_session(&s, &utts, &turns).unwrap();

        let got = store.get_session(&s.id).unwrap().unwrap();
        assert_eq!(got, s);
        assert_eq!(store.count_utterances(&s.id).unwrap(), 2);
        assert_eq!(store.count_ai_turns(&s.id).unwrap(), 1);
    }

    #[test]
    fn reindex_is_idempotent_no_duplicates() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("sess-1");
        let utts = vec![utt(&s.id, 1, "mic", "a"), utt(&s.id, 2, "mic", "b")];
        store.replace_session(&s, &utts, &[]).unwrap();
        // Re-index the SAME session — counts must stay, not double.
        store.replace_session(&s, &utts, &[]).unwrap();
        assert_eq!(store.count_utterances(&s.id).unwrap(), 2);
        assert_eq!(store.list_sessions().unwrap().len(), 1);
    }

    #[test]
    fn list_sessions_orders_newest_first() {
        let mut store = Store::open_in_memory().unwrap();
        let mut older = sample_session("old");
        older.started_at_ms = Some(100);
        let mut newer = sample_session("new");
        newer.started_at_ms = Some(900);
        store.replace_session(&older, &[], &[]).unwrap();
        store.replace_session(&newer, &[], &[]).unwrap();
        let ids: Vec<String> = store
            .list_sessions()
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec!["new".to_string(), "old".to_string()]);
    }

    #[test]
    fn get_missing_session_is_none() {
        let store = Store::open_in_memory().unwrap();
        assert!(store.get_session("nope").unwrap().is_none());
    }
}
