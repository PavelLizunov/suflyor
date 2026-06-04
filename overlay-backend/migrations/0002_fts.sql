-- 0002 — full-text search over the catalog (SQLite FTS5 + BM25). A standalone
-- FTS5 table, kept in sync by AFTER INSERT triggers on utterances / ai_turns, so
-- the indexer's normal inserts auto-populate it. replace_session clears a
-- session's rows up front (DELETE FROM search_index WHERE session_id = ...), so
-- a re-index repopulates without duplicates. UNINDEXED columns are stored +
-- returned but NOT tokenized; only `body` is searchable.
--
-- `unicode61 remove_diacritics 2` tokenizes Unicode (Russian + English) and
-- splits on punctuation, so "хеш-таблица" → "хеш" + "таблица" and a search for
-- "хеш" matches.
CREATE VIRTUAL TABLE search_index USING fts5(
    session_id UNINDEXED,
    kind       UNINDEXED,        -- utterance | question | answer
    unix_ms    UNINDEXED,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER trg_utterances_fts_ai AFTER INSERT ON utterances BEGIN
    INSERT INTO search_index (session_id, kind, unix_ms, body)
    SELECT new.session_id, 'utterance', new.unix_ms, new.text
    WHERE trim(new.text) <> '';
END;

CREATE TRIGGER trg_ai_turns_fts_ai AFTER INSERT ON ai_turns BEGIN
    INSERT INTO search_index (session_id, kind, unix_ms, body)
    SELECT new.session_id, 'question', new.unix_ms, new.question
    WHERE trim(new.question) <> '';
    INSERT INTO search_index (session_id, kind, unix_ms, body)
    SELECT new.session_id, 'answer', new.unix_ms, new.answer
    WHERE trim(new.answer) <> '';
END;
