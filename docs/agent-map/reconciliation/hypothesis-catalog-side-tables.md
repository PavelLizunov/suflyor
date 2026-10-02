# Confirmed persistence C10/C12: bounded side-table contract

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_catalog_side_table_hypotheses.py) apply shipped migrations to a temporary catalog and inspect frozen source. No owner catalog or Rust `Store` execution. Existing confirmed statuses stay unchanged.

## C10 — rebuild and hard delete have different side effects

[replace_session](<../../../overlay-backend/src/persistence/sqlite_store.rs#L87-L154>) deletes and recreates the session projection and FTS rows. Utterances cascade, but the function contains no diarization or memory delete. The temporary schema confirms diarization and memory survive that projection replacement.

[delete_session](<../../../overlay-backend/src/persistence/sqlite_store.rs#L162-L190>) explicitly deletes the diarization row before deleting the session. It still does not delete memory. [Indexer match](<../../../overlay-backend/src/persistence/indexer.rs#L65-L144>) projects only session start/stop, transcript, and AI response. [Journal variants](<../../../overlay-backend/src/journal/types.rs#L7-L90>) include summary, detector, tile, rate-limit, and error events that this match ignores.

## C12 — unreadable JSON is an error

[get_diarization](<../../../overlay-backend/src/persistence/sqlite_store.rs#L230-L281>) documents `None` for an unreadable row, but `serde_json::from_str(...).context(...)?` propagates an error. A temporary row with `{` remains present while JSON parsing fails. This source/JSON disagreement is not a Rust caller-path execution.

## Limits

These fixtures do not call rusqlite, inspect an owner catalog, or prove every memory API. They add executable bounds to already confirmed mechanisms; 39/75/5 remains unchanged.
