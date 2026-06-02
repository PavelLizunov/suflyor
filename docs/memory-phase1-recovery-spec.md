# Memory — Phase 1: crash recovery (no SQLite) — implementation spec

Scope of the FIRST shippable slice of `docs/personal-memory-and-session-store-architecture.md`.
Phase 1 is **decision-free** (the doc's 6 open decisions are about archive/SQLite/
embeddings — none gate recovery). Backend detection is unit-testable; the only UI
surface is a small "recover context?" offer at startup.

## Goal
On launch, detect a session that ended WITHOUT a clean stop (crash / force-kill /
power loss) and offer to carry its context into the new session — instead of
starting cold and losing the user's place mid-interview.

## Backend (overlay-backend/src/journal.rs) — pure + testable
The journal already writes per-session JSONL files with events incl.
`SessionStart` and `SessionStop`/`SessionSummary` (see journal.rs). Add:

- `pub struct UnfinishedSession { pub session_id, pub path, pub started_unix_ms,
  pub last_lines: Vec<String>, pub last_qa: Option<(String,String)>,
  pub summary: Option<String> }` — REDACTED-friendly (no secrets; transcript lines
  are user content, acceptable for the user's own recovery).
- `pub fn find_unfinished_session(journal_dir) -> Option<UnfinishedSession>`:
  1. enumerate `*.jsonl`, newest first by mtime;
  2. for the newest, parse events; if it has a `SessionStart` and NO `SessionStop`
     (and no terminal `SessionSummary`), it's unfinished;
  3. extract: last N transcript lines, the last completed Q&A (last AiRequest +
     its AiDone/answer), and any local summary if present;
  4. return it. Skip files older than e.g. 12h (stale → don't nag).
- Idempotent + side-effect-free (no writes). Tolerate malformed/truncated trailing
  JSONL lines (a crash often truncates the last line) — parse line-by-line, skip
  unparseable lines, never panic.

### Tests (the doc explicitly asks for stop/crash/restart)
- graceful stop: a JSONL WITH SessionStop → `find_unfinished_session` returns None.
- crash: a JSONL with SessionStart, some lines, NO SessionStop → returns Some with
  the right last_lines/last_qa.
- truncated tail: last line is half-written JSON → still parses the rest, no panic.
- stale: an unfinished JSONL older than the cutoff → None.
- empty dir / only-Start-no-lines → sensible (None or empty fields, no panic).

## Wiring (overlay_host.rs startup)
- After the bar is up (mirror the first-run wizard's ~2.2s delayed open), if
  `find_unfinished_session(journal_dir)` is Some, spawn a small RECOVER offer:
  a tile (or a reuse of the help/text-ask window pattern) titled e.g.
  "↩ Восстановить прошлую сессию?" showing the last Q&A + a couple transcript
  lines + Recover / Dismiss. Stealth-aware + park-before-show (present_window_
  stealth_aware) like every other on-demand window.
- On Recover: seed the new live session's context with the recovered profile +
  last lines + last answer; link via a `recovered_from_session_id` field written
  into the new session's SessionStart event. Do NOT auto-recover: in-flight
  network request, mic recording, screenshot payload, open tiles 1:1, streaming
  state (per the doc).
- On Dismiss: nothing (the old JSONL stays on disk; pruning is the journal's job).

## Constraints
- SECURITY: transcript lines are the user's own meeting content — fine to show in
  THEIR recovery UI, but the offer window must respect stealth (WDA) so it isn't
  visible on a screen-share. No secrets/keys in any of this.
- i18n: every new visible string `@tr()` + ru.po pair.
- Glyphs: emoji only (↩), no bare text-glyphs.
- No unwrap/expect/panic outside #[cfg(test)] (both crates deny these).
- Phase 1 ONLY — no SQLite, no rusqlite, no embeddings, no archive settings.

## Gate
clippy + tests both crates, fmt, boot smoke (app starts, and with a hand-seeded
unfinished JSONL the offer appears + Recover/Dismiss work). The live visual of the
offer window needs the user's eyes (defer that check to the morning per the run's
visual-verify constraint).
