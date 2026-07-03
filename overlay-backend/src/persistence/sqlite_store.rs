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
use super::models::{
    AiTurn, MemoryCandidate, MemoryItem, NewMemoryCandidate, NewMemoryItem, SearchHit, Session,
    Utterance,
};

/// The catalog handle. Owns one SQLite connection.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Default catalog location: `%APPDATA%\suflyor\catalog.sqlite` (legacy
    /// `overlay-mvp` until migrated), next to the `sessions/` JSONL dir. `None`
    /// if the OS config dir can't be resolved.
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        crate::paths::data_root().map(|r| r.join("catalog.sqlite"))
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
        // busy_timeout (v0.17.2): the stop-session indexer thread and the
        // archive-open sweep can write concurrently (stop → immediate F7);
        // WAL allows ONE writer, and without a timeout the loser gets an
        // instant SQLITE_BUSY instead of waiting out the ~ms-long window.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 2000;",
        )
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
        // The FTS5 index isn't FK-cascaded; clear this session's rows explicitly
        // (the AFTER INSERT triggers repopulate it as the rows below re-insert).
        tx.execute(
            "DELETE FROM search_index WHERE session_id = ?1",
            params![session.id],
        )
        .context("clear prior search rows")?;
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
                .prepare("INSERT INTO utterances (session_id, unix_ms, source, text, audio_ms) VALUES (?1, ?2, ?3, ?4, ?5)")
                .context("prepare utterance insert")?;
            for u in utterances {
                stmt.execute(params![
                    u.session_id,
                    u.unix_ms,
                    u.source,
                    u.text,
                    u.audio_ms
                ])
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

    /// Hard-delete a session from the catalog: its row (ON DELETE CASCADE drops
    /// the utterances / ai_turns) plus its FTS rows (cleared explicitly — the
    /// FTS5 index isn't FK-cascaded, same as `replace_session`). Idempotent:
    /// deleting an absent session affects 0 rows and still commits.
    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        let tx = self.conn.transaction().context("begin delete tx")?;
        tx.execute(
            "DELETE FROM search_index WHERE session_id = ?1",
            params![session_id],
        )
        .context("clear search rows")?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .context("delete session row")?;
        tx.commit().context("commit delete tx")?;
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

    /// v0.22.x — recompute each session's headline `ai_model` from the model its
    /// AI turns ACTUALLY ran on: the most-frequent non-empty `ai_turns.model`
    /// (ties broken toward the most recent). The per-turn model is the resolved
    /// local/cloud endpoint, so this corrects rows whose `SessionStart` logged the
    /// raw cloud `ai_model` config field — a local session that showed e.g.
    /// `claude-sonnet-4-6` now reflects its real local model.
    ///
    /// A session with NO AI turns has no model to attribute: the correlated
    /// subquery returns no row → `ai_model` is cleared to NULL, which the archive
    /// renders as "—" (an honest "no AI was used here", not the stale configured
    /// default it was journaled with). Idempotent; returns the rows touched.
    pub fn backfill_session_models(&self) -> Result<usize> {
        self.conn
            .execute(
                "UPDATE sessions SET ai_model = ( \
                     SELECT t.model FROM ai_turns t \
                     WHERE t.session_id = sessions.id AND t.model <> '' \
                     GROUP BY t.model \
                     ORDER BY COUNT(*) DESC, MAX(t.unix_ms) DESC \
                     LIMIT 1 \
                 )",
                [],
            )
            .context("backfill session models")
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
                "SELECT session_id, unix_ms, source, text, audio_ms FROM utterances \
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

    /// Full-text search the catalog (FTS5 + BM25, best match first). `query` is a
    /// raw FTS5 MATCH expression — a bare word or `"a phrase"` works. Returns up
    /// to `limit` hits across utterances + AI questions/answers; an FTS syntax
    /// error in `query` surfaces as `Err` (the caller can show "no results").
    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, kind, CAST(unix_ms AS INTEGER), body, bm25(search_index) \
                 FROM search_index WHERE search_index MATCH ?1 ORDER BY bm25(search_index) LIMIT ?2",
            )
            .context("prepare search")?;
        let rows = stmt
            .query_map(params![query, limit], |r| {
                Ok(SearchHit {
                    session_id: r.get(0)?,
                    kind: r.get(1)?,
                    unix_ms: r.get(2)?,
                    body: r.get(3)?,
                    rank: r.get(4)?,
                })
            })
            .context("query search")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map search hit")?);
        }
        Ok(out)
    }

    // ======================================================================
    // Curated personal memory (Phase 3b) — candidates + approved items. These
    // tables are USER-OWNED (not a rebuildable projection): the indexer never
    // touches them, so they survive a catalog re-index. Timestamps are
    // caller-stamped (like `Session::indexed_at_ms`) so callers control the
    // clock and tests stay deterministic.
    // ======================================================================

    /// Insert a memory candidate (status defaults `pending`). Returns its id.
    pub fn insert_candidate(&mut self, c: &NewMemoryCandidate, created_at_ms: i64) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO memory_candidates \
                 (profile_id, source_session_id, kind, text, reason, status, created_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
                params![
                    c.profile_id,
                    c.source_session_id,
                    c.kind,
                    c.text,
                    c.reason,
                    created_at_ms
                ],
            )
            .context("insert memory candidate")?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Candidates for a profile in a status (e.g. `pending`), newest first.
    /// `limit` caps the rows (a NEGATIVE value = unlimited, SQLite `LIMIT`
    /// semantics): the review UI passes a small cap so a huge backlog can't freeze
    /// or blow up the tab (F8), while callers needing the full set pass -1. The
    /// rendered text columns are COALESCE'd to '' so a corrupt NULL row degrades to
    /// blank instead of failing the whole query.
    pub fn list_candidates(
        &self,
        profile_id: &str,
        status: &str,
        limit: i64,
    ) -> Result<Vec<MemoryCandidate>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, profile_id, source_session_id, COALESCE(kind, ''), \
                 COALESCE(text, ''), COALESCE(reason, ''), status, created_at_ms \
                 FROM memory_candidates WHERE profile_id = ?1 AND status = ?2 \
                 ORDER BY created_at_ms DESC, id DESC LIMIT ?3",
            )
            .context("prepare list_candidates")?;
        let rows = stmt
            .query_map(params![profile_id, status, limit], row_to_candidate)
            .context("query list_candidates")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map candidate")?);
        }
        Ok(out)
    }

    /// Just the candidate TEXTS for a profile across ALL statuses — for the
    /// extractor's dedup (never re-suggest an existing text). One query selecting
    /// only the `text` column, instead of three unbounded `list_candidates(-1)`
    /// full-row scans (P1-3). profile_id is indexed.
    pub fn candidate_texts(&self, profile_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT COALESCE(text, '') FROM memory_candidates WHERE profile_id = ?1")
            .context("prepare candidate_texts")?;
        let rows = stmt
            .query_map(params![profile_id], |r| r.get::<_, String>(0))
            .context("query candidate_texts")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map candidate text")?);
        }
        Ok(out)
    }

    /// Count candidates for a profile in a status (e.g. `pending` → a badge).
    pub fn count_candidates(&self, profile_id: &str, status: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM memory_candidates WHERE profile_id = ?1 AND status = ?2",
                params![profile_id, status],
                |r| r.get(0),
            )
            .context("count candidates")
    }

    /// Set a candidate's status (e.g. `rejected`). To APPROVE use
    /// [`approve_candidate`] (it also mints the item).
    pub fn set_candidate_status(&mut self, id: i64, status: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE memory_candidates SET status = ?2 WHERE id = ?1",
                params![id, status],
            )
            .context("set candidate status")?;
        Ok(())
    }

    /// Edit a candidate's text (refine before approving).
    pub fn update_candidate_text(&mut self, id: i64, text: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE memory_candidates SET text = ?2 WHERE id = ?1",
                params![id, text],
            )
            .context("update candidate text")?;
        Ok(())
    }

    /// Approve a candidate: mark it `approved` AND mint a [`MemoryItem`] from it,
    /// in ONE transaction. Returns the new item id. Errors if the candidate id
    /// doesn't exist (the row read fails → the tx rolls back).
    pub fn approve_candidate(&mut self, candidate_id: i64, approved_at_ms: i64) -> Result<i64> {
        let tx = self.conn.transaction().context("begin approve tx")?;
        let (profile_id, kind, text, source): (String, String, String, Option<String>) = tx
            .query_row(
                "SELECT profile_id, kind, text, source_session_id \
                 FROM memory_candidates WHERE id = ?1",
                params![candidate_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .context("load candidate to approve")?;
        tx.execute(
            "UPDATE memory_candidates SET status = 'approved' WHERE id = ?1",
            params![candidate_id],
        )
        .context("mark candidate approved")?;
        tx.execute(
            "INSERT INTO memory_items \
             (profile_id, kind, text, source_session_id, approved_at_ms, embedding_status) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'none')",
            params![profile_id, kind, text, source, approved_at_ms],
        )
        .context("mint memory item")?;
        let item_id = tx.last_insert_rowid();
        tx.commit().context("commit approve tx")?;
        Ok(item_id)
    }

    /// Insert a memory item directly (a manual note). Returns its id.
    pub fn insert_memory_item(&mut self, m: &NewMemoryItem, approved_at_ms: i64) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO memory_items \
                 (profile_id, kind, text, source_session_id, approved_at_ms, embedding_status) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 'none')",
                params![
                    m.profile_id,
                    m.kind,
                    m.text,
                    m.source_session_id,
                    approved_at_ms
                ],
            )
            .context("insert memory item")?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Memory items for a profile, newest first. By default ACTIVE only;
    /// `include_archived` adds the soft-deleted ones.
    /// `limit` caps the rows (NEGATIVE = unlimited, SQLite `LIMIT` semantics): the
    /// review UI passes a small cap (F8); the AI-context callers pass -1. Text
    /// columns are COALESCE'd so a corrupt NULL row degrades to blank.
    pub fn list_memory_items(
        &self,
        profile_id: &str,
        include_archived: bool,
        limit: i64,
    ) -> Result<Vec<MemoryItem>> {
        let mut stmt = self
            .conn
            .prepare(if include_archived {
                "SELECT id, profile_id, COALESCE(kind, ''), COALESCE(text, ''), \
                 source_session_id, approved_at_ms, archived_at_ms, embedding_status \
                 FROM memory_items WHERE profile_id = ?1 \
                 ORDER BY approved_at_ms DESC, id DESC LIMIT ?2"
            } else {
                "SELECT id, profile_id, COALESCE(kind, ''), COALESCE(text, ''), \
                 source_session_id, approved_at_ms, archived_at_ms, embedding_status \
                 FROM memory_items WHERE profile_id = ?1 \
                 AND archived_at_ms IS NULL ORDER BY approved_at_ms DESC, id DESC LIMIT ?2"
            })
            .context("prepare list_memory_items")?;
        let rows = stmt
            .query_map(params![profile_id, limit], row_to_item)
            .context("query list_memory_items")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("map memory item")?);
        }
        Ok(out)
    }

    /// Edit an approved item's text.
    pub fn update_memory_item_text(&mut self, id: i64, text: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE memory_items SET text = ?2 WHERE id = ?1",
                params![id, text],
            )
            .context("update memory item text")?;
        Ok(())
    }

    /// Soft-delete (archive) an item — it stops feeding context but stays on
    /// record. Pass the archive timestamp.
    pub fn archive_memory_item(&mut self, id: i64, archived_at_ms: i64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE memory_items SET archived_at_ms = ?2 WHERE id = ?1",
                params![id, archived_at_ms],
            )
            .context("archive memory item")?;
        Ok(())
    }

    /// Hard-delete an item (the user's "delete" — gone for good).
    pub fn delete_memory_item(&mut self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM memory_items WHERE id = ?1", params![id])
            .context("delete memory item")?;
        Ok(())
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
        audio_ms: row.get(4)?,
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

/// Map a `memory_candidates` row (column order matches the SELECTs above).
fn row_to_candidate(row: &Row) -> rusqlite::Result<MemoryCandidate> {
    Ok(MemoryCandidate {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        source_session_id: row.get(2)?,
        kind: row.get(3)?,
        text: row.get(4)?,
        reason: row.get(5)?,
        status: row.get(6)?,
        created_at_ms: row.get(7)?,
    })
}

/// Map a `memory_items` row (column order matches the SELECTs above).
fn row_to_item(row: &Row) -> rusqlite::Result<MemoryItem> {
    Ok(MemoryItem {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        kind: row.get(2)?,
        text: row.get(3)?,
        source_session_id: row.get(4)?,
        approved_at_ms: row.get(5)?,
        archived_at_ms: row.get(6)?,
        embedding_status: row.get(7)?,
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
            audio_ms: None,
        }
    }

    #[test]
    fn delete_session_removes_row_children_and_search() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("2026-06-04_10-00-00_ab12");
        store
            .replace_session(&s, &[utt(&s.id, 1, "mic", "hash map lookup")], &[])
            .unwrap();
        assert_eq!(store.count_utterances(&s.id).unwrap(), 1);
        assert!(!store.search("hash", 10).unwrap().is_empty());

        store.delete_session(&s.id).unwrap();
        assert_eq!(store.count_utterances(&s.id).unwrap(), 0); // FK cascade
        assert!(store.search("hash", 10).unwrap().is_empty()); // FTS cleared
        assert!(store.session_utterances(&s.id).unwrap().is_empty());
        // Idempotent: deleting an absent session is a no-op.
        store.delete_session(&s.id).unwrap();
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
    fn utterance_audio_ms_round_trips() {
        // F1: the per-line audio offset survives insert→select; NULL (old lines)
        // round-trips as None.
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("2026-06-25_10-00-00_aud0");
        let mut u0 = utt(&s.id, 10, "mic", "first");
        u0.audio_ms = Some(0);
        let mut u1 = utt(&s.id, 20, "system", "second");
        u1.audio_ms = Some(4_200);
        let u2 = utt(&s.id, 30, "mic", "third"); // audio_ms stays None
        store.replace_session(&s, &[u0, u1, u2], &[]).unwrap();

        let got = store.session_utterances(&s.id).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].audio_ms, Some(0));
        assert_eq!(got[1].audio_ms, Some(4_200));
        assert_eq!(got[2].audio_ms, None);
    }

    #[test]
    fn backfill_sets_headline_to_turn_mode() {
        let mut store = Store::open_in_memory().unwrap();
        // A session journaled with the WRONG cloud headline (the historical bug:
        // SessionStart logged the raw cloud `ai_model` even on a local session)…
        let mut s = sample_session("sess-backfill");
        s.ai_model = Some("claude-sonnet-4-6".into());
        // …but whose turns actually ran on a local model (gemma twice → mode).
        let turns = vec![
            AiTurn {
                session_id: s.id.clone(),
                unix_ms: 10,
                purpose: "live_ask".into(),
                model: "gemma-4-12B".into(),
                question: "q1".into(),
                answer: "a1".into(),
                latency_ms: None,
                attached_screenshot: false,
            },
            AiTurn {
                session_id: s.id.clone(),
                unix_ms: 20,
                purpose: "summary".into(),
                model: "gemma-4-12B".into(),
                question: String::new(),
                answer: "a2".into(),
                latency_ms: None,
                attached_screenshot: false,
            },
        ];
        store.replace_session(&s, &[], &turns).unwrap();

        let n = store.backfill_session_models().unwrap();
        assert!(n >= 1);
        let got = store.get_session(&s.id).unwrap().unwrap();
        assert_eq!(got.ai_model.as_deref(), Some("gemma-4-12B"));
    }

    #[test]
    fn backfill_nulls_turnless_sessions() {
        let mut store = Store::open_in_memory().unwrap();
        let mut s = sample_session("sess-noturns");
        s.ai_model = Some("claude-sonnet-4-6".into());
        store.replace_session(&s, &[], &[]).unwrap();
        store.backfill_session_models().unwrap();
        let got = store.get_session(&s.id).unwrap().unwrap();
        // No AI turns → no model to attribute → headline cleared (archive "—"),
        // not the stale cloud default it was journaled with.
        assert_eq!(got.ai_model, None);
    }

    #[test]
    fn backfill_mode_wins_and_ignores_empty() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("sess-mode");
        let mk = |ms: i64, model: &str| AiTurn {
            session_id: s.id.clone(),
            unix_ms: ms,
            purpose: "live_ask".into(),
            model: model.into(),
            question: String::new(),
            answer: "a".into(),
            latency_ms: None,
            attached_screenshot: false,
        };
        // gemma ×2 beats a lone cloud escalation; the empty-model turn is ignored.
        let turns = vec![
            mk(10, "gemma-4-12B"),
            mk(20, "claude-sonnet-4-6"),
            mk(30, "gemma-4-12B"),
            mk(40, ""),
        ];
        store.replace_session(&s, &[], &turns).unwrap();
        store.backfill_session_models().unwrap();
        let got = store.get_session(&s.id).unwrap().unwrap();
        assert_eq!(got.ai_model.as_deref(), Some("gemma-4-12B"));
    }

    #[test]
    fn backfill_tie_breaks_toward_most_recent() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("sess-tie");
        let mk = |ms: i64, model: &str| AiTurn {
            session_id: s.id.clone(),
            unix_ms: ms,
            purpose: "live_ask".into(),
            model: model.into(),
            question: String::new(),
            answer: "a".into(),
            latency_ms: None,
            attached_screenshot: false,
        };
        // One turn each → a 1–1 tie; `ORDER BY COUNT(*) DESC, MAX(unix_ms) DESC`
        // breaks it toward the model used most recently.
        let turns = vec![mk(10, "older-model"), mk(20, "newer-model")];
        store.replace_session(&s, &[], &turns).unwrap();
        store.backfill_session_models().unwrap();
        let got = store.get_session(&s.id).unwrap().unwrap();
        assert_eq!(got.ai_model.as_deref(), Some("newer-model"));
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

    #[test]
    fn fts_search_finds_question_answer_and_utterance() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("s1");
        let utts = vec![utt(&s.id, 1, "mic", "tell me about binary trees")];
        let turns = vec![AiTurn {
            session_id: s.id.clone(),
            unix_ms: 2,
            purpose: "live_ask".into(),
            model: "gemma".into(),
            question: "what is a hash map".into(),
            answer: "a key-value structure".into(),
            latency_ms: Some(10),
            attached_screenshot: false,
        }];
        store.replace_session(&s, &utts, &turns).unwrap();

        assert!(store
            .search("hash", 10)
            .unwrap()
            .iter()
            .any(|h| h.kind == "question" && h.body.contains("hash")));
        assert!(store
            .search("structure", 10)
            .unwrap()
            .iter()
            .any(|h| h.kind == "answer"));
        assert!(store
            .search("binary", 10)
            .unwrap()
            .iter()
            .any(|h| h.kind == "utterance"));
    }

    #[test]
    fn fts_reindex_does_not_duplicate_hits() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("s1");
        let utts = vec![utt(&s.id, 1, "mic", "uniquetoken here")];
        store.replace_session(&s, &utts, &[]).unwrap();
        assert_eq!(store.search("uniquetoken", 10).unwrap().len(), 1);
        // Re-index the same session — the explicit search_index clear must keep
        // it at one hit, not two.
        store.replace_session(&s, &utts, &[]).unwrap();
        assert_eq!(store.search("uniquetoken", 10).unwrap().len(), 1);
    }

    #[test]
    fn fts_search_matches_russian() {
        let mut store = Store::open_in_memory().unwrap();
        let s = sample_session("ru1");
        let utts = vec![utt(&s.id, 1, "mic", "расскажи про хеш-таблицу и деревья")];
        store.replace_session(&s, &utts, &[]).unwrap();
        // unicode61 splits on the hyphen, so "хеш" is its own searchable token.
        assert!(!store.search("хеш", 10).unwrap().is_empty());
        assert!(!store.search("деревья", 10).unwrap().is_empty());
    }

    // ---- Curated memory (Phase 3b) ----

    fn new_cand(text: &str) -> NewMemoryCandidate {
        NewMemoryCandidate {
            profile_id: "default".into(),
            source_session_id: Some("sess-1".into()),
            kind: "answer".into(),
            text: text.into(),
            reason: "asked twice".into(),
        }
    }

    #[test]
    fn latest_migration_version_is_5() {
        let store = Store::open_in_memory().unwrap();
        let v: i32 = store
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, migrations::LATEST_VERSION);
        // Literal pin — bump deliberately when adding a migration (now incl. 0005
        // memory_v2: source_text / entity / norm_status for fact normalization, M1).
        assert_eq!(v, 5);
    }

    #[test]
    fn candidate_insert_then_list_pending() {
        let mut store = Store::open_in_memory().unwrap();
        let id = store
            .insert_candidate(&new_cand("explain B-trees"), 100)
            .unwrap();
        assert!(id > 0);
        let pending = store.list_candidates("default", "pending", -1).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].text, "explain B-trees");
        assert_eq!(pending[0].status, "pending");
        assert_eq!(store.count_candidates("default", "pending").unwrap(), 1);
    }

    #[test]
    fn list_candidates_respects_limit_newest_first() {
        // F8 — the review tab caps how many rows it loads: a positive limit returns
        // the newest N (created_at_ms DESC); a negative limit returns all.
        let mut store = Store::open_in_memory().unwrap();
        store.insert_candidate(&new_cand("oldest"), 100).unwrap();
        store.insert_candidate(&new_cand("middle"), 200).unwrap();
        store.insert_candidate(&new_cand("newest"), 300).unwrap();
        let capped = store.list_candidates("default", "pending", 2).unwrap();
        assert_eq!(capped.len(), 2); // capped
        assert_eq!(capped[0].text, "newest"); // newest first
        assert_eq!(capped[1].text, "middle");
        let all = store.list_candidates("default", "pending", -1).unwrap();
        assert_eq!(all.len(), 3); // negative = unlimited
    }

    #[test]
    fn reject_candidate_leaves_no_item() {
        let mut store = Store::open_in_memory().unwrap();
        let id = store.insert_candidate(&new_cand("x"), 1).unwrap();
        store.set_candidate_status(id, "rejected").unwrap();
        assert_eq!(store.count_candidates("default", "pending").unwrap(), 0);
        assert_eq!(store.count_candidates("default", "rejected").unwrap(), 1);
        assert!(store
            .list_memory_items("default", false, -1)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn approve_candidate_mints_item_and_marks_approved() {
        let mut store = Store::open_in_memory().unwrap();
        let cid = store
            .insert_candidate(&new_cand("use a hash map"), 5)
            .unwrap();
        let item_id = store.approve_candidate(cid, 50).unwrap();
        assert!(item_id > 0);
        assert_eq!(store.count_candidates("default", "pending").unwrap(), 0);
        assert_eq!(store.count_candidates("default", "approved").unwrap(), 1);
        let items = store.list_memory_items("default", false, -1).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "use a hash map");
        assert_eq!(items[0].kind, "answer");
        assert_eq!(items[0].source_session_id.as_deref(), Some("sess-1"));
        assert_eq!(items[0].approved_at_ms, 50);
        assert!(items[0].archived_at_ms.is_none());
    }

    #[test]
    fn approve_missing_candidate_errs() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(store.approve_candidate(999, 1).is_err());
        // The failed transaction left no item behind.
        assert!(store
            .list_memory_items("default", true, -1)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn edit_candidate_then_approve_keeps_edit() {
        let mut store = Store::open_in_memory().unwrap();
        let cid = store.insert_candidate(&new_cand("raw"), 1).unwrap();
        store.update_candidate_text(cid, "refined").unwrap();
        store.approve_candidate(cid, 2).unwrap();
        let items = store.list_memory_items("default", false, -1).unwrap();
        assert_eq!(items[0].text, "refined");
    }

    #[test]
    fn archive_hides_from_active_but_kept() {
        let mut store = Store::open_in_memory().unwrap();
        let cid = store.insert_candidate(&new_cand("y"), 1).unwrap();
        let item_id = store.approve_candidate(cid, 2).unwrap();
        store.archive_memory_item(item_id, 99).unwrap();
        assert!(store
            .list_memory_items("default", false, -1)
            .unwrap()
            .is_empty());
        let all = store.list_memory_items("default", true, -1).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].archived_at_ms, Some(99));
    }

    #[test]
    fn manual_item_insert_then_hard_delete() {
        let mut store = Store::open_in_memory().unwrap();
        let item_id = store
            .insert_memory_item(
                &NewMemoryItem {
                    profile_id: "default".into(),
                    kind: "note".into(),
                    text: "manual note".into(),
                    source_session_id: None,
                },
                10,
            )
            .unwrap();
        assert_eq!(
            store.list_memory_items("default", true, -1).unwrap().len(),
            1
        );
        store.delete_memory_item(item_id).unwrap();
        assert!(store
            .list_memory_items("default", true, -1)
            .unwrap()
            .is_empty());
    }
}
