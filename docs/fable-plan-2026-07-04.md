# Fable audit + plan — suflyor memory/transcript/select (2026-07-04)

Response to `docs/fable-brief-2026-07-04.md`. `/ponytail full`. Owner reviews before any code.

## AUDIT (issue D) — real bugs in committed code

- **D1 — offline == terminal (A1 root cause, in code).** `normalize.rs` `ai::complete(...).await.ok()?`
  discards the error kind; `tile_copy.rs finalize_normalized(...,"heuristic")` on `None` is TERMINAL.
  "server off" (conn-refused ~7s) is recorded identically to "AI ran, all facts rejected" → never
  revisited. Feature is structurally silent offline; no later sweep can help because the retryable
  state was erased.
- **D2 — `has_negation` kills RECALL even when AI is ON (2nd root cause of "не увидел").**
  `(t.starts_with("не")||"ни") && len>=5` flags «неделя, несколько, низкий, Николай» as negation.
  Worse, parity is source-GLOBAL vs fact-LOCAL: real STT is full of «не знаю/не помню» filler, so ANY
  extracted fact without its own negation word is REJECTED (source «я не знаю, сервер Альфа продакшн»
  → fact «сервер Альфа продакшн» → REJECT → heuristic). With D1 → «почищено»/unchanged on nearly
  every save regardless of server state.
- **D3 — user-edit clobber race.** `update_memory_item_text` sets only `text`, leaves
  `norm_status='pending'`; a background finalize during the (up to ~9.5 min) window overwrites the
  manual edit. One-line fix: edit UPDATE also sets `norm_status='none'` (guard then no-ops).
- **D4 — orphaned `'pending'`.** Detached thread; crash mid-flight leaves the row «normalizing…»
  forever. Same family as D1; the A1 sweep heals it.
- **D5 — the i16 story is FOLKLORE.** Renderer is **Skia (GPU)**, not the software renderer
  (`Cargo.toml`). All the "SW-renderer i16=32767px" comments (tile_copy, transcript, settings_panel)
  cite a limit this app doesn't use. `build_select_text`'s assert proves ≤~29200<32767 yet the window
  still broke → real ceiling is lower (likely GPU max texture 16384px). **Cap arithmetic is guesswork
  against an unverified limit — stop building tall single elements instead of re-tuning caps.**
- **D6 — ListView virtualization: right in principle, unverified.** Bounds instantiated elements, but
  large VIEWPORT OFFSETS untested: 2000 rows × ~22px ≈ 44000px > 32767. Tail "плохо кликаются" is
  consistent with either SelectableText swallowing hit-zones + C2 overlap, OR real hit-test
  misalignment at large offset. 10-min check: synthetic 5000-row session, scroll to tail, click-test.
- **D7 — C1 root cause: display order ≠ displayed clock.** Rows `ORDER BY unix_ms` (STT-FINALIZE wall
  clock); timecode shown is start-based `audio_ms`. 11-min inversion (21:29 before 21:18) can't be
  overlap (VAD splits at seconds) → **per-channel epoch skew**: `audio.rs` stamps chunks with
  per-capture-loop `start_ts.elapsed()`; mic + system loops have independent epochs; a loop
  restarting mid-session resets its epoch. If confirmed, that channel's player seeks are also wrong.
- **Sound (no action):** rowid-reuse guard; WAL+busy_timeout+AI_SEMAPHORE concurrency; parse_facts;
  heuristic_clean; identifier whole-token equality. **Minors:** badge says «cleaned» even when
  heuristic changed nothing; `entity` stored UNGATED (only model output bypassing all checks);
  `tile_copy.rs` uses `ai_endpoint(true)` where ADR said `false` — with provider=cloud this sends raw
  STT spans to the cloud bridge (egress — get explicit owner OK).

## PLAN (prioritized)

- **P1 — C2 scrollbar-over-⭐ (mechanical, first).** Row `padding-right: 4px` → ~18px (reuse tiles'
  14px inset). 1 line, transcript.slint. Skip: restyling the scrollbar.
