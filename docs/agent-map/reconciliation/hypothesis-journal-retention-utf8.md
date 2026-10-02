# Persistence C03/C17: bounded retention and UTF-8 evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_journal_retention_utf8_hypotheses.py) use temporary files and frozen source. They do not run Rust retention or read owner journals. C03 stays confirmed; C17 stays a hypothesis.

## C03 — UTF-8 failure precedes line skipping

[Indexer read](<../../../overlay-backend/src/persistence/indexer.rs#L32-L66>) calls `read_to_string` before iterating lines. A temporary file with one valid JSON line, a `0xFF` byte, and another valid line raises `UnicodeDecodeError` at whole-file decoding. Decoding only the first physical line still yields its JSON event, so the line parser itself is not the failing boundary.

## C17 — count cap and byte cap are sequential

[Retention](<../../../overlay-backend/src/journal/retention.rs#L5-L55>) keeps the newest 100 JSONL files, then applies a 500 MB cap only when `max_bytes > 0`. Metadata and mtime errors are skipped. The sort key is file mtime, not a timestamp inside the journal.

The temporary model keeps the newer file at count cap 1, then the one-byte second cap deletes it too. A zero byte cap disables that second pass. Non-JSONL files remain. This is a Python model of the source order, not Rust filesystem execution.

## Limits

No owner session directory, Rust retention process, or indexer execution was run. Original statuses and 39/75/5 remain unchanged.
