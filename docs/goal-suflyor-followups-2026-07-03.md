# Goal — suflyor follow-ups (2026-07-03): layout-independent shortcuts + связная память + transcript parity

## ⏭ NEXT SESSION (owner paused 2026-07-04 «завтра продолжим») — condense open issues

> **RESOLVED 2026-07-04 (continuation):** all three done + gate-green.
> **(1)** A focused review proved loosening the gate to ≥50% was UNSAFE (single-word lies like
> «перезагружен»→«взломан» passed — bag-of-words can't tell a synonym from a lie, and a % threshold
> can't catch one lie in a long fact). So instead of loosening the gate, I made the CONDENSE prompt
> **EXTRACTIVE** (keep the source's WORDS, drop only filler) and kept `is_grounded` STRICT (≥90%) —
> a faithful extraction passes, a paraphrase/fabrication is rejected → heuristic. This ALSO fixes
> the айпи→IP false-reject (prompt now says keep «айпи»). **RESIDUAL (documented in the is_grounded
> doc):** bag-of-words still can't catch RECOMBINATION of real words into a false claim (e.g. «сервер
> Бета продакшн» when Бета was the test server) — closing that needs an entailment check (future,
> and NOT LLM-self-grading per methodology). **(2)** `было:` now shows only for `'llm'` items
> (heuristic shows just «почищено», no duplicated raw). **(3)** transcript → virtualized `ListView`
> (renders only visible rows → no i16 cap → all lines show; the 200-cap is now a 2000 memory-sanity
> bound). ⚠ Owner retest still needed (their AI must be up for condense; ListView is a render change
> — confirm a long transcript scrolls + ⭐/selection work + no crash). Retest:
> `docs/retest-tile-lock-normalization.html` (adds X2 transcript). Select-mode drag-scroll on huge
> answers still deferred (Slint TextInput limitation; condense reduces its need).


Feature A (condense — commits `8b1e7fe`+`9de8fbc`) is LIVE + installed (PID at the time). Verified
on the owner's real gemma-4-12B: a long **tile** answer → 3 clean short RU facts ('llm'). BUT the
owner's live test surfaced TWO issues to fix next:

1. **Condense REJECTS paraphrased colloquial STT → falls back to heuristic ("почищено"), so the
   text isn't shortened + looks DUPLICATED** (screenshot: memory items show the full ramble AND
   `было:` = the same ramble). ROOT CAUSE: `is_grounded`'s content-word containment (≥90% of the
   fact's words must be IN the source) is right for NORMALIZE (minimal rewrite) but TOO STRICT for
   CONDENSE — summarizing colloquial speech PARAPHRASES (жрёт→ест, synonyms) → containment <90% →
   every extracted fact rejected → `grounded.is_empty()` → None → heuristic. Works only when the AI
   REUSES source words (clean tile answers), fails when it must paraphrase (raw STT).
   **FIX (design first, maybe fable):** split the gate — for CONDENSE, keep the ANTI-HALLUCINATION
   core (every DIGIT identifier + name verbatim; NO NEW identifiers introduced; negation parity)
   but DROP/loosen the content-word containment so legit paraphrase passes. i.e. a separate
   `is_grounded_condense` (or a param) that allows reworded content but still forbids fabricated
   numbers/names. Then re-verify on the colloquial STT samples (the «кот/таблетки» + «биом» rambles
   in the screenshot).
2. **UI: `было:` duplicates the text for HEURISTIC items.** Only show `было: <raw>` when it
   MATERIALLY differs from the stored text (i.e. for 'llm'), OR drop it for 'heuristic'. Quick fix
   in settings_panel.slint (the `if m.norm-status == "llm" || "heuristic"` branch → split so
   heuristic shows just a subtle mark, no full `было:`).
