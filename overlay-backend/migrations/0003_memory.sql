-- 0003 — curated personal memory (Phase 3b). USER-OWNED metadata layered on the
-- catalog: candidate suggestions mined from sessions, plus the user's APPROVED
-- memory items. Unlike sessions/utterances/ai_turns (a rebuildable projection of
-- the JSONL journals), these rows are NOT derived from the journals — they are
-- the user's curation, so they MUST survive a catalog re-index. The indexer only
-- upserts session rows by id (replace_session) and never touches these tables,
-- so a normal re-index preserves them.
--
-- profile_id defaults to 'default': multi-profile memory is an OPEN question in
-- docs/personal-memory-and-session-store-architecture.md; the column exists now
-- so per-profile memory can land later without a schema break.
--
-- IMMUTABLE once shipped — a change is a NEW migration file + entry in
-- migrations.rs.

CREATE TABLE memory_candidates (
    id                INTEGER PRIMARY KEY,
    profile_id        TEXT NOT NULL DEFAULT 'default',
    source_session_id TEXT,                            -- session it was mined from (nullable)
    kind              TEXT NOT NULL,                   -- experience|preference|answer|weak_topic|note
    text              TEXT NOT NULL,
    reason            TEXT NOT NULL DEFAULT '',        -- why suggested (shown to the user)
    status            TEXT NOT NULL DEFAULT 'pending', -- pending|approved|rejected
    created_at_ms     INTEGER NOT NULL
);
CREATE INDEX idx_mem_cand ON memory_candidates(profile_id, status, created_at_ms);

CREATE TABLE memory_items (
    id                INTEGER PRIMARY KEY,
    profile_id        TEXT NOT NULL DEFAULT 'default',
    kind              TEXT NOT NULL,                   -- experience|preference|answer|weak_topic|note
    text              TEXT NOT NULL,
    source_session_id TEXT,
    approved_at_ms    INTEGER NOT NULL,
    archived_at_ms    INTEGER,                         -- NULL = active; set = archived (soft delete)
    embedding_status  TEXT NOT NULL DEFAULT 'none'     -- none|pending|done (Phase 4 embeddings)
);
CREATE INDEX idx_mem_item ON memory_items(profile_id, archived_at_ms);
