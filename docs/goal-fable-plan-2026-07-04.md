# Goal — implement fable's P1–P5 (2026-07-04)

Owner 2026-07-04: «начали всё сразу делать» — implement ALL of fable's audit+plan
(`docs/fable-plan-2026-07-04.md`), not one item at a time. `/ponytail full` in force.

## Decisions (owner)
- **Egress: provider-agnostic.** Normalization uses the configured `ai_endpoint` (LOCAL or CLOUD by
  `ai_provider`), NOT forced local-only. On a cloud provider the raw STT span goes to the cloud bridge
  — same egress class as answers/summary; owner accepts. Keep the AI error tiles generic (no base_url
  leak). Resolve the ADR prep:true/false detail during P3 (tier only, not local-vs-cloud).
- **Order:** P1 → P4 → P3 → P2 → P5 (trivial win first; then the memory architecture P4 which deletes
  the D2 negation bug; then offline retry P3; then transcript order P2; then select-mode P5).
- **Method (every piece):** ci.ps1 0/0 ×3 crates + independent adversarial review + owner HTML retest.
  No push/release without «релизь». Accumulate.

## The five (see fable-plan for detail + anchors)
- **P1 — C2** transcript ⭐ scroll inset: row `padding-right` 4→~18px. transcript.slint. [mechanical]
- **P4 — A3/A2 QUOTE-SPAN** (the big one): model returns 1–3 VERBATIM contiguous quotes; Rust
  `locate_span()` matches each as a contiguous substring (casefold + ws-collapse, snapped to
  whole-token bounds) and stores the SOURCE SLICE, not the model's text. Recombination impossible by
  construction; negation local (check 1–2 tokens before the span); gate `entity`; spans ≥2 tokens or
  ≥1 identifier. Deletes `is_grounded`/`has_negation`/`source_contains`/`is_identifier` (~150 lines),
  adds `locate_span` (~50) + prompt rewrite; keeps shape caps, `parse_facts`, tile_copy pipeline.
  Cost: source word order preserved, mid-span filler stays. normalize.rs only. This is the real fix
  for D2 (negation recall-killer) + A2 (recombination).
- **P3 — A1 offline reliability:** `normalize_fact -> Result<Option<..>>` — `Err`(transport) → leave
  `'pending'` (retryable); `Ok(None)` → `'heuristic'` (AI declined, terminal); `Ok(Some)` → `'llm'`.
  `sweep_pending()` (one thread, shared `condense_one`, filter pending from `list_memory_items`,
  sequential, ABORT on first `Err`, AtomicBool guard) on boot (~15s) + Memory-tab open — also heals
  D4 orphans. Honest badges («ждёт ИИ»; «почищено» only if text≠source_text). Ride-along D3 one-liner
  (`update_memory_item_text` also sets `norm_status='none'`). Files: normalize.rs, tile_copy.rs,
  settings_memory.rs, settings_panel.slint, sqlite_store.rs, ru.po.
- **P2 — C1 timecode order:** diagnose first (SQL on owner DB: wall_off vs audio_ms per source →
  epoch-skew H1 vs overlap H2). Either way stable-sort rows+utts by the DISPLAYED clock at the load
  site (aux_windows.rs; precompute offsets in original order). Only if H1: share one epoch across
  capture loops (audio.rs) — also fixes seeks. Fold in the D6 tail click-test (synthetic long session).
- **P5 — B select-mode:** delete the single-huge-field Option A (`select-mode`/`select-text` + 2nd
  ScrollView + `build_select_text`); keep `capture_selection` for idx≥0. Add SHIFT-CLICK ⭐ block-range
  marking (modifiers via pointer-event; reuse `marked` model + «В память (N)» bar) + «Копировать» on
  the mark bar (`join_marked_text` exists). Native scroll between clicks → no drag-scroll problem.
  Files: tile.slint, tile_copy.rs, ru.po.

## Audit ride-alongs (fold into the relevant P)
- D3 edit-clobber (→ P3), D4 orphan-pending (→ P3 sweep), entity ungated (→ P4), badge-when-unchanged
  (→ P3). D5/D6: stop trusting i16 comments (renderer is Skia/GPU) — P4/P5 avoid tall single elements
  rather than re-tuning caps; verify ListView tail with a long-session click-test (→ P2).

## STATUS (2026-07-04)
- **P1** ✅ `67eb8e5` — ⭐ scroll-inset (transcript.slint padding-right 4→18).
- **P2** ✅ `67eb8e5` — timecode order: DIAGNOSED as STT-latency (H2, not epoch skew — per-source
  wall-audio deltas ~6s), fixed by sorting rows+utts together by the displayed clock; no audio.rs change.
- **P4** ✅ `b6c67c0` — QUOTE-SPAN extraction (net −108 lines): model returns verbatim quotes,
  `locate_span` stores the contiguous source slice → fabrication/truncation/recombination impossible
  by construction. Deleted is_grounded/is_identifier/source_contains/has_negation. Adversarial review:
  core sound; 2 guards added (reject clause/sentence-boundary spans; cap ~200 chars). 7 tests.
- **P3** ✅ code-complete, gating — offline reliability: `normalize_fact -> Result<Option<..>>`
  (TRANSIENT err→`Err`→leave `'pending'`/retry; PERMANENT 4xx→`Ok(None)`→terminal `'heuristic'`;
  located→`Ok(Some)`→`'llm'`) + `sweep_pending()` (boot+15s & Память-open, re-entrancy-guarded,
  abort-on-transient-Err) + honest badges («ждёт ИИ»; «почищено» only if changed) + D3 (edit→'none').
  Independent review: HIGH found+fixed (permanent 4xx/bad-config was mis-classed as retryable → stuck
  'pending' forever + sweep starvation; now `ai::is_permanent_ai_error` routes it to terminal heuristic).
- **P5** ✅ code-complete, gating — DELETED select-mode Option A (the overflow bug) + `build_select_text`;
  added SHIFT-click ⭐ range-marking (Rust `Cell` anchor, `pointer-event.modifiers.shift`) + «Копировать»
  on the mark bar (`join_marked_text`→clipboard). Adversarial review in flight.
- Gate `budpvo7z2` running (fmt+clippy-D+test×3+i18n). NOT pushed (accumulating). ⚠ Owner test of the
  memory (P3/P4) needs the local AI UP; the AI-OFF case now defers to `'pending'` + auto-retries.
