> **Redacted historical candidate report.** Original file was not modified. Model identity and original source commit are unverified; claims require reconciliation. Private-network and user-path examples are replaced with placeholders.

## Adversarial audit: catalog, indexer, migrations, JSONL writer

Invariant under test: JSONL is source of truth; `catalog.sqlite` is a rebuildable projection (except documented user-owned tables). WAL + `busy_timeout=2000` + `Store: !Sync` on worker threads.

---

### 1. Live `fs::copy` of a WAL database is not a consistent backup
- **Finding / Hypothesis:** `Store::open` (`sqlite_store.rs`, preexisting + `current < LATEST_VERSION`) runs best-effort `PRAGMA wal_checkpoint(TRUNCATE)` (return value discarded), then `std::fs::copy(path, path.with_extension("sqlite.bak"))` of the main file only, while `conn` is already open. It does not use `VACUUM INTO`, the backup API, or copy `-wal`/`-shm`.
- **Rationale:** `TRUNCATE` fails if another connection holds a read lock (archive sweep + stop-session indexer are documented concurrent writers). A failed checkpoint still proceeds to `fs::copy`. Copying a live SQLite main file without a successful checkpoint (or the backup API) yields a `.bak` that is missing WAL frames, is mid-page-write torn, or is already the post-migration file if a second process races. Restore then looks like “we had a backup” while user-owned `memory_*` / `diarization` rows are gone or the header `user_version` does not match the pages. Catalog projection can be rebuilt; memory cannot.
- **Verification Method:** Open catalog in process A with a long read transaction. In process B, set `user_version` behind `LATEST_VERSION` and call `Store::open`. Force checkpoint to fail (`PRAGMA wal_checkpoint` → busy). Diff `.bak` vs a `VACUUM INTO` snapshot; open `.bak` and `PRAGMA integrity_check`. Repeat with a second writer inserting memory rows between checkpoint and copy.

---

### 2. `transaction()` is DEFERRED; two catalog writers can hit `SQLITE_BUSY` with no retry
- **Finding / Hypothesis:** `replace_session`, `delete_session`, `approve_candidate` all use `self.conn.transaction()` (rusqlite default `BEGIN` / DEFERRED). `open` sets `PRAGMA busy_timeout = 2000` only. Comment in `Store::open` admits stop-session indexer and archive-open sweep write concurrently. There is no `transaction_with_behavior(Immediate)`, no retry/backoff, and no mapping of `ErrorCode::DatabaseBusy` to a retryable type.
- **Rationale:** DEFERRED takes the write lock on the first mutating statement. Two connections that `SELECT` (e.g. `finalized_session_ids` / `get_diarization`) then `DELETE`/`INSERT` can deadlock; SQLite returns `SQLITE_BUSY` to one side. After 2s the loser fails the whole `replace_session`. `index_all` then counts `failed` and skips that journal for this sweep. A first-time completed session stays invisible in the archive until a later sweep; a heal of `crashed` → `completed` is delayed. Under a large reindex (full JSONL rewrite of FTS), 2s is not a “~ms-long window”.
- **Verification Method:** Two threads, two `Store::open` on the same file. Thread 1: `BEGIN` + `SELECT` + sleep 3s + `replace_session` of a large utterance set. Thread 2: `index_all` / `replace_session` on another id. Assert `SQLITE_BUSY` / anyhow error after ~2000ms and that thread 2’s session is absent from `list_sessions` until retry. Re-run with `BEGIN IMMEDIATE` as control.

---

