-- 0006 — speaker diarization results (D2). A per-session, user-triggered OFFLINE
-- re-analysis of the session's `system.wav`: the sherpa diarizer's speaker
-- segments, later aligned to transcript lines to show WHO spoke.
--
-- Like the memory tables (0003), this is NOT derived from the JSONL journals — it
-- is a separate analysis keyed by the stable session id, so it MUST survive a
-- catalog re-index. The indexer only upserts session rows (replace_session) and
-- never touches this table.
--
-- One row per session (write-all-once per run, read-all-once per open, mapped to
-- utterances in Rust at render time — nothing queries segments by time-range
-- across sessions, so a JSON blob fits and a normalized segment table would model
-- a query that does not exist). Re-running REPLACEs the row and clears the renames
-- (cluster ids permute across runs, so stale names would mislabel).
--
-- IMMUTABLE once shipped — a change is a NEW migration file + entry in migrations.rs.

CREATE TABLE diarization (
    session_id         TEXT PRIMARY KEY,             -- stable journal-stem id (not FK'd to the projection)
    created_at_ms      INTEGER NOT NULL,
    num_speakers       INTEGER NOT NULL,             -- as reported by the diarizer
    model_id           TEXT NOT NULL,                -- provenance, e.g. "pyannote-3.0+wespeaker-resnet34"
    segments_json      TEXT NOT NULL,                -- [{"s":ms,"e":ms,"sp":i},...] sorted by start
    speaker_names_json TEXT NOT NULL DEFAULT '{}'    -- {"0":"Тимур",...} user renames (display id -> name)
);
