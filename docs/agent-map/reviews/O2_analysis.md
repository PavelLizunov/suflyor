> **TL;DR:** Suflyor's persistence layer is a well-engineered two-tier architecture: append-only JSONL journals (source of truth) backed by a rebuildable SQLite catalog (`catalog.sqlite`) with WAL mode, 2s busy_timeout, FK cascades, FTS5 full-text search, curated personal memory tables, and diarization side-data. Transaction boundaries, crash recovery, migration safety, and backup guarantees are all solid. The main residual risks are (1) a subtle retention-vs-catalog data lifecycle asymmetry that's acknowledged by design, (2) the 2s busy_timeout being tight for a VACUUM-concurrent scenario, and (3) the absence of vector embeddings (Phase M4 is design-only).

---

# Exhaustive Persistence & Database Analysis — Suflyor `overlay-backend`

## 1. Architecture Overview

### 1.1 Two-Tier Design (JSONL + SQLite)

The persistence layer is split into two deliberate tiers documented in `mod.rs:1-22`:

| Layer | Role | Location | Mutability |
|---|---|---|---|
| **JSONL Journals** | Primary source of truth, append-only event log | `%APPDATA%/suflyor/sessions/*.jsonl` | Append-only per session; pruned by retention |
| **SQLite Catalog** | Queryable projection for archive UI + FTS search | `%APPDATA%/suflyor/catalog.sqlite` | Rebuildable index; also stores user-owned memory/diarization |

**Critical design nuance** (`mod.rs:10-18`): The catalog is described as "rebuildable" but has evolved beyond a pure projection. The `memory_candidates`, `memory_items` (migration 0003/0005), and `diarization` (migration 0006) tables are **user-owned data** that do NOT derive from journals and **cannot** be rebuilt from them. Deleting `catalog.sqlite` loses these permanently. The module doc at line 13-18 acknowledges this: once journals are pruned, catalog session rows become the "LAST surviving copy."

### 1.2 Module Layout

```
overlay-backend/src/persistence/
├── mod.rs            # Public API: reindex_default(), open_default_store(), sessions_dir()
├── sqlite_store.rs   # Store struct (owns Connection), all CRUD, FTS search, memory ops
├── migrations.rs     # Declarative schema version tracking (LATEST_VERSION=6)
├── models.rs         # Plain row structs: Session, Utterance, AiTurn, MemoryCandidate, etc.
├── indexer.rs        # JSONL→SQLite projection: index_journal_file(), index_all()
├── maintenance.rs    # Non-destructive diagnose + repair, targeted memory clears
└── AGENTS.md         # Operational guide
```

---

## 2. SQLite Schema (6 Migrations)

### 2.1 Migration Registry (`migrations.rs:14-27`)

```rust
pub(crate) const LATEST_VERSION: i32 = 6;
const MIGRATIONS: &[(i32, &str)] = &[
    (1, include_str!("../../migrations/0001_session_catalog.sql")),
    (2, include_str!("../../migrations/0002_fts.sql")),
    (3, include_str!("../../migrations/0003_memory.sql")),
    (4, include_str!("../../migrations/0004_utterance_audio_ms.sql")),
    (5, include_str!("../../migrations/0005_memory_v2.sql")),
    (6, include_str!("../../migrations/0006_diarization.sql")),
];
```

SQL files are embedded via `include_str!` — immutable at compile time, never read from disk at runtime. The immutability rule is enforced by convention and documented in `AGENTS.md:8` and every migration file header.

### 2.2 Schema Tables