### 3. Invalid UTF-8 in a torn append fails the entire journal, not one line
- **Finding / Hypothesis:** `index_journal_file` (`indexer.rs`) uses `std::fs::read_to_string(path)?`. Any invalid UTF-8 returns `Err`, which `index_all` treats as `failed` (no row, or stale row left in place). Line-level `serde_json` skip only runs after a successful UTF-8 read. `spawn_writer` uses `writeln!(file, "{line}")` with no `sync_all`; on disk-full / kill it still continues (`note_write_error` does not stop the loop).
- **Rationale:** Append-only does not make `write(2)` atomic. A crash or `ENOSPC` mid-line can emit a truncated multibyte sequence. `read_to_string` then fails the whole file. Subsequent valid JSONL events after the torn bytes are not indexed. That contradicts the stated “corrupt line is skipped, rest indexed” policy (`corrupt_line_is_skipped_not_fatal` only covers ASCII `"not json at all"`). A previously finalized catalog row is not rebuilt; a new session may be missing entirely.
- **Verification Method:** Write a valid `session_start` line, then bytes `[0xFF, 0xFE]`, then a valid `transcript_line` + `session_stop`. Call `index_journal_file`. Expect today’s code to `Err`; after a lossy/`BufReader` + skip-invalid-line fix, expect 1 utterance + `completed`. Repeat by filling the disk during `writeln!` and killing the writer thread.

---

### 4. Writer never `fsync`s; `flush()` is not durability
- **Finding / Hypothesis:** `spawn_writer` / `finish_writer` (`journal/writer.rs`) only `BufWriter::flush()`. `OpenOptions` has no `sync_all` on interval, on `SessionStop`, or on shutdown ack. Shutdown success means “userspace buffer reached the OS,” not “stable storage.”
- **Rationale:** `SIGKILL` / `TerminateProcess` / power loss drops (a) unflushed `BufWriter` pages and (b) OS dirty pages. `emit_summary_and_stop` can therefore lose `session_stop` after the UI has already treated the session as closed. Next `index_all` projects `status = 'crashed'` (`finished_at_ms` none). Because `finalized_session_ids` excludes `crashed`, this heals if the stop line later appears; if the stop line never reached disk, the session is permanently `crashed` despite a graceful UI stop. Last N events (STT / cost) vanish with no error at `Journal::write` (`let _ = tx.send`).
- **Verification Method:** Integration test: mock `File` or use a ramdisk + hard kill after `write(SessionStop)` but before process exit; or `journalctl`-style: `shutdown` then immediately `echo b > /proc/sysrq-trigger` on a VM. Confirm last JSONL lines vs catalog status. Compare with an `f.into_inner()?.sync_all()` on shutdown.

---

### 5. Disk-full / write error does not stop the writer; torn line can land *in the middle* of the file
- **Finding / Hypothesis:** On `writeln!` / `flush` failure, `note_write_error` logs, stashes `first_error`, and the loop continues to accept more `WriterCmd::Line`. `finish_writer` still flushes and acks `Err("earlier journal write failed: …")`.
- **Rationale:** After `ENOSPC`, `BufWriter` may have already pushed a prefix of the current line (no trailing `\n`) to the `File`. Later successful writes (space freed, or partial success) append complete lines after that prefix. Indexer `content.lines()` then sees one corrupt line (skipped) — unless the torn prefix + next line concatenate into a *valid but wrong* JSON object (e.g. truncated number/string that still parses). Worse, a failed write can split UTF-8 (finding 3) or glue two events. Shutdown reports a single string; callers of `close()` ignore it (`let _ = self.shutdown(...)`).
- **Verification Method:** Wrap the file to fail on the Nth `write` with `ErrorKind::StorageFull`, then succeed. Inspect raw bytes for a newline-free fragment between two events. Run `index_journal_file` and check for merged `kind` fields / missing cost. Assert `Journal::close` currently swallows the error.

---

### 6. Session file creation is not exclusive → same-ms collision appends two sessions into one JSONL
- **Finding / Hypothesis:** `open_new_session_with_limits` builds `{chrono_like_stamp()}_{now_unix_ms() & 0xFFFFFF:06x}.jsonl` and opens with `create(true).append(true)` — not `create_new(true)`.
- **Rationale:** 24 bits of millisecond time plus a second-granularity stamp is not a unique id. Two `open_new_session` calls in the same millisecond (tests, crash-restart, overlapping shutdown/start) open the same path in append mode. Events from two logical sessions interleave in one stem. Indexer uses **file stem as `sessions.id`**: one catalog row, mixed utterances, last `session_stop` wins, `has_event` true. Rebuild from JSONL cannot split them. `skip_active` only skips one stem, so the “other” live session is indexed while still being written.
- **Verification Method:** Stub `now_unix_ms` / stamp; call `open_new_session` twice; write different `session_start` ids/models into each `Journal`; `index_journal_file` on the path; assert one row vs two. Also `OpenOptions::create_new` as the correct control.

