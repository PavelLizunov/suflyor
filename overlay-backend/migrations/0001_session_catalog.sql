-- 0001 — session catalog: a REBUILDABLE indexed projection over the append-only
-- JSONL journals. The JSONL files stay the source of truth (see
-- docs/personal-memory-and-session-store-architecture.md); this DB can be
-- deleted and re-indexed from them without losing raw history.
--
-- IMMUTABLE: once shipped this file is never edited — a schema change is a NEW
-- migration file + entry in migrations.rs.

CREATE TABLE sessions (
    id                    TEXT PRIMARY KEY,          -- JSONL file stem (YYYY-MM-DD_HH-MM-SS_rand)
    journal_path          TEXT NOT NULL,             -- absolute path to the source .jsonl
    started_at_ms         INTEGER,                   -- unix_ms of session_start (NULL if absent)
    finished_at_ms        INTEGER,                   -- unix_ms of session_stop (NULL = crashed/active)
    status                TEXT NOT NULL DEFAULT 'completed', -- completed | crashed | active
    ai_model              TEXT,
    transcript_lines      INTEGER NOT NULL DEFAULT 0,
    ai_turns_count        INTEGER NOT NULL DEFAULT 0,
    total_cost_microcents INTEGER NOT NULL DEFAULT 0,
    indexed_at_ms         INTEGER NOT NULL           -- when this row was (re)indexed
);

CREATE TABLE utterances (
    id          INTEGER PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    unix_ms     INTEGER NOT NULL,
    source      TEXT NOT NULL,                       -- mic | system
    text        TEXT NOT NULL
);
CREATE INDEX idx_utterances_session ON utterances(session_id);

CREATE TABLE ai_turns (
    id                  INTEGER PRIMARY KEY,
    session_id          TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    unix_ms             INTEGER NOT NULL,
    purpose             TEXT NOT NULL DEFAULT '',
    model               TEXT NOT NULL DEFAULT '',
    question            TEXT NOT NULL DEFAULT '',
    answer              TEXT NOT NULL DEFAULT '',
    latency_ms          INTEGER,
    attached_screenshot INTEGER NOT NULL DEFAULT 0   -- 0/1 bool
);
CREATE INDEX idx_ai_turns_session ON ai_turns(session_id);
