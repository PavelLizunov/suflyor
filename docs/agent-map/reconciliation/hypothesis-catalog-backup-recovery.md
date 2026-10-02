# Confirmed persistence C01/C16: bounded backup and recovery evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_catalog_backup_recovery_hypotheses.py) use temporary SQLite/JSONL files and frozen source. They do not migrate an owner catalog or execute Rust recovery. Existing confirmed statuses stay unchanged.

## C01 — backup copies the main file only

[Open path](<../../../overlay-backend/src/persistence/sqlite_store.rs#L56-L75>) runs `wal_checkpoint(TRUNCATE)`, discards its result, then copies `catalog.sqlite` to `.sqlite.bak`. The selected source does not copy `-wal` or `-shm`. A temporary WAL database confirms the raw main-file copy leaves no backup WAL sidecar. This does not prove a failed checkpoint or an unrestorable owner backup.

## C16 — recovery has three negative gates

[Recovery constants](<../../../overlay-backend/src/journal/recovery.rs#L6-L8>) set the maximum age to 12 hours. [Classification](<../../../overlay-backend/src/journal/recovery.rs#L130-L149>) returns no candidate when start is absent, stop or summary is present, or the start is older than the limit.

Temporary files confirm those gates independently: start-only is eligible, stop rejects it, summary rejects it even without stop, and one millisecond past 12 hours rejects it. This is a Python model of the source conditions, not Rust directory scanning.

## Limits

No owner catalog, failed-checkpoint backup, or Rust recovery scan was run. C01 and C16 were already confirmed; 39/75/5 remains unchanged.
