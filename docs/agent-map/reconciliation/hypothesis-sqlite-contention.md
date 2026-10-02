# Original persistence C02/C18: bounded actual SQLite engine evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [portable fixtures](../operations/test_sqlite_contention_hypotheses.py) use stdlib SQLite with **shipped six migrations**, two private temporary connections and dummy records. No owner DB/journals/backups or native rusqlite/UI/indexer operations. SQL engine evidence is not bundled SQLite/Windows/macOS task reachability acceptance; original statuses remain hypotheses.

## C02 — distinguish deferred writer versus stale-read upgrade

[Store pragmas](<../../../overlay-backend/src/persistence/sqlite_store.rs#L39-L54>) WAL/FK/busy_timeout2000 and [replace](<../../../overlay-backend/src/persistence/sqlite_store.rs#L87-L109>)/[delete](<../../../overlay-backend/src/persistence/sqlite_store.rs#L162-L181>) default transactions. [Approval](<../../../overlay-backend/src/persistence/sqlite_store.rs#L552-L582>) begins then SELECT pending candidate **before** writing item/status; replace/delete begin then write first. Absence of retry/transaction_with_behavior observed, not corrected here.

Actual engine fixture observations:

- A BEGIN DEFERRED, SELECT pending; B commits status update; A UPDATE fails **SQLITE_BUSY_SNAPSHOT (517)**. Rollback refreshes A snapshot, sees committed value. A generic busy_timeout cannot make a stale snapshot valid: need transaction restart semantics, not just elapsed wait. This is SQL precondition/counterexample, not Rust approve duplicate/caller fault test.
- A begins/writes and holds transaction; B write-first fails **SQLITE_BUSY (5)** after a short explicit timeout; after A commit B can write. Fixture timeout **20ms versus product2000ms**, no measured duration/speed claim.
- WAL reader retains old committed view while B update succeeds; rollback/end restores latest view. Thus ordinary reader/writer coexistence counterevidence remains: not every two catalog operations cause SQLITE_BUSY.

Original C02 text focused two writers. More precise prerequisites differ by read-before-write and transaction lifetime. No native release/debug SQLite version/locking stress/archiving UI behavior reproduced, no hypothesis automatically promoted to runtime bug.

## C18 — explicit checkpoint absent is not automatic-checkpoint absence

[Store source](<../../../overlay-backend/src/persistence/sqlite_store.rs#L56-L77>) best-effort migration TRUNCATE+raw main-file backup, no explicit wal_autocheckpoint override. Fixture engine `PRAGMA wal_autocheckpoint` reports **1000**. This is observed stdlib-engine default, not proven bundled rusqlite compile/runtime option.

Short held-reader fixture: establish snapshot after clean truncate; another connection commits four row updates. TRUNCATE reports **busy=1, log>checkpointed**, so active reader constrains checkpoint. After reader rollback, TRUNCATE returns **(0,0,0)** and WAL length zero. This is a bounded checkpoint-lifetime mechanism, **not unbounded WAL growth/performance/production lock denial/force-rebuild test**. No large FTS/load/soak or long-running DB action.

Counterevidence: default auto checkpoint plus connection-close behavior and short app statement lifetimes can limit WAL; absence of explicit checkpoint on indexer close alone does not prove user-visible issue. Original C18 remains hypothesis requiring actual reader lifetimes/engine flags/native product reproduction.

## Scope/verification

All six fixtures passed locally: three actual transaction/snapshot cases, default pragmas/source check, held-reader checkpoint, source approve-write ordering/original status integrity. Python engine/runtime version recorded in portable receipt; SQL schema file hashes bound to frozen source by checkpoint. No Python clone of Rust Store, migrations untouched, no production/db fix applied. Original register hash and C02/C18 identity remain unchanged.

Next useful evidence is trusted exact-SHA native rusqlite call sequencing/provider timing/fault behavior, not repeating tiny SQL tests to increase counts. Other C05/C06/C08/C09/C10/C12/C14/C15/C17 persistence hypotheses unadvanced by these engine fixtures, full semantic/native/independent acceptance open.