---

### 7. Prune deletes JSONL; indexer never deletes catalog rows (projection drift)
- **Finding / Hypothesis:** `open_new_session_with_limits` calls `prune_old_sessions_with_size_cap` after creating the new file. `index_all` only inserts/replaces for files that exist; there is no “reconcile missing files → `delete_session`” pass. `delete_session` exists but is not used here.
- **Rationale:** After retention prune, archive/`search` still return sessions whose `journal_path` is gone. FTS hits point at corpses. Wiping `catalog.sqlite` and rebuilding **does** drop them (invariant holds one way). The live catalog silently violates “index over JSONL.” User-owned memory rows still reference `source_session_id` for vanished sessions (OK), but session detail will fail when the UI opens the journal.
- **Verification Method:** Index two completed journals; delete one `.jsonl` (or run prune with `keep_sessions=1`); call `index_all`; `list_sessions` still contains the missing id. Then wipe DB, `index_all` again; the id is gone. Add a test that prune also calls `delete_session`.

---

### 8. `index_all` skip set is a snapshot; live journal can be projected as `crashed` if `skip_active` is wrong/absent
- **Finding / Hypothesis:** `index_all` loads `finalized_session_ids()` once, then for each `*.jsonl` skips only `skip_active` (exact stem) or ids with `status <> 'crashed'`. `index_journal_file` uses a full-file `read_to_string` while `journal-writer` may still be appending; missing `session_stop` ⇒ `status = "crashed"` and `replace_session`.
- **Rationale:** Archive-open sweep concurrent with a live session (the exact contention the busy_timeout comment describes) will, if `skip_active` is `None` or a different stem, rewrite the live session as `crashed` with a partial transcript. That row is not finalized, so every later sweep re-reads the growing file and `DELETE`+re-`INSERT`s FTS (lock duration ↑). UI can show the in-progress meeting as crashed. A stray `session_stop` written not at EOF (or a test file) marks `completed`; **later appended events are never indexed** because the id enters `finalized_session_ids`.
- **Verification Method:** Hold a live `Journal`, omit `skip_active`, run `index_all`; assert `crashed` + partial counts. Append `session_stop` then more `transcript_line`; run `index_all` twice; assert extra lines absent. File-lock test on Windows: writer holds the handle while indexer reads.

---

### 9. Rebuild from JSONL does not restore headline `ai_model` (backfill is a one-shot side path)
- **Finding / Hypothesis:** `index_journal_file` sets `Session.ai_model` only from `session_start.ai_model`. `backfill_session_models` (`sqlite_store.rs`) recomputes mode of `ai_turns.model` and is **not** called from `replace_session` / `index_all`. Comments document that `SessionStart` historically logged the cloud config even for local runs.
- **Rationale:** Wiping `catalog.sqlite` and reindexing (the advertised recovery) restores the *wrong* headline, then archive shows `claude-sonnet-4-6` again until something else invokes backfill. Turnless sessions are nulled by backfill but reindex puts the journaled default back. Invariant “rebuild without data loss” fails for a field the product already decided was incorrect.
- **Verification Method:** Use the existing `backfill_sets_headline_to_turn_mode` fixture; drop the in-memory store; `index_journal_file` from equivalent JSONL; assert `ai_model` is the start-event value, not the turn mode. After calling `backfill_session_models`, wipe and reindex again.

---

