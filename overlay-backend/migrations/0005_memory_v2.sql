-- 0005 — memory v2 (fact normalization, M1 of docs/memory-architecture.md).
--
-- Facts are captured from RAW STT dialog, which is messy; storing them verbatim
-- pollutes memory forever. This adds the columns the capture→normalize pipeline
-- needs: the CLEANED/normalized text stays in `text` (what feeds prompts), the
-- verbatim capture is preserved in `source_text` (provenance — a bad rewrite is
-- always recoverable), `entity` is the normalized subject key (for coherent
-- grouping, M3), and `norm_status` records how `text` was produced.
--
-- Additive only: `ALTER TABLE ADD COLUMN` (no data rewrite). Legacy rows get the
-- defaults (`source_text` NULL = `text` IS the source; `norm_status`='none') and
-- are treated exactly as before — retrieval/injection behavior is unchanged until
-- the normalize pipeline runs.
--
-- IMMUTABLE once shipped — a change is a NEW migration file + entry in migrations.rs.

ALTER TABLE memory_items ADD COLUMN source_text TEXT;                          -- verbatim capture; NULL = `text` is the source
ALTER TABLE memory_items ADD COLUMN entity TEXT;                               -- normalized subject key ('z14-backup'); NULL = none
ALTER TABLE memory_items ADD COLUMN norm_status TEXT NOT NULL DEFAULT 'none';  -- none|heuristic|llm|failed

ALTER TABLE memory_candidates ADD COLUMN source_text TEXT;
ALTER TABLE memory_candidates ADD COLUMN entity TEXT;
ALTER TABLE memory_candidates ADD COLUMN norm_status TEXT NOT NULL DEFAULT 'none';