3. **Transcript shows only a subset — "скрыто 17 строк"** (screenshot 1). The transcript display
   is capped (i16 SW-renderer guard, Баг5 class — un-virtualized tall content panics). «Копировать
   всё» gives the full text, but the owner wants to SEE all. **FIX:** virtualize the transcript
   list (Slint `ListView`, which renders only visible rows → no i16 panic → no cap needed), or
   raise the cap safely. Anchor: `aux_windows.rs open_transcript` + `transcript.slint`.

Also still open (lower prio): select-mode drag-scroll on huge answers (screenshot from prior turn;
Slint `TextInput`-in-`ScrollView` limitation; condense reduces the need). Everything is committed +
gate-green + NOT pushed. No release without «релизь».

---

Follow-up to the text-selection work (see `docs/goal-text-selection-2026-07-03.md`). The
selection ask (ТЗ 2026-07-03 **part 1**) is DONE + owner-verified (retest r1/r2). This goal
captures everything remaining, per owner 2026-07-03: "всё из перечисленного заверни в goal…
проблему с рус нужно решать комплексно, она же будет на других раскладках тоже".

## Done so far (committed, NOT pushed — accumulating)
- `8096671` P1 — tiles/summaries/archive selectable text + right-click Copy / Add-to-memory.
- `98565a5` P2 — transcript per-line selectable + shared `SelectableText` (controls.slint).
- `12ac032` D′ — block-height collapse fix (SelectableText root Rectangle + `height:
  ti.preferred-height`; ContextMenuArea reports 0 layout-info → collapse/overlap). Owner-verified.
- `b254f39` Option A — «Выделить текст» mode (cross-block selection in tiles; dual-capped
  join for i16) + muted transcript ⭐. Owner-verified r2 (E1-E6, Z1, A1 all OK).

## Remaining scope