- **P2 — C1 timecode order (diagnose then sort).** Run on owner DB:
  `SELECT unix_ms-(started_at_ms) AS wall_off, audio_ms, source, substr(text,1,30) FROM utterances
  WHERE session_id=? ORDER BY unix_ms DESC LIMIT 40;` — constant-per-source but minutes-apart between
  sources ⇒ epoch skew (H1); adjacent-only inversions ⇒ overlap (H2). Either way: stable-sort rows +
  utts by the DISPLAYED clock at the load site (aux_windows.rs, precompute offsets in original order
  first — fallback reads prev row). Only if H1: share ONE epoch across capture loops (audio.rs) — also
  fixes seeks. Skip: changing SQL ORDER BY (other consumers want finalize order); any fix before the query.
- **P3 — A1 offline reliability (semantic + sweep).** `'pending'`=retryable, `'heuristic'`=AI declined
  (terminal). `normalize_fact -> Result<Option<..>>`: `Err`(transport)→leave `'pending'`;
  `Ok(None)`→`'heuristic'`; `Ok(Some)`→`'llm'`. `sweep_pending()` (one thread, factor `condense_one`):
  filter `list_memory_items` for pending in Rust, process sequentially, ABORT on first `Err`, AtomicBool
  guard; triggers = boot (~15s) + Memory-tab open. Heals D4. Honest badges («ждёт ИИ»; «почищено» only
  if text≠source_text). Ride-along D3 one-liner + confirm prep:true/false. Skip: retry counters/new
  columns, health-poll service, live refresh, toasts.
- **P4 — A3 (subsumes A2): QUOTE-SPAN EXTRACTION (the real design decision).** Verdict: extractive-
  prompt + bag-of-words gate is a local maximum — tighten→recall dies (D2), loosen→lies return, and NO
  bag check closes recombination («сервер Бета продакшн» has a perfect bag). Closable only STRUCTURALLY:
  **model returns 1-3 VERBATIM QUOTES (contiguous fragments), not free text.** Rust `locate_span()`
  finds each as a contiguous substring of the exact text the model saw (casefold + ws-collapse, snapped
  to whole-token boundaries — kills truncated-IP for free) and STORES THE SOURCE SLICE, never the
  model's text. ⇒ grounding trivial+absolute; recombination IMPOSSIBLE by construction (a fact is one
  contiguous slice); negation local (check the 1-2 tokens before the span for {не,ни,нет,без,нельзя});
  gate `entity`; require spans ≥2 tokens or ≥1 identifier. Net diff NEGATIVE: delete `is_grounded`,
  `has_negation`, `source_contains`, `is_identifier` (~150 lines) + add `locate_span` (~50) + prompt
  rewrite; keep shape caps, parse_facts, the tile_copy pipeline. Cost: facts keep source word order,
  mid-span filler stays. Residual (accept+doc): semantic inversion w/o local negation (sarcasm,
  «если бы»). Files: normalize.rs only. Skip: NLI (~1GB sidecar, probabilistic — worse than a
  structural guarantee), 2nd-LLM grader (self-grade spirit), embedding sim (order-blind).
- **P5 — B: delete select-mode, add shift-click block-range.** Single huge selectable field is
  unbuildable under Slint 1.16 (no TextInput virtualization, no drag auto-scroll, D5 cap failed).
  Delete Option A (`select-mode`/`select-text` + 2nd ScrollView + `build_select_text`); KEEP
  `capture_selection` for idx≥0 (per-block precise). Add SHIFT-CLICK range marking on ⭐ (modifiers via
  pointer-event; mark contiguous block range from anchor; reuse `marked` model + «В память (N)» bar) —
  scrolling between clicks is native, no drag = no auto-scroll problem. Add «Копировать» to the mark bar
  (`join_marked_text` exists). Named loss: one-gesture char-precise selection ACROSS blocks (the exact
  thing the platform can't render safely). Files: tile.slint, tile_copy.rs, .po. Skip: drag-to-mark,
  field pagination, N≥2 trim editor. Fallback if owner insists on the single field: halve caps (≤6k
  chars/250 lines) as a stated stopgap.

## Skipped overall
M2-M5 memory ADR (retrieval/merge/embeddings); bounded worker pool / health-poll / retry schema /
error-string classification; virtualizing the (capped) Settings memory list; scrollbar restyle; toasts;
any C1 code fix before the diagnosis query.
