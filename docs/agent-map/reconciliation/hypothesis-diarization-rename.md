# Original persistence C13: bounded rename evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_diarization_rename_hypotheses.py) use shipped migrations and temporary SQLite connections. They do not execute Rust or touch an owner catalog. C13 remains a hypothesis.

## Read-modify-replace can lose a concurrent name

[rename_speaker](<../../../overlay-backend/src/persistence/sqlite_store.rs#L309-L320>) reads the row, edits the map, then calls [put_diarization](<../../../overlay-backend/src/persistence/sqlite_store.rs#L283-L307>), which replaces the whole JSON blob. The selected function has no transaction or `json_set`.

Two temporary connections demonstrate the precondition: both read `{}`, each adds a different speaker, and the later replace leaves only its own name. Blank removal and a missing row are separate deterministic boundaries.

## Limits

This is a sequential SQLite interleaving, not a Rust thread race or production concurrency failure. Original status and 39/75/5 remain unchanged.
