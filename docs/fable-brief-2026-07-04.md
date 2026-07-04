# Fable brief — suflyor memory + transcript (2026-07-04)

**Purpose:** hand fable ONE brief to (1) **review** the current memory-condensation + transcript
implementations for soundness, and (2) **design** fixes for the open issues below. This file is
context; the ready-to-paste **PROMPT FOR FABLE** is at the very bottom.

`/ponytail full` is active — the design must prefer the laziest correct option: reuse what exists,
fewest files, no speculative abstraction, deletion over addition. Name what to SKIP.

---

## Stack / where things live (so fable can navigate)

Pure **Rust + Slint 1.16**, Windows overlay. THREE crates, NO workspace:
- `overlay-backend/` — no-UI lib. Memory logic in `src/memory/` (`normalize.rs`, `context_builder.rs`,
  `candidates.rs`, `summary_ref.rs`), storage in `src/persistence/` (`sqlite_store.rs`, `models.rs`,
  `migrations/`), AI in `src/ai.rs`, config in `src/config.rs`.
- `slint-experiment/` — the `overlay-host` binary. UI in `ui/*.slint`; host glue in a ~26-module dir
  `src/bin/overlay_host/`.
- `suflyor-tts/` — TTS sidecar (irrelevant here).

Gate: `scripts/ci.ps1` (fmt + clippy -D + tests ×3 + i18n). Local AI = external llama.cpp on
`127.0.0.1:8080` (gemma-4-12B), often OFF between sessions. AI entry: `ai::complete(base,bearer,model,
msgs,max)` (async, no-think); endpoint via `config.ai_endpoint(prep:bool)`.

## What's built this week (the thing to review)

**Memory formatting (feature "condense", M1):** every ⭐/selection save (tiles + transcript) →
`tile_copy::insert_approved_note(text)`:
1. store the heuristic-clean instantly (`memory_items.norm_status='pending'`, raw kept in
   `source_text`; migration `0005`), so the ⭐ feels instant;