### 10. Diarization (and memory) cannot be rebuilt from JSONL — catalog wipe is data loss
- **Finding / Hypothesis:** `replace_session` deliberately does not touch `diarization` (side table, no FK). `delete_session` does delete it. Memory APIs state tables are user-owned and indexer-never-touched. Neither is written by `journal/writer.rs` events handled in `index_journal_file` (`session_start/stop`, `transcript_line`, `ai_request/response` only).
- **Rationale:** The module rustdoc says dropping `catalog.sqlite` and reindexing loses nothing. Speaker names / segments and curated memory are only in SQLite. A “corrupt catalog → delete file” recovery (ops runbook implied by rebuild invariant) destroys them. `.bak` is the only copy, and finding 1 shows that copy is best-effort/racy. Re-index of a `crashed` session does keep diarization (good), which also means **stale diarization survives a full JSONL rewrite** if cluster ids permuted — `put_diarization` comment admits this but indexer will not clear it.
- **Verification Method:** `put_diarization` + memory insert; delete `catalog.sqlite`; `index_all`; `get_diarization` is `None`, `list_memory_items` empty. Second test: put diarization, rewrite JSONL with different audio length, `index_all` of crashed/heal path; names/segments still old.

---

### 11. FTS maintenance is a manual `DELETE FROM search_index WHERE session_id=?` inside the same tx as table rebuild
- **Finding / Hypothesis:** `replace_session` / `delete_session` explicitly `DELETE FROM search_index WHERE session_id = ?1` because “FTS5 isn’t FK-cascaded,” then rely on AFTER INSERT triggers to repopulate. Migration SQL is not in this dump (`0002_fts.sql`).
- **Rationale:** FTS5 often rejects or no-ops `DELETE`/`UPDATE` whose `WHERE` is not `rowid=?` or `MATCH`. If `session_id` is `UNINDEXED`, this delete can remove 0 rows. Reindex then **duplicates** hits (the in-memory test `fts_reindex_does_not_duplicate_hits` would catch a content-row FTS; it would **not** catch an external-content FTS or trigger that inserts on `sessions` as well as `utterances`). Partial trigger failure after `DELETE FROM sessions` (cascade children) + failed FTS delete ⇒ search hits with no parent session, or missing hits until a full `rebuild`. `delete_session` order (FTS, diarization, sessions) vs `replace_session` (sessions, FTS, insert) is inconsistent; a crash is rolled back, but trigger side effects on FTS5 have historically been a source of `integrity_check` failures.
- **Verification Method:** After `replace_session` twice, `SELECT count(*) FROM search_index WHERE session_id=?`. `INSERT` directly into `utterances` bypassing `replace_session` to see trigger shape. If `0002` uses `content='utterances'`, run `INSERT INTO search_index(search_index) VALUES('rebuild')` and compare ranks. Force `DELETE FROM search_index WHERE session_id=?` on a copy and print changes.

---

### 12. `get_diarization` contradicts its own contract on unreadable JSON
- **Finding / Hypothesis:** Doc comment: `None` if never run **or the row is unreadable JSON**. Implementation `serde_json::from_str(&segments_json).context("parse segments_json")?` (and names) returns `Err`, aborting the caller.
- **Rationale:** A single truncated `segments_json` (process kill during `put_diarization` — that `execute` is not wrapped in an explicit tx with a size check, though a single statement is atomic) or a failed migration/partial edit makes **session detail fail entirely** instead of degrading to “no diarization.” `rename_speaker` also fails because it calls `get_diarization`.
- **Verification Method:** `INSERT OR REPLACE` a diarization row with `segments_json='['`. Call `get_diarization` / `rename_speaker`. Expect `Err` today; contract says `Ok(None)`.

---

### 13. `rename_speaker` is a non-atomic read-modify-write on `&self`
- **Finding / Hypothesis:** `rename_speaker` → `get_diarization` → mutate map → `put_diarization` (`INSERT OR REPLACE` of the whole JSON blob). No SQL `json_set`, no `BEGIN IMMEDIATE`, two round-trips. `put_diarization` takes `&self`, so two workers can share the pattern on different `Store` connections.
- **Rationale:** Concurrent rename of speaker 0 and 1: last `REPLACE` wins, one name dropped. Concurrent `put_diarization` from a re-run (new cluster ids) + rename: names applied to the wrong run. Lost updates will not heal on catalog rebuild (finding 10).
- **Verification Method:** Two threads, barrier after both `get_diarization`; rename different speakers; assert both names present (will fail). Interleave `put_diarization` with new `num_speakers`.

