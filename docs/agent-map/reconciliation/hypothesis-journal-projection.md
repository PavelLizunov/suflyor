# Original persistence C08/C09: bounded projection evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_journal_projection_hypotheses.py) use shipped migrations, temporary SQLite rows, and frozen source order. They do not run the Rust indexer or read owner journals. Original C08/C09 remain hypotheses.

## C08 — crashed rows are intentionally retried

[index_all](<../../../overlay-backend/src/persistence/indexer.rs#L177-L214>) snapshots finalized IDs before reading the directory. [finalized IDs](<../../../overlay-backend/src/persistence/sqlite_store.rs#L348-L367>) exclude `crashed`, so a later stop can be projected again. [status selection](<../../../overlay-backend/src/persistence/indexer.rs#L148-L166>) marks a usable journal without `session_stop` as `crashed`.

The temporary projection confirms that sequence: missing stop is outside the finalized set, then a later stop heals it to `completed`. A wrong or absent `skip_active` can still expose a live file to that path, but no concurrent writer/read race was executed.

## C09 — backfill is separate and now called

Direct projection preserves `session_start.ai_model`. The shipped backfill SQL then selects the most common non-empty turn model; with no turn it clears the headline. [reindex_default](<../../../overlay-backend/src/persistence/mod.rs#L53-L65>) calls that backfill after `index_all` and logs its error without failing reindex.

Thus the original “not called” claim is source-countered for this startup path. Other callers are not exhaustively proven, and this Python projection is not the Rust `Store::backfill_session_models` execution.

## Limits

No owner catalog, live append, Rust indexer, or UI archive was run. Both original IDs stay `hypothesis`; 39/75/5 is unchanged.
