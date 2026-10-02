# Original persistence C14/C15: bounded schema evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_memory_migration_hypotheses.py) use shipped migrations and a temporary catalog. They do not run Rust migrations or touch an owner catalog. C14/C15 remain hypotheses.

## C15 — status is advisory and approval can fork

[Candidate schema](<../../../overlay-backend/migrations/0003_memory.sql#L16-L25>) stores status as text with a comment listing three values, but no `CHECK`. The temporary table accepts `banana`. [Status writer](<../../../overlay-backend/src/persistence/sqlite_store.rs#L528-L536>) writes the supplied value directly. [Approval](<../../../overlay-backend/src/persistence/sqlite_store.rs#L552-L576>) checks pending only at its initial read, then updates status and inserts an item.

The fixture distinguishes two sequences. Repeating the write/insert after one pending snapshot creates two items. Re-checking pending before the second insert blocks it. This is a schema/precondition model, not a concurrent Rust transaction race.

## C14 — shipped memory has no session cascade

[Memory schema](<../../../overlay-backend/migrations/0003_memory.sql#L16-L38>) contains neither `CHECK` nor `REFERENCES sessions`. [Catalog schema](<../../../overlay-backend/migrations/0001_session_catalog.sql#L22-L42>) has exactly two session cascade references: utterances and AI turns. Deleting the temporary session leaves the memory item intact.

[Migration runner](<../../../overlay-backend/src/persistence/migrations.rs#L32-L52>) skips versions at or below current and has no downgrade branch. A future-schema process can therefore continue without migration work. No actual future catalog or Rust `user_version` transition was executed.

## Limits

No rusqlite migration runner, concurrent approval, owner catalog, or UI action was run. Original statuses and 39/75/5 remain unchanged.
