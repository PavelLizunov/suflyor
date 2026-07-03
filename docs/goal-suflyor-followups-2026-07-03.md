# Goal — suflyor follow-ups (2026-07-03): layout-independent shortcuts + связная память + transcript parity

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