---

### 14. Migrations: per-file tx is good; several version/race holes remain
- **Finding / Hypothesis:** `run_migrations` (`migrations.rs`) applies each `(version, sql)` in its own `transaction()`, then `PRAGMA user_version = {version}` via `format!` inside that tx. `Store::open` reads `user_version` separately to decide backup, then calls `run_migrations` which reads it again. No downgrade path; if `current > LATEST_VERSION`, the loop no-ops and the process runs against an unknown schema. SQL files are `include_str!` and not shown; `execute_batch` will honor embedded `BEGIN`/`COMMIT`.
- **Rationale:** (1) Nested `COMMIT` in a migration file commits DDL **before** `user_version` bump; a later statement failure leaves schema mutated at old version → next start re-runs the same SQL (`CREATE TABLE` without `IF NOT EXISTS` fails; `ADD COLUMN` may succeed twice or fail). (2) Two app versions opening the same file: both pass the backup check, both migrate; SQLite serializes writers, but `.bak` can be the already-upgraded DB (finding 1). (3) Opening a v7 DB with a v6 binary silently skips migrations and then `prepare` fails on missing columns — or worse, succeeds with wrong column ordinals in `row_to_*`. (4) `PRAGMA user_version` is header metadata; it is transactional in SQLite, but mixing it with `execute_batch` that itself starts transactions is a classic split-brain. (5) Failure at v5 after v4 committed: memory v2 half-not-applied; retry is OK **unless** v5 is non-idempotent DML.
- **Verification Method:** Unit-test `run_migrations` against a temp file with a fake SQL containing `BEGIN; CREATE TABLE t(x); COMMIT; SELECT fail`. Read `user_version` and `sqlite_master`. Open a DB with `PRAGMA user_version=99` and assert a loud error (today: `Ok(99)` and later query errors). Concurrent `Store::open` during 0002 FTS build; measure `.bak` `user_version`.

---

### 15. `approve_candidate` / candidate status APIs can fork memory items; indexer FKs are an unshown landmine
- **Finding / Hypothesis:** `set_candidate_status` can set any string with no CHECK that the row is still `pending`. `approve_candidate` is the only path that mints `memory_items` in a tx. After `approved`, a caller can set status back to `pending` and `approve_candidate` again → second item. `insert_candidate` has no uniqueness on `(profile_id, text)`. If `0003_memory.sql` / `0005` ever `REFERENCES sessions(id) ON DELETE CASCADE`, `replace_session`’s `DELETE FROM sessions WHERE id=?` would wipe user memory on every reindex (including crashed heals), contradicting the comment “indexer never touches them.”
- **Rationale:** User-owned data is the one thing the rebuild invariant cannot restore. Status holes duplicate facts in AI context (`list_memory_items` cap `-1`). An FK cascade is the worst silent data-loss bug this layer can have; it would not show up in `replace_session` tests because those tests do not insert memory rows before reindex.
- **Verification Method:** `approve_candidate` → `set_candidate_status(id, "pending")` → `approve_candidate` again; count items. Reindex a session that is `source_session_id` for a candidate/item; assert rows remain. `PRAGMA foreign_key_list(memory_candidates)` / `memory_items` on a migrated DB.

---

### 16. Unbounded journal channel + ignored send errors = silent event loss / RAM growth
- **Finding / Hypothesis:** `mpsc::unbounded_channel`; `Journal::write` does `let _ = tx.send(WriterCmd::Line(line))` after serialize. Slow/blocked `writeln!` (network profile dir, disk full) lets the queue grow without backpressure. If the worker already exited (`None` from `blocking_recv` path only flushes and returns), later `send` fails and is ignored; counters still bump (`bump_counters` runs **before** send).
- **Rationale:** `snapshot_counters` / `SessionSummary` then over-report transcript/AI counts vs JSONL. Indexer trusts the file, so catalog under-counts vs in-memory UI. Process kill loses the entire unbounded queue (not just the last line). Shutdown timeout (`safe_to_join = false`) **drops the `JoinHandle`** (detach) while the worker may still be blocked in IO; a later `open_new_session` can prune or collide (finding 6) while the old worker still holds the handle.
- **Verification Method:** Pause the writer thread (debug break on `writeln!`); flood `write`; watch RSS. Kill worker; continue `write`; compare counters vs file lines. Call `shutdown(Duration::from_millis(1))` during a blocking write; assert handle detach + second `open_new_session` on Windows sharing violations.

