# Original persistence C05/C06: bounded journal write and identity evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_journal_write_identity_hypotheses.py) inspect frozen writer source and use private temporary JSONL files. No owner journal, catalog, or Rust writer fault was executed. Original C05/C06 remain hypotheses.

## C05 — a failed line does not stop later lines

[Writer loop](<../../../overlay-backend/src/journal/writer.rs#L277-L340>) logs a write error and continues. [First-error latch](<../../../overlay-backend/src/journal/writer.rs#L254-L275>) keeps the first error; [finish](<../../../overlay-backend/src/journal/writer.rs#L200-L228>) returns that error ahead of a later flush result. This is source order, not an injected disk-full failure.

A temporary file containing a newline inside one otherwise JSON value shows the indexer-shaped boundary: `str.splitlines()` plus `json.loads` skips the broken physical line and accepts the following complete line. The broken payload consumes the next physical line, so “middle tear” is not automatically two independently skipped lines. No product disk-full or partial `write_all` was reproduced.

## C06 — identity is not exclusive

[Session open](<../../../overlay-backend/src/journal/writer.rs#L70-L97>) uses `create(true)` and `append(true)`, not `create_new`. [Clock suffix](<../../../overlay-backend/src/journal/time.rs#L3-L16>) uses a one-second stamp plus only the low 24 bits of the same millisecond clock. Equal inputs therefore produce one filename.

Opening one temporary file twice in append mode preserved both dummy sessions, but kernel buffering grouped writes rather than strictly alternating lines. The collision/append mechanism is demonstrated; exact byte interleaving is not a stable contract. No concurrent Rust session-open race or owner-file collision was run.

## Limits

These fixtures do not execute the Rust writer, force ENOSPC, or race two processes at the same millisecond. They also do not change `candidates.json`: both original IDs remain `hypothesis`.
