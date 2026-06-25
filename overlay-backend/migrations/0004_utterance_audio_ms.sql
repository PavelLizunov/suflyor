-- 0004 — per-utterance AUDIO offset (ms from the RECORDING start), for accurate
-- click-to-seek in the transcript player. The journal previously persisted only
-- the STT-finalize wall-clock (now_unix_ms); the real audio offset (the chunk's
-- ms-from-capture-start carried by the STT) was discarded, so player seeks landed
-- seconds late. NULLable: rows indexed before this migration — and old journals
-- that never wrote the field — stay NULL = "no audio offset", and the player falls
-- back to the prev-line wall-clock approximation. The FTS trigger only reads
-- session_id / unix_ms / text, so this column does not affect search.
-- IMMUTABLE once shipped: a change is a NEW migration file.
ALTER TABLE utterances ADD COLUMN audio_ms INTEGER;