---

### 17. Indexer parses with `lines()` + `Value`; several event classes never become catalog state
- **Finding / Hypothesis:** Unknown `kind` and JSON errors are skipped (`_ => {}`). `SessionSummary`, tiles, errors, detector, costs on anything except `ai_response.cost_microcents` are ignored. `num()` only `as_i64` / `as_u64`; JSON floats become `0`. Duplicate `session_start` last-wins; `ai_model` only updated if non-empty. `read_dir` uses `flatten()`, silently dropping unreadable directory entries.
- **Rationale:** A journal whose only durable “finished” marker were a summary (writer killed after summary, before stop — both are separate channel messages in `emit_summary_and_stop`) still indexes as `crashed` and **drops summary cost**. Float timestamps from an older writer zero `unix_ms`, scrambling `ORDER BY unix_ms` and BM25 hit times. `has_event` stays false for a file of only future kinds → `Ok(None)` → no row (G3), so an upgraded writer + old binary looks like “empty session” forever until a recognized kind appears. `flatten()` can skip a `.jsonl` on a transient FS error; that session disappears from a rebuild sweep without `failed++`.
- **Verification Method:** JSONL with only `session_summary` + costs; assert catalog cost 0 and `crashed`. Line with `"unix_ms": 1.0`; assert `unix_ms == 0`. Directory entry that errors (`chmod` / dangling symlink); assert `scanned` omits it. Pair `emit_summary_and_stop` with a writer abort between the two `write` calls.

---

### 18. WAL checkpoint policy: TRUNCATE on migrate, never on indexer close; 2s busy timeout vs huge FTS rewrite
- **Finding / Hypothesis:** Only checkpoint is the ignored `wal_checkpoint(TRUNCATE)` in `open`. `replace_session` can rewrite all utterances + FTS for every `crashed` session on **every** `index_all` (intentional G1). No `wal_autocheckpoint` override, no post-sweep checkpoint, `Store` drop does not `TRUNCATE`.
- **Rationale:** `-wal` grows without bound if crashed sessions are large and the archive UI refreshes often. Readers stay consistent, but the next migrate backup (finding 1) copies a huge WAL-skewed main file; `busy_timeout=2000` is more likely to fire during checkpoint or during FTS insert of tens of thousands of trigger rows. In-memory `open_in_memory` does not set WAL/timeout, so tests never see this.
- **Verification Method:** Create many `crashed` journals with large utterance counts; loop `index_all`; measure `-wal` size and time spent in `replace_session`. Concurrent `list_sessions` from a second `Store`; count `SQLITE_BUSY`. After loop, `PRAGMA wal_checkpoint(PASSIVE)` vs `TRUNCATE`.

---

### Highest-probability user-visible failures (priority)
1. Kill/power-loss → missing `session_stop` / tail events (no `fsync`) and possible **whole-file** index failure on torn UTF-8.
2. Retention prune / UTF-8 failure / `skip_active` miss → archive lies (orphans, holes, live session marked crashed).
3. Dual-writer `SQLITE_BUSY` during stop + archive open → that session’s heal/index skipped with a log line.
4. Catalog file delete “to rebuild” → **permanent** loss of memory + diarization; `.bak` may be junk.
5. Same-ms journal path collision → irretrievably merged session.

I did not see the SQL under `overlay-backend/migrations/` or `journal/recovery.rs`; findings 11 and 14–15 should be confirmed against those files (`IF NOT EXISTS`, FTS `content=`, FKs, embedded `COMMIT`).