2. a background `std::thread` + current-thread tokio runtime runs
   `memory::normalize_fact(raw, base, bearer, model)` →
   - `heuristic_clean` (drop ws + ≥4-letter stutter repeats),
   - `ai::complete` with `CONDENSE_SYSTEM` (EXTRACTIVE prompt: extract 1–3 short facts using the
     SOURCE'S OWN WORDS, drop filler, don't paraphrase; keep identifiers + «айпи» verbatim; same
     language),
   - `parse_facts` (all facts from `{"facts":[{entity,text}]}`, tolerates ```json fences),
   - keep only facts that pass `is_grounded(raw, fact)` (STRICT: identifiers = whole-token verbatim,
     ≥90% content-word containment, negation parity, shape ≤400, no-new-identifier), join ≤3 with «; »,
   - `store.update_memory_item_normalized(id, source_text, text, entity, 'llm')` guarded by
     `WHERE id=?1 AND norm_status='pending' AND source_text=?2` (rowid-reuse guard);
   - on any AI/parse/all-rejected failure → keep heuristic text (`norm_status='heuristic'`).
3. Настройки→Память shows a badge: `'llm'` → «подчищено ИИ · было: <raw>»; `'heuristic'` → «почищено»
   (no raw, to avoid duplicating an unchanged fact); `'none'`/typed «свой факт» → nothing.

**Transcript viewer:** `aux_windows::open_transcript` builds a `[TranscriptLine]` model (cap now 2000,
memory-sanity only); `ui/transcript.slint` renders it in a **virtualized `ListView`** (was a capped
`for`-in-`ScrollView`; the cap existed only for the SW-renderer i16=32767px limit — ListView
instantiates only visible rows so the cap is gone). Per-line: checkbox, timecode (from
`session_audio::line_start_offset_ms`), speaker, `SelectableText`, ⭐-mark. "Copy all" exports all.

Also shipped (not under review): bar 🔒 lock chip (toggles `config.suppress_tiles`), tile close-button
overflow fix.

---

## OPEN ISSUES (owner's live retest, 2026-07-04) — all in scope

### A. Memory — condensation reliability + safety (deepest; architecture)
- **A1 — INVISIBLE / UNVERIFIABLE when the local AI is OFF.** Owner tried 3 sessions and never saw
  condensation ("явно не увидел что работает"). Root cause almost certainly: `:8080` was down →
  `ai_endpoint` base_url reachable-check fails or `ai::complete` errors → every save falls to
  `'heuristic'` (≈ the raw text, unshortened). The feature is silent when the AI is offline. NEED: a
  design so memory-formatting is reliable + visible — e.g. run/queue condensation when the AI comes
  online (a pending `norm_status='pending'` sweep on next boot / when the server is detected), and/or
  make "not condensed because AI offline" visible + actionable, without blocking the instant save.
- **A2 — RECOMBINATION residual (the real safety hole).** `is_grounded` is bag-of-words: it verifies
  every word of a fact is present in the source, but NOT that they're combined faithfully. It CANNOT
  catch "attach a real subject to the wrong predicate" — e.g. source «сервер Альфа продакшн, сервер
  Бета тестовый» → fabricated «сервер Бета продакшн» passes (0 missing words). The extractive prompt
  reduces this, but doesn't close it. An adversarial review (transcript in-session) confirmed this
  class. NEED: a design to catch recombination/faithfulness that is NOT LLM-self-grading (the owner's
  methodology bans self-preference; a verifier must be independent/deterministic or at least an
  independent grader). Options to weigh: a cheap NLI/entailment check, a span-provenance constraint
  ("each fact must be a contiguous-ish slice of the source"), a second independent model, or accepting
  the residual with clear scoping. Pick the laziest defensible one.
- **A3 — the gate is tuned by trial-and-error.** Prior attempt loosened content-containment to ≥50%
  and a review showed single-word lies slipping («перезагружен»→«взломан»); reverted to strict ≥90% +
  extractive prompt. fable should sanity-check this whole approach: is EXTRACTIVE-prompt + strict-gate
  the right architecture, or is there a cleaner model (e.g. purely extractive span selection with no
  free-text generation, so grounding is trivial)?

### B. Slint — "select text" on a LARGE answer overflows (owner bug #1)
- The tile/summary "Выделить текст" mode puts the whole answer in ONE `SelectableText`
  (read-only `TextInput`) inside a `ScrollView` (`tile.slint`, per-row `max-height:120px` is for the
  BLOCK view, not this whole-answer field). On a long answer the window "уходит за лимит" (the i16
  32767px SW-renderer limit — same class as the transcript cap) AND drag-selection doesn't auto-scroll
  the ScrollView (Slint `TextInput` limitation), so off-screen text selects blind. NEED: a robust
  design for cross-block text selection on arbitrarily long answers under Slint 1.16 (fable designed
  the original selection model — TextInput offsets, ContextMenuArea; see its earlier work). Consider:
  virtualize, or a different selection model, or cap+page the selectable field.

### C. Transcript — two bugs the ListView exposed (owner bugs #2, #3)
- **C1 — timecodes jumbled at the tail.** With all lines now visible (ListView), the LAST rows show
  out-of-order times (e.g. `21:29 «Слабие.»` BEFORE `21:18 «I not lak Radient Victory.»`). Likely a
  data/ordering bug in `session_audio::line_start_offset_ms` (or the utterance order) that was hidden
  while the tail was capped off. Also "плохо кликаются" (row play-line seek hard to hit at the tail).
- **C2 — scrollbar overlaps the ⭐ column.** The `ListView` scrollbar sits on the right edge, on top of
  the per-line ⭐ (right-aligned). Tiles already solved this (a gutter/inset so ⭐ isn't under the
  scrollbar); the transcript didn't get that inset. (This one is likely a small mechanical fix — owner
  wants it in the brief for completeness.)

### D. Verify the implementations
Owner: "проверить реализации." fable should AUDIT, not just design: does the condense async flow have
races/leaks? is the rowid-reuse guard correct? does ListView virtualization actually hold under a
5000-line transcript (no i16)? is the observability badge logic right? Flag anything unsound in what's
already committed (commits this week: `1a84bf4`, `3ea23fe`, `c384e0e`, `e605872`, plus tile-lock).

---

## Anchors
- Memory: `overlay-backend/src/memory/normalize.rs` (`heuristic_clean`, `is_grounded`, `normalize_fact`,
  `parse_facts`, `CONDENSE_SYSTEM`); `slint-experiment/src/bin/overlay_host/tile_copy.rs`
  (`insert_approved_note`, `store_note`, `finalize_normalized`); `overlay-backend/src/persistence/
  sqlite_store.rs` (`insert_memory_item`, `update_memory_item_normalized`, `list_memory_items`);
  `overlay-backend/migrations/0005_memory_v2.sql`; `slint-experiment/ui/settings_panel.slint` (memory
  list + badge, ~2790); `settings_memory.rs` (`item_row`).
- Transcript: `slint-experiment/ui/transcript.slint` (ListView, rows, ⭐, `SelectableText`);
  `slint-experiment/src/bin/overlay_host/aux_windows.rs` (`open_transcript`, `TRANSCRIPT_DISPLAY_CAP`);
  `overlay-backend/src/session_audio.rs` (`line_start_offset_ms`).
- Select-mode: `slint-experiment/ui/tile.slint` (select-mode ScrollView + SelectableText);
  `slint-experiment/ui/controls.slint` (`SelectableText`).
- Memory ADR (prior fable design): `docs/memory-architecture.md`.

---

## PROMPT FOR FABLE

> You are designing for **suflyor** (pure Rust + Slint 1.16 Windows overlay; read
> `docs/fable-brief-2026-07-04.md` and `docs/memory-architecture.md` first, then the anchor files).
> `/ponytail full` is in force: laziest correct design, reuse existing code, fewest files, no
> speculative abstraction; explicitly name what to SKIP and why.
>
> Do TWO things:
> 1. **Audit** the memory-condensation + transcript implementations shipped this week (issue D +
>    anchors) — call out anything unsound (async races, the rowid-reuse guard, ListView i16 safety,
>    badge logic, the extractive-prompt + strict-gate architecture).
> 2. **Design** fixes for the open issues, prioritized, each with a concrete approach + the files it
>    touches + what you'd skip:
>    - **A (memory):** reliable + observable formatting when the local AI is intermittently OFF
>      (A1); catching RECOMBINATION-into-falsehood without LLM-self-grading (A2, the owner's hard
>      constraint); and whether extractive-prompt+strict-gate is the right architecture or a purely
>      extractive span-selection model is cleaner (A3).
>    - **B (Slint):** robust cross-block text selection on arbitrarily long answers under Slint 1.16
>      (window overflows i16 + no drag-scroll).
>    - **C (transcript):** the jumbled-timecode tail (C1, likely `line_start_offset_ms`/ordering) and
>      the scrollbar-over-⭐ overlap (C2, tiles already solved it — reuse that).
>
> Output: a prioritized plan (what to fix first, what to skip/defer, what's a real design vs a
> mechanical fix), grounded in the actual code — not generic advice. Do NOT write the implementation;
> the owner reviews your plan first.

---

## Status when this brief was written
All committed + gate-green, NOT pushed. Installed build (PID at the time) running. Owner said "не
правим" — this brief is prep only; launch fable on the owner's explicit «го».
