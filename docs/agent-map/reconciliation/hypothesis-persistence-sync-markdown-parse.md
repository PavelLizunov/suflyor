# Original persistence C04 / tile C02: bounded journal fsync and streaming markdown parsing evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_persistence_sync_markdown_parse_confirmed.py) inspect frozen JSONL journal writer flushing and tile markdown parser logic. They do not perform live file writes, force OS flushes, or stream live UI tokens. C04 and C02 remain confirmed mechanisms.

## C04 — `BufWriter::flush()` without physical `fsync` / `sync_all`

In `overlay-backend/src/journal/writer.rs`:
- In [spawn_writer](<../../../overlay-backend/src/journal/writer.rs#L277-L323>), the background loop writes lines and executes `file.flush()` upon receiving lines or shutdown commands;
- In [finish_writer](<../../../overlay-backend/src/journal/writer.rs#L261-L274>), final shutdown executes `let flush_result = file.flush()`;
- Neither path calls `file.get_ref().sync_all()` or `sync_data()`. Durability acknowledges that data has moved from userspace buffers to the operating system's page cache, but does not force a synchronous write to physical non-volatile storage.

## C02 — Streaming markdown re-parsing and unbounded collection

In `slint-experiment/src/markdown.rs`:
- [parse_streaming](<../../../slint-experiment/src/markdown.rs#L67-L73>) calls `parse(stable_streaming_prefix(source))` on each token emission interval, running the entire commonmark pipeline from scratch over the whole accumulated prefix;
- [parse_single_pass](<../../../slint-experiment/src/markdown.rs#L218-L270>) instantiates a fresh `Parser::new_ext` over the entire source buffer;
- Table parsing accumulates rows into `table_rows: Vec<Vec<String>>` with no row cap (only cell width is clamped to 28 characters);
- Nested list items indent by `list_depth` via `"  ".repeat(list_depth - 1)` without a fixed recursion ceiling.

## Limits

No sudden power interruptions or OS kernel panics were induced to test un-fsynced write loss, and no excessive multi-megabyte markdown streams were rendered live. Original statuses in `candidates.json` remain `confirmed`.