| Table | Migration | Type | FK Cascade | Notes |
|---|---|---|---|---|
| `sessions` | 0001 | Rebuildable projection | — | PK = JSONL file stem |
| `utterances` | 0001 | Rebuildable projection | `ON DELETE CASCADE` → sessions | Indexed on `session_id` |
| `ai_turns` | 0001 | Rebuildable projection | `ON DELETE CASCADE` → sessions | Indexed on `session_id` |
| `search_index` | 0002 | FTS5 virtual table | None (explicit DELETE) | `unicode61 remove_diacritics 2` tokenizer |
| `memory_candidates` | 0003 | **User-owned** | None | Indexed on `(profile_id, status, created_at_ms)` |
| `memory_items` | 0003 | **User-owned** | None | Indexed on `(profile_id, archived_at_ms)` |
| `diarization` | 0006 | **User-owned** side-table | None (deliberately not FK'd) | PK = `session_id` (TEXT), not FK'd to projection |

**0004** adds `utterances.audio_ms` (nullable `ALTER TABLE ADD COLUMN`).
**0005** adds `source_text`, `entity`, `norm_status` to both memory tables.

### 2.3 FTS5 Search Index (`0002_fts.sql`)

The FTS5 table uses `unicode61 remove_diacritics 2` tokenizer, which handles both Russian and English and splits on punctuation (e.g., `хеш-таблица` → `хеш` + `таблица`). Three column types are searchable:

- `utterance` (from `utterances.text`)
- `question` (from `ai_turns.question`)
- `answer` (from `ai_turns.answer`)

Synchronization is via **AFTER INSERT triggers** on `utterances` and `ai_turns` (`0002_fts.sql:19-31`). Empty text is filtered (`WHERE trim(new.text) <> ''`). The FTS5 table is **not** FK-cascaded, so `replace_session` and `delete_session` must explicitly `DELETE FROM search_index WHERE session_id = ?1` before re-inserting — correctly implemented at `sqlite_store.rs:95-98` and `sqlite_store.rs:164-168`.

**BM25 ranking** is used in `Store::search()` (`sqlite_store.rs:416`): `ORDER BY bm25(search_index)` — SQLite's convention where lower = more relevant.

### 2.4 Diarization Side-Table Design Decision

The `diarization` table (`0006`) is deliberately **not** FK'd to `sessions` (`0006_diarization.sql:19` comment: "not FK'd to the projection"). This is intentional: diarization results must survive a catalog re-index (where session rows are replaced wholesale). The segments are stored as a JSON blob (`segments_json`), which is justified in the migration comment: "nothing queries segments by time-range across sessions, so a JSON blob fits."

---

## 3. Transaction Boundaries & Atomicity

### 3.1 Migration Transactions (`migrations.rs:32-52`)

Each migration runs in its own `conn.transaction()`:

```rust
let tx = conn.transaction().context("begin migration tx")?;
tx.execute_batch(sql)?;
tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
tx.commit()?;
```

**Assessment:** ✅ Correct. A failed migration rolls back its own transaction and bails before subsequent migrations. The `user_version` bump is inside the same transaction, so an interrupted migration leaves `user_version` at the last good state. The `version` value comes from a trusted in-crate constant (`MIGRATIONS` array), not user input, so the `format!` is safe.

### 3.2 Session Replacement (`sqlite_store.rs:84-153`)

`replace_session()` wraps the entire delete-then-reinsert cycle in a single transaction:

```rust
let tx = self.conn.transaction()?;
tx.execute("DELETE FROM sessions WHERE id = ?1", ...)?;     // CASCADE drops utterances + ai_turns
tx.execute("DELETE FROM search_index WHERE session_id = ?1", ...)?;  // FTS not FK'd
tx.execute("INSERT INTO sessions ...", ...)?;
// prepared-statement loop: INSERT INTO utterances ...
// prepared-statement loop: INSERT INTO ai_turns ...
tx.commit()?;
```

**Assessment:** ✅ Correct. The entire replace is atomic. FK `ON DELETE CASCADE` cleans up children when the session row is deleted; FTS is explicitly cleared and repopulated by the triggers on the new INSERTs. Idempotent: test `reindex_is_idempotent_no_duplicates` and `fts_reindex_does_not_duplicate_hits` verify this.

### 3.3 Session Deletion (`sqlite_store.rs:162-177`)

`delete_session()` also uses a transaction, explicitly clearing `search_index`, `diarization`, and the session row:

```rust
let tx = self.conn.transaction()?;
DELETE FROM search_index WHERE session_id = ?1
DELETE FROM diarization WHERE session_id = ?1
DELETE FROM sessions WHERE id = ?1   // CASCADE drops utterances + ai_turns
tx.commit()?;
```

**Assessment:** ✅ Correct. Covers all three non-cascading side-tables. Idempotent: deleting an absent session touches 0 rows.

### 3.4 Candidate Approval (`sqlite_store.rs:552-576`)

`approve_candidate()` atomically marks the candidate `approved` AND mints a `MemoryItem` in one transaction:

```rust
let tx = self.conn.transaction()?;
// SELECT ... WHERE id = ?1 AND status = 'pending'  (fails if not pending)
// UPDATE memory_candidates SET status = 'approved'
// INSERT INTO memory_items ...
let item_id = tx.last_insert_rowid();
tx.commit()?;
```

**Assessment:** ✅ Correct. The `status = 'pending'` guard prevents double-approval (tested by `approve_candidate_twice_mints_only_one_item`). A missing candidate causes `query_row` to return `QueryReturnedNoRows` → `Err`, rolling back — no orphan item (tested by `approve_missing_candidate_errs`).

### 3.5 Non-Transactional Operations

Several write operations are **not** wrapped in explicit transactions:

- `put_diarization()` — `INSERT OR REPLACE` (single statement, auto-committed)
- `set_candidate_status()` — single `UPDATE`
- `update_candidate_text()` — single `UPDATE`
- `update_memory_item_text()` — single `UPDATE`
- `archive_memory_item()` — single `UPDATE`
- `delete_memory_item()` — single `DELETE`
- `insert_candidate()` — single `INSERT`
- `insert_memory_item()` — single `INSERT`
- `backfill_session_models()` — single correlated `UPDATE`

**Assessment:** ✅ Acceptable. Each is a single SQL statement, which is implicitly auto-committed by SQLite. No cross-table consistency is required for these operations, so explicit transactions would add overhead without benefit.

---

## 4. WAL Mode, Concurrency Guards, and SQLITE_BUSY

### 4.1 PRAGMA Configuration (`sqlite_store.rs:48-51`)

Every connection (both `Store::open` and `maintenance::open_main`) sets:

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 2000;
```

**WAL mode** allows concurrent readers while one writer is active. This is essential because:

1. At **startup**, `reindex_default()` runs on a detached `std::thread::spawn` (`overlay_host_windows.rs:539`), opening its own `Store`.
2. The **archive UI** (F7) opens a separate `Store` via `open_default_store()` for read-only browsing (`aux_windows/archive.rs:129`).
3. At **session stop**, `index_journal_file` runs on another detached thread (`slint_session.rs:1487`).

The comment at `sqlite_store.rs:44-47` explains the 2s busy_timeout: "the stop-session indexer thread and the archive-open sweep can write concurrently (stop → immediate F7); WAL allows ONE writer, and without a timeout the loser gets an instant SQLITE_BUSY instead of waiting out the ~ms-long window."

### 4.2 Thread Isolation (`!Sync`)

`Store` wraps `rusqlite::Connection` which is `!Sync` — it cannot be shared across threads. The architecture enforces this:

- The indexer opens its own `Store` on a detached background thread
- The archive UI opens its own `Store` for read queries
- The memory context builder opens its own `Store` per call (`context_builder.rs:177`, `summary_ref.rs:173`)
- Settings memory operations open their own `Store` per action (`settings_memory.rs` passim)

**Assessment:** ✅ Correct. No `Arc<Mutex<Store>>` sharing, no cross-thread connection reuse. Each caller gets its own connection with fresh pragmas. The `!Sync` marker on `rusqlite::Connection` makes sharing a compile error.

### 4.3 Concurrency Risk Analysis

**Potential contention scenario:** The stop-session indexer (`slint_session.rs:1487-1519`) retries up to 4 times with 400ms sleep between attempts specifically to handle `SQLITE_BUSY`:

```rust
for attempt in 0..4u8 {
    if attempt > 0 {
        std::thread::sleep(Duration::from_millis(400));
    }
    let indexed = open_default_store().and_then(|mut store| {
        index_journal_file(&mut store, &path)
    });
    // ...
}
```

**Assessment:** ✅ Robust. The retry loop handles the race between the stop-session indexer and a concurrent archive-open or maintenance operation. The 2000ms `busy_timeout` + 400ms inter-retry sleep provides ample time for typical sub-ms writer windows. However, a `VACUUM` operation (from maintenance) could hold the writer lock longer — see §6.2 for the risk.

---

## 5. Crash Consistency

### 5.1 JSONL Journal Layer

The JSONL journal is the primary crash-consistency mechanism:

1. **Append-only writes** via `BufWriter<File>` on a dedicated `journal-writer` thread (`writer.rs:277-323`).
2. **Flush after each batch**: the writer drains the channel, then flushes (`writer.rs:307`).
3. **Graceful shutdown** with ack: `emit_summary_and_stop()` writes `SessionSummary` + `SessionStop`, then calls `close()` → `shutdown(2s)` which sends a `Shutdown` command and waits for the writer thread's flush + ack (`writer.rs:156-216`).
4. **Crash detection**: A journal with `session_start` but no `session_stop` is indexed as `status = "crashed"` (`indexer.rs:155-159`). Crashed rows are **re-indexed** on every sweep until a stop marker appears — the `finalized_session_ids()` query excludes crashed rows (`sqlite_store.rs:354-367`), so `index_all` re-projects them (tested by `index_all_heals_a_crashed_row_once_a_stop_appears`).

### 5.2 SQLite Catalog Layer

SQLite WAL mode provides its own crash consistency:
- WAL journal survives process crashes; SQLite recovers automatically on next open.
- Pre-migration backup: `Store::open` creates `catalog.sqlite.bak` before migrating (`sqlite_store.rs:52-65`), with WAL checkpoint first so the backup is self-contained.
- Maintenance backup: `diagnose_and_repair_at` creates timestamped backups via `VACUUM INTO` (preferred) or raw file copy including `-wal`/`-shm` siblings (`maintenance.rs:281-318`).

### 5.3 Session Recovery (`journal/recovery.rs`)

`find_unfinished_session()` scans for the newest `.jsonl` that has a `session_start` but no `session_stop`/`session_summary`, within a 12-hour window (`RECOVERY_MAX_AGE_MS = 12h`). It extracts the last 8 transcript lines and the last Q&A pair for the recovery UI. Files > 16MB are skipped (`RECOVERY_MAX_READ_BYTES`).

**Assessment:** ✅ Well-designed. The 12h window prevents stale crashed sessions from triggering recovery indefinitely. The `parse_unfinished` function tolerates corrupt lines (JSON parse failures are skipped).

---

## 6. Error Handling: Locked/Corrupted DB

### 6.1 Locked Database (`SQLITE_BUSY`)

- **Primary guard**: `busy_timeout = 2000` makes SQLite wait up to 2s for a write lock before returning `SQLITE_BUSY`.
- **Secondary guard**: The stop-session indexer retries 4× with 400ms delays (`slint_session.rs:1491-1519`).
- **Tertiary guard**: All catalog operations are best-effort from the hot path's perspective. The startup reindex runs on a detached thread; its failure is logged but never blocks app startup (`overlay_host_windows.rs:541-548`). The archive UI handles `open_default_store()` failure by showing an "archive unavailable" state (`mod.rs:72-74`).

### 6.2 Corrupted Database (`maintenance.rs`)

The `diagnose_and_repair_at()` pipeline:

1. **Backup first** — `VACUUM INTO` (preferred, self-contained) or raw file copy with `-wal`/`-shm` siblings. **Hard invariant:** if backup fails, repair is **aborted** without touching the DB (`maintenance.rs:98-111`).
2. **Pre-repair checks** — `PRAGMA integrity_check(50)` + `PRAGMA foreign_key_check` (`maintenance.rs:159-211`).
3. **Non-destructive repair** — `wal_checkpoint(TRUNCATE)`, `REINDEX`, FTS5 `rebuild`, `VACUUM` (`maintenance.rs:215-246`). **Hard invariant:** NO `DROP`/`DELETE`/`CREATE`/`UPDATE` on user data (`maintenance.rs:7-10`).
4. **Post-repair re-check** — same integrity + FK checks → final verdict.
5. **Backup pruning** — keeps only the 5 newest `catalog-<millis>.sqlite` backups (`maintenance.rs:324-354`).

**Unopenable DB**: If `Connection::open()` fails, the error is reported in `DbHealth` rather than propagated as a fatal error (`maintenance.rs:115-125`).

### 6.3 Targeted Memory Clears (`maintenance.rs:378-479`)

`clear_memory_candidates_default()` and `clear_memory_items_default()` delete only `memory_candidates` or `memory_items` rows for `profile_id = 'default'`. Safety:

- **Whitelist guard** (`maintenance.rs:448-450`): only `"memory_candidates" | "memory_items"` reach SQL; anything else bails before I/O.
- **SQL injection prevention**: The table name is interpolated from the whitelist (not user input), and `profile_id` uses a parameterized query (`params!["default"]`).
- **Backup-before-delete**: Strict — no backup → no delete (`maintenance.rs:462-465`).

Tests verify: whitelist rejection (`clear_rejects_non_whitelisted_table`), cross-table isolation (`clear_candidates_clears_only_the_queue_and_backs_up`), no-op on missing DB (`clear_missing_db_is_noop`).

---

## 7. Migration Upgrade Paths

### 7.1 Forward-Only, Per-Step Transactions

```rust
for (version, sql) in MIGRATIONS {
    if *version <= current { continue; }
    let tx = conn.transaction()?;
    tx.execute_batch(sql)?;
    tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
    tx.commit()?;
}
```

- Migrations are **strictly forward**: no downgrade path. A newer binary always migrates an older DB up.
- Each step is independently transactional: a failure at migration 4 leaves the DB at version 3 (not 0).
- The `user_version` bump is **inside** the same transaction, so it's atomic with the schema change.

### 7.2 Pre-Migration Backup (`sqlite_store.rs:52-65`)

Before migrating a pre-existing DB:
1. WAL checkpoint (`TRUNCATE`) to fold the WAL into the main file → self-contained backup.
2. `std::fs::copy(path, path.with_extension("sqlite.bak"))` — best-effort, logged-and-continued on failure.

**Note:** This creates a single `.bak` file (overwritten each time), unlike the maintenance module's timestamped backups. A rapid sequence of migrations (e.g., upgrading from v1 to v6) would overwrite the backup 5 times, but each step is small and unlikely to fail.

### 7.3 Schema Evolution Strategy

All 6 migrations use safe, additive DDL:
- **0001–0003**: `CREATE TABLE`, `CREATE INDEX`, `CREATE VIRTUAL TABLE`, `CREATE TRIGGER`
- **0004–0005**: `ALTER TABLE ADD COLUMN` (no data rewrite, NULLable defaults)
- **0006**: `CREATE TABLE`

No migration uses `DROP`, data rewrite, or `ALTER TABLE` rename/modify. This is the safest possible evolution pattern for SQLite.

---

## 8. JSONL Journal System

### 8.1 Writer Architecture (`journal/writer.rs`)

The journal writer uses an **unbounded async channel** (`tokio::sync::mpsc::unbounded_channel`) consumed by a dedicated OS thread (`journal-writer`):

```
Main thread → mpsc::UnboundedSender<WriterCmd> → journal-writer thread → BufWriter<File>
```

The writer thread does batch draining: after processing a `blocking_recv()`, it calls `try_recv()` in a loop to batch-flush multiple lines, then flushes. This amortizes syscall overhead.

**Shutdown protocol** (`writer.rs:152-220`):
1. Send `WriterCmd::Shutdown(ack_tx)` through the channel.
2. Wait up to 2s for the ack via a `std::sync::mpsc` rendezvous.
3. Join the thread if the ack confirms.
4. Record `ShutdownState::Done(outcome)` to prevent double-shutdown.

### 8.2 Retention Policy (`journal/retention.rs`)

```rust
pub const KEEP_LAST_SESSIONS: usize = 100;
pub const MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024; // 500 MB
```

`prune_old_sessions_with_size_cap()` sorts by mtime (newest first), deletes everything beyond `keep` count, then enforces the byte cap by deleting oldest-first.

**Assessment:** ✅ Simple and effective. Runs at session open (`writer.rs:89`), so pruning is never on the critical recording path. Only `.jsonl` files are pruned — audio `.wav` files are not managed here.

### 8.3 Retention ↔ Catalog Asymmetry

**This is the most important architectural subtlety.** `mod.rs:10-18` documents it explicitly:

> The indexer is additive (it never deletes session rows on its own), so once a journal is pruned from disk under journal retention, its catalog row becomes the LAST surviving copy of that session's transcript + AI turns.

This means:
- **First 100 sessions**: Have both JSONL + catalog rows. Catalog is rebuildable.
- **Sessions 101+**: JSONL pruned. Catalog row is the **only** surviving copy. Deleting `catalog.sqlite` loses them permanently.
- **Memory tables**: Never derived from journals. Always the only copy.
- **Diarization**: Never derived from journals. Always the only copy.

**Risk:** A user who deletes `catalog.sqlite` expecting "just rebuild from journals" will lose all sessions beyond the retention window, all curated memory, and all diarization results. This is documented as intentional ("a feature, not drift").

---

## 9. Knowledge Base (KB) Search Index

### 9.1 Architecture (`kb.rs`)

The KB is **entirely in-memory**, **compile-time embedded**, and has **no SQLite involvement**:

```rust
const GLOSSARY_MD: &str = include_str!("../knowledge/glossary.md");
const COMMANDS_MD: &str = include_str!("../knowledge/commands.md");
const PATTERNS_MD: &str = include_str!("../knowledge/patterns.md");
static CACHE: OnceLock<Vec<KBEntry>> = OnceLock::new();
```

~1600 entries parsed from markdown on first access (~30ms), cached in a `OnceLock`. Search is a linear scan with pre-lowercased string comparison — 4-tier ranking (exact key match → key-starts-with → heading-contains → body-contains). Query capped at 200 chars (DoS guard).

**Assessment:** The KB has zero interaction with SQLite persistence. It's a read-only, thread-safe, static reference. No crash consistency concerns.

### 9.2 KB vs. FTS5 Search

| Feature | KB (`kb.rs`) | Catalog FTS (`search_index`) |
|---|---|---|
| Data source | Embedded markdown (compile-time) | Session utterances + AI turns |
| Storage | In-memory `OnceLock<Vec>` | SQLite FTS5 virtual table |
| Algorithm | Linear scan, substring match, 4-tier ranking | BM25 relevance ranking |
| Tokenizer | Manual lowercase + `contains()` | `unicode61 remove_diacritics 2` |
| Surface | F4 palette | F7 archive search |
| Mutability | Static (recompile to change) | Dynamic (grows with sessions) |

---

## 10. Vector Embeddings / Memory Search

### 10.1 Current State: Schema-Only

The `memory_items` table has an `embedding_status` column (`none|pending|done`, migration 0003), but **no embedding storage or vector search exists in code**. The column is always set to `'none'` on insert (`sqlite_store.rs:569`, `sqlite_store.rs:585`).

From the memory module's `AGENTS.md`:
> **Phase M4 (Embeddings & Hybrid RRF):** Sidecar `llama-server` on port `:8082` running `multilingual-e5-small Q8` GGUF (~130 MB); SQLite `memory_embeddings` vector table (f32 BLOB); in-Rust brute-force cosine similarity scan; RRF combining BM25 FTS5 and cosine ranks.

This is **proposed design only** — not implemented.

### 10.2 Current Memory Retrieval

Memory retrieval for prompt context uses **keyword-based relevance** (`context_builder.rs:rank_by_relevance`):
- Symmetric prefix root matching (`words_match`, shared root ≥ 4 chars) between query tokens and item `text`/`entity` fields.
- Fallback to newest-first recency when no terms match.
- Hard limits: 8 items, 1200 chars total, 240 chars per item.

For summary references (`summary_ref.rs`): key-term extraction (capitalized words, ALL-CAPS, Latin in Cyrillic) + prefix-stem matching (≥5 chars for Russian declension tolerance).

---

## 11. Identified Risks & Observations

### 11.1 Low-Severity Risks

| # | Risk | Location | Severity | Notes |
|---|---|---|---|---|
| 1 | Pre-migration backup overwrites single `.bak` file | `sqlite_store.rs:61` | Low | Rapid multi-version upgrade writes 1 backup for the first pending migration, doesn't back up intermediate states. But migrations are additive DDL, so partial failure is recoverable. |
| 2 | `busy_timeout = 2000` vs. `VACUUM` | `maintenance.rs:242` | Low | A maintenance `VACUUM` holds an exclusive lock for the duration of the rewrite; a concurrent `Store::open` or archive read during those seconds could SQLITE_BUSY. Mitigated by the fact maintenance is user-triggered and rare. |
| 3 | Unbounded JSONL channel | `writer.rs:95` | Low | `mpsc::unbounded_channel` could theoretically grow if the writer thread stalls. In practice, the writer thread is I/O-bound on local disk and drains faster than events arrive. |
| 4 | `count_table_at` uses `format!` for table name in SQL | `maintenance.rs:494` | Low | Mitigated by the whitelist check 6 lines above (`maintenance.rs:487-489`). Only `memory_candidates` or `memory_items` ever reach the format string. |

### 11.2 Design Observations (Not Bugs)

| # | Observation | Assessment |
|---|---|---|
| A | `diarization` table is deliberately NOT FK'd to `sessions` | Correct: survives re-index. `delete_session` explicitly cleans it. |
| B | FTS5 index is NOT FK-cascaded | Correct: requires explicit DELETE before re-insert. Both `replace_session` and `delete_session` handle this. |
| C | `open_default_store()` opens a fresh connection per call | Acceptable: each call gets its own WAL reader. No connection pooling needed for the usage pattern (infrequent UI-triggered reads). |
| D | `backfill_session_models()` is a global correlated UPDATE | Idempotent; runs once per startup reindex. Could be slow on a very large catalog but bounded by session count. |
| E | Journal `now_unix_ms()` returns `u128` but is cast to `i64` in the indexer | `indexer.rs:164`: `i64::try_from(...)`.unwrap_or(0)`. Safe for the next ~292 billion years. |

### 11.3 Test Coverage Assessment

The persistence module has **excellent** test coverage (46+ tests across `sqlite_store.rs`, `indexer.rs`, `maintenance.rs`):

- **Idempotency**: `reindex_is_idempotent_no_duplicates`, `fts_reindex_does_not_duplicate_hits`
- **Crash recovery**: `index_all_heals_a_crashed_row_once_a_stop_appears`
- **Empty/corrupt tolerance**: `corrupt_line_is_skipped_not_fatal`, `index_all_skips_empty_journal_then_indexes_later_content`
- **Cascade correctness**: `delete_session_removes_row_children_and_search`
- **Transaction safety**: `approve_missing_candidate_errs`, `approve_candidate_twice_mints_only_one_item`
- **Maintenance safety**: `repair_is_healthy_backs_up_and_preserves_rows`, `clear_rejects_non_whitelisted_table`
- **Backup hygiene**: `prune_keeps_at_most_five_backups`, `is_backup_file_matches_only_our_pattern`
- **Migration pin**: `latest_migration_version_is_6` (literal assertion — must be bumped deliberately)

All tests use `Store::open_in_memory()` for isolation, with `tempfile::tempdir()` for disk-based tests (indexer, maintenance).

---

## 12. Summary Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                  overlay-host.exe                    │
│                                                     │
│  ┌─ startup ──┐   ┌─ session stop ─┐   ┌─ F7 UI ─┐│
│  │ std::thread│   │ std::thread    │   │ UI thread││
│  │::spawn()  │   │::spawn()       │   │          ││
│  └─────┬──────┘   └──────┬─────────┘   └────┬─────┘│
└────────┼─────────────────┼──────────────────┼──────┘
         │                 │                  │
    reindex_default()  index_journal_file() open_default_store()
         │                 │                  │
    ┌────▼─────────────────▼──────────────────▼──────┐
    │                  Store::open()                  │
    │  PRAGMA journal_mode = WAL                      │
    │  PRAGMA foreign_keys = ON                       │
    │  PRAGMA busy_timeout = 2000                     │
    │  run_migrations() [0001..0006]                  │
    │                                                 │
    │  ┌─ Rebuildable ─────────────────────────────┐  │
    │  │ sessions ←FK── utterances                 │  │
    │  │          ←FK── ai_turns                   │  │
    │  │ search_index (FTS5, trigger-synced)        │  │
    │  └───────────────────────────────────────────┘  │
    │  ┌─ User-Owned (survives reindex) ───────────┐  │
    │  │ memory_candidates (pending/approved/reject)│  │
    │  │ memory_items (active/archived, norm_status)│  │
    │  │ diarization (JSON segments + speaker names)│  │
    │  └───────────────────────────────────────────┘  │
    └────────┬────────────────────────────────────────┘
             │ Projection of:
    ┌────────▼────────────────────────────────────────┐
    │  JSONL Journals  (%APPDATA%/suflyor/sessions/)  │
    │  Append-only, one file per session              │
    │  Dedicated writer thread (BufWriter + flush)    │
    │  Retention: 100 sessions / 500 MB               │
    └─────────────────────────────────────────────────┘
```

---

## 13. Conclusion

The persistence layer is **production-grade and well-defended**. The two-tier JSONL+SQLite design correctly separates crash-proof append-only logging from queryable projections. Transaction boundaries are precise — multi-row operations use explicit transactions, single-statement ops rely on SQLite's auto-commit. WAL mode + busy_timeout + thread-per-connection isolation handle concurrency correctly. The migration system is forward-only with per-step atomicity and pre-migration backups. The maintenance module's backup-before-repair invariant and whitelist-guarded clears are exemplary defensive coding. The only material gap is the unimplemented Phase M4 embedding/vector search (schema placeholder exists, no runtime code).",
