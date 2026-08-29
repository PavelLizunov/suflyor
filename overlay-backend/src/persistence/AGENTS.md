# Persistence Module (`overlay-backend/src/persistence`)

Operational guide for Suflyor's database persistence, declarative schema migrations, SQLite runtime invariants, recovery mechanisms, and verification rules.

## Schema & Migration Ownership
- **Declarative Migration Files**: SQL schema migrations live as immutable files under `overlay-backend/migrations/` (`0001_session_catalog.sql` through `0006_diarization.sql`).
- **Embedding & Dispatch**: `migrations.rs` embeds SQL files via `include_str!` into the `MIGRATIONS` array and tracks `PRAGMA user_version` against `LATEST_VERSION`.
- **Immutability Rule**: Shipped migration files are **IMMUTABLE**. Never edit an existing migration file. Any schema change requires creating a new `.sql` file in `overlay-backend/migrations/`, incrementing `LATEST_VERSION`, and adding the new tuple to `MIGRATIONS` in `migrations.rs`.
- **Transaction Safety**: Each migration step executes inside its own connection transaction (`conn.transaction()`), bumping `PRAGMA user_version` on success. Failure rolls back the individual transaction and bails before applying subsequent migrations.
- **Model Abstraction**: `models.rs` defines clean Rust models (`Session`, `Utterance`, `AiTurn`, `MemoryCandidate`, `MemoryItem`, `Diarization`, `DiarSegment`, `SearchHit`). Public API callers never interact directly with `rusqlite` connections, statement handles, or raw SQL.

## SQLite Invariants
- **Thread Isolation**: `Store` (`sqlite_store.rs`) wraps `rusqlite::Connection` and is `!Sync`. Catalog operations run on dedicated background threads (e.g., `reindex_default`). The live audio capture and AI pipeline **NEVER** block on SQLite queries or disk I/O.
- **Mandatory Pragmas**: Connection opening (`Store::open`) enforces:
  - `PRAGMA journal_mode = WAL;`: Enables concurrent non-blocking reads while background worker writes.
  - `PRAGMA foreign_keys = ON;`: Enables relational constraints and `ON DELETE CASCADE` cleanup.
  - `PRAGMA busy_timeout = 2000;`: Prevents instant `SQLITE_BUSY` errors during write contention between background indexing and archive UI queries.
- **Rebuildable Projection & Rotation**: SQLite (`catalog.sqlite`) is a queryable projection over JSONL append-only session journals (`sessions/`). While journals are pruned by retention policies, catalog session records are additive and deliberate long-term history. If `catalog.sqlite` is removed, re-indexing reconstructs the database from surviving journals.
- **Atomic Session Replacement**: `Store::replace_session` replaces a session and its child utterances/AI turns within a single transaction, clearing prior FTS index rows and re-populating them via triggers.

## Recovery & Security Risk
- **Pre-Migration Backup**: `Store::open` checks `user_version` against `LATEST_VERSION`. If a schema migration is pending, it truncates the WAL (`PRAGMA wal_checkpoint(TRUNCATE)`) and creates a best-effort pre-migration backup (`catalog.sqlite.bak`) prior to applying SQL migrations.
- **Non-Destructive Maintenance & Repair** (`maintenance.rs`):
  - Diagnostics run `PRAGMA integrity_check(50)` and `PRAGMA foreign_key_check`.
  - Repair operations (`wal_checkpoint`, `REINDEX`, FTS `rebuild`, `VACUUM`) are strictly non-destructive and **NEVER** issue `DROP`, `DELETE`, `CREATE`, or `UPDATE` on user data tables.
  - **Strict Backup Guarantee**: Prior to executing repair or targeted table clear operations, a backup is written to `backups/catalog-<millis>.sqlite` (via `VACUUM INTO` or raw file copy including `-wal`/`-shm`). If backup creation fails, repair/clear operations immediately abort without mutating the database. Backups are pruned to keep at most 5 historical copies.
- **Targeted Table Clears**: `clear_memory_candidates_default` and `clear_memory_items_default` issue purges on memory tables only. They strictly check a hardcoded whitelist (`memory_candidates`, `memory_items`) and reject any other table name to prevent SQL injection or unintended table truncation.
- **Security Boundary**: Raw credentials (`groq_api_key`, `ai_bearer`) and Windows Credential Manager secrets **MUST NEVER** be stored in SQLite catalog tables. Search indexing sanitizes text, and diagnostics reporting uses generic error descriptors to avoid path or secret leakage.

## Gate Classification Requirement
- Database schema changes, migration additions, `Store` modifications, or maintenance routine updates represent high-risk data persistence and recovery code paths.
- **Required Gate**: All changes to `overlay-backend/src/persistence/**` or `overlay-backend/migrations/**` require **Full Gate** validation:
  ```powershell
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/git-gate-native.ps1 manual -Full
  ```
  or full CI execution (`scripts/ci.ps1`).

## Primary Verification
Validate guide formatting and whitespace integrity:
```powershell
git diff --check -- overlay-backend/src/persistence/AGENTS.md
```