### G1 — Keyboard-layout-independent shortcuts  [P0 — real bug, owner-hit]
**Bug (retest r2 D1):** Ctrl+V (paste) under the RU layout does NOT work (Ctrl+C copy now
does). Root: the shim matches RU chars (ф/с/м/ч), so it only fixes Russian — DE/FR/any
non-US layout will break the same way (owner: "языков же много").
**Need:** layout-INDEPENDENT Ctrl+C / V / X / A (and Z if cheap) that works on ANY layout,
in EVERY editable field: the memory capture editor `LineEdit` (tile.slint), «Продолжить
диалог» follow-up, Settings (incl. «Свой факт»), wizard, archive — AND copy/select-all on
the read-only `SelectableText` (controls.slint). Latin shortcuts must keep working.
**Open question (fable):** does Slint 1.16 `KeyEvent` expose a layout-independent key
(physical/logical), letting a `.slint` handler match the C/V/X/A KEY regardless of the
produced char? If not, must this be done at the Win32 level (VK_C/V/X/A + Ctrl via
WM_KEYDOWN / a low-level hook) and routed into the focused field? Design a reusable wrapper
(e.g. `LayoutSafeLineEdit` in controls.slint, per-field internal FocusScope — a single
shared FocusScope in multi-field windows can't tell which field is focused) + how to apply
it across the fields. NB the existing `text_ask.slint`/`palette.slint` char-shim is RU-only —
replace, don't extend.

### G2 — Связная память (ТЗ 2026-07-03 **part 2**)  [P1]
The ТЗ's second half — not yet started. Example: «Компьютерное имя z14-4443-backup / Подсеть
10.255.28.96/27 / IP 10.255.28.116» must be ONE record, not three.
- **G2a — N⭐ → ONE record.** Today the tile multi-⭐ `on_save_marked` writes each marked
  block as a SEPARATE note (`tile_copy.rs`). Join marked blocks into one `insert_approved_note`
  (newline/`; ` sep); show the edit buffer at N>1 too (tile.slint `if marked-count==1` →
  `>=1`). Diff ~15 lines + test. **This is the direct fix for the fragmentation example.**
- **G2b — transcript multi-⭐.** Owner (r2 D1): "в стенограмме нельзя выделить сразу несколько
  звёзд". Add tile-style multi-mark to the transcript (mark several lines → one joined record).
- **G2c — AI-grouping of auto-extraction  [P3, defer-able].** Opt-in AI pass grouping
  auto-extracted facts by entity/topic. Non-deterministic + egress; defer until G2a/b proven.

### G3 — Transcript cross-block selection  [P2 — parity]
Owner (r2 D1): "в стенограмме нет сквозного выделения". Extend Option A's «Выделить текст»
mode to the transcript (dual-capped join of the DISPLAYED lines — transcript already caps at
200 lines). Lower priority: the transcript already has per-line ⭐ + «Копировать выбранное».

## STATUS (updated 2026-07-03)
- **G1 layout-independent Ctrl+C/V/X/A** — ✅ `79ee2c3` (winit filter, `unstable-winit-030`).
  Owner-verified r3 (RU + EN, all fields). Old Cyrillic shims KEPT as dead-code safety net —
  **delete after more live confidence** (small cleanup left).
- **G2a tile N⭐ → one record** — ✅ `79ee2c3`. Owner-verified r3.
- **G2b transcript ⭐ multi-mark → one record** — ✅ `4212b7c`. Reworked from a checkbox
  button to tile-style ⭐-multi (owner: «не могу выбрать сразу несколько звёздочек»).
  Owner-verified r4 (5/5). Ported the tile I-1 edit-guard.
- **Memory rework — design DONE + APPROVED.** `docs/memory-architecture.md` (fable ADR).
  Owner chose **FULL pipeline M1–M4** (2026-07-03): M1 normalization-on-capture (fact
  formatting — the MUST) · M2 relevance retrieval (FTS5 BM25) · M3 coherence (entity-grouping
  + merge) · M4 embeddings + hybrid (e5-small sidecar + cosine/RRF). **Building now, M1 first.**
  Then (owner, after M1–M4): **M6 graph memory** (entity+relation knowledge-graph over facts —
  design-first later) → **Slint 1.16→1.17 migration** (Tooltip / DragArea / cross-axis-align +
  the richer MCP; verify the G1 `unstable-winit-030` filter + byte-offset props + ContextMenuArea
  still hold).

#### M1 progress (normalization)
- **Schema (0005)** — ✅ `41096c3`. `memory_items`/`memory_candidates` +
  `source_text`/`entity`/`norm_status` (dormant). `LATEST_VERSION`→5.
- **M1-a — pure core** — ✅ `81bea74`. `overlay-backend/src/memory/normalize.rs`:
  `heuristic_clean` (ws + ≥4-letter stutter-dedup, never drops numbers/short words) +
  `is_grounded(source,rewrite)` (anti-hallucination gate). Pub lib API (not dead-code),
  7 tests. **Adversarial review caught a CRITICAL false-accept** — identifiers were matched
  by substring so a truncated IP (`10.0.0.11` vs `10.0.0.116`) passed → fixed to WHOLE-TOKEN
  equality against a tokenized source; + acronym-swap (VPN→DNS now gated), incomplete-negation
  (added `не…`/`ни…` prefix + `нельзя`/`без`), and vacuous-`0≤0` holes, all fixed + pinned.
  No behaviour change yet (nothing calls it).
- **M1-b-1 — store CRUD for 0005 columns** — ✅ `3ea23fe`. `MemoryItem`/`NewMemoryItem`
  +`source_text`/`entity`/`norm_status`; `insert_memory_item` writes them, `list_memory_items`
  reads them, new `update_memory_item_normalized(id, text, entity, norm_status)` for the async
  completion. INVISIBLE — every caller passes `None`/`None`/`"none"`, behaviour unchanged.
  Round-trip test (insert → read → normalized-update, source_text preserved). 31/31 backend
  tests + both crates clippy-clean. No owner retest needed (nothing user-visible changed).
- **M1-b-2 — async LLM normalization on capture** — ✅ `1a84bf4`. Star a single raw-STT
  transcript line → instant save as heuristic-clean (`norm_status='pending'`, raw in
  `source_text`); a worker thread (std::thread + current-thread tokio rt) runs `normalize_fact`
  (heuristic → `ai::complete` no-think JSON → `parse_first_fact` → **gate with
  `is_grounded(raw, rewrite)`**) then `update_memory_item_normalized` (→ `'llm'` on success,
  else `'heuristic'`). ⭐ never blocks on the AI. Routing: ONLY the single-⭐ transcript line
  normalizes; tile blocks / multi-⭐ joins / selection spans / typed facts stay verbatim.
  Adversarial review caught a SQLite rowid-REUSE bug (delete/clear mid-window could land the
  UPDATE on a reused-id row) → guarded the UPDATE with `norm_status='pending' AND source_text=?`;
  test added. Gate 0/0. **Owner retest: `docs/retest-tile-lock-normalization.html`.**
  Follow-ups: «Извлечь» batched normalization; broaden to selection spans if the owner wants;
  sweep stale `'pending'` rows on boot (cosmetic — text is already the heuristic clean).
- **M2–M4 (relevance / coherence / embeddings)** — ⏳ NEXT, after the owner signs off M1.

#### Side feature — 🔒 bar lock chip (listening mode)
- ✅ `cc5a7e6` — owner ТЗ (2026-07-03/04): «отключать всплывание тайлов … кнопка с замочком …
  векторная SVG … потрясывается, чтобы понять почему тайлов нет» (для записи игр — тайл
  перекрывает игру). REUSES `config.suppress_tiles` (existing field + live enforcement in
  `slint_session::maybe_spawn_auto_tile` + a Settings checkbox — no new flag/enforcement); adds a
  bar `LockChip` (SVG padlock `assets/icons/lock.svg`, TREMBLES via `Timer`+`transform-rotation`
  ±7° while active) + `on_suppress_tiles_toggle_clicked` wiring + startup seed (persisted-locked
  restart shows the shaking lock). Adversarial review clean (only note: bar↔Settings widgets
  re-sync on open, not live — accepted, mirrors `auto_tiles`). Owner retest in the same HTML.
- **G3 transcript cross-block select-mode** — ⏳ deferred (P2 parity; low value now).

## Backlog
- **Slint 1.17 + Slint-MCP for verification.** 1.17 (2026-06-24) adds DragArea/DropArea,
  Tooltip (markdown), RadioGroup, `cross-axis-alignment` — NOTHING for text-selection /
  KeyEvent / winit, so no forced upgrade for our features. BUT the "Getting Good Vibes from
  Slint" post (2026-07-03, `slint.dev/blog/slint-and-AI-MCP.html`) documents the Slint MCP
  (the `mcp` feature we already build with `SLINT_MCP_PORT`) with richer capabilities than
  our 1.16 exposes: real element-tree inspection, click/drag/keyboard dispatch, inline
  screenshots, `SLINT_BACKEND=headless` for CI, hot-reload AI loops. **Investigate:** does
  1.17's MCP element-tree populate (ours returned 0 children)? If so it could REPLACE the
  painful computer-use smoke, and headless-MCP could add automated UI checks to the gate.
- Tooltip widget (1.17) — nice-to-have for the UI later.

## Method (every phase)
ci.ps1 0/0 ×3 crates + independent adversarial review + **owner HTML retest** (fillable
`docs/retest-*.html`, golden rule) before it's "done". No computer-use flailing — owner tests.
No push / release without explicit «релизь»; accumulate. Live-smoke is the owner's retest.

## Anchors
- Shortcut shim (RU-only, to replace): `ui/text_ask.slint:80-85`, `ui/palette.slint:87-95`,
  `ui/controls.slint` SelectableText key-pressed.
- Editable fields lacking the shim: tile.slint capture-editor LineEdit + follow-up LineEdit,
  settings_panel.slint (~«Свой факт»), wizard.slint, archive.slint.
- Memory join: `tile_copy.rs on_save_marked` / `insert_approved_note`; transcript
  `aux_windows.rs wire_transcript_actions` + `transcript.slint` per-line ⭐.
