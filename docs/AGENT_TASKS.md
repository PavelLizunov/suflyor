# Agent task queue

Self-contained tasks for delegated agents (Codex etc.). Rules of engagement:
AGENTS.md (read it first). One task = one `codex/<name>` branch = one session.
Definition of done for EVERY task: `scripts/ci.ps1` fully green + the task's
own acceptance bullets. Do NOT release, do NOT push tags, do NOT merge to
master unless the task says so.

Status legend: [ ] open · [~] claimed (write your agent/branch) · [x] done.

---

## [ ] T1 — TTS number normalization (Q1 of docs/goal-quality-2026-07-10.md)

**Branch:** `codex/tts-number-normalize`
**Problem:** `overlay-backend/src/tts.rs` cleans markdown before speech but
has NO number normalization — the neural Piper voices read "123", "45%",
"14:30", "v0.33.0" as raw digits. The SAPI fallback handles numbers itself
and must NOT be affected.
**Do:**
1. New module `overlay-backend/src/tts_normalize.rs`:
   `pub fn normalize_for_speech(text: &str) -> String` — Russian words for:
   integers (nominative case is enough; up to trillions; negatives), percents
   ("45%" → "сорок пять процентов"), clock times ("14:30"), decimal comma/dot
   ("3.5" → "три и пять"), simple ranges ("3-5" → "три-пять"), version-like
   tokens ("v0.33.0" → spell the parts). Leave everything else untouched.
2. Register in `lib.rs`; call it ONLY in the neural branch of
   `Tts::speak()` in `tts.rs` (after the markdown cleanup, before SPEAK).
3. ~15 unit tests in the module (happy paths + "no numbers → identity" +
   mixed RU/EN sentence).
**Accept:** gate green; tests cover each rule; `speak()` SAPI path unchanged
(diff shows the call only under the neural condition).

## [ ] T2 — Diarization segment post-merge (Q2.1)

**Branch:** `codex/diar-postmerge`
**Problem:** `suflyor-tts/src/diar.rs` returns raw sherpa segments
(threshold 0.5, min_duration_on 0.3) — short fragments and A-B-A flicker on
VoIP audio smear speaker attribution.
**Do:** after clustering, post-process the segment list:
1. Merge adjacent segments of the SAME speaker when the gap < 0.6 s.
2. Fragments shorter than 0.4 s: attach to the previous segment if the gap
   < 0.3 s, else drop them.
3. Keep the function pure (`fn postprocess(segs: Vec<Seg>) -> Vec<Seg>`) with
   unit tests on synthetic segment lists (merge, attach, drop, no-op cases).
**Accept:** gate green; tests for all four cases; output stays sorted and
non-overlapping.

## [ ] T3 — Icon guard test + star.svg regridding (Q3.1 + Q3.4)

**Branch:** `codex/icon-guard`
**Problem:** 42 of 43 icons in `slint-experiment/assets/icons/` follow the
convention `viewBox="0 0 16 16"` + `stroke-width="1.6"`; `star.svg` is the
outlier (24x24, stroke 2) and looks alien in the UI.
**Do:**
1. Redraw `star.svg` on the 16x16 grid, stroke 1.6, round caps/joins,
   5-point star centered, visually matching the weight of `check.svg`/`x.svg`.
2. New gate test `slint-experiment/tests/icon_guard.rs` (mirror the style of
   `tests/i18n_guard.rs`): read every `assets/icons/*.svg`, fail unless the
   file contains `viewBox="0 0 16 16"` and every `stroke-width` equals `1.6`
   (icons with fill-only paths and no stroke-width are allowed).
3. Add a short `assets/icons/README.md` stating the convention.
**Accept:** gate green (the new test runs in `scripts/ci.ps1` automatically —
it picks up crate tests); star.svg passes its own guard.

## [ ] T4 — deps: global-hotkey 0.6 → 0.8 (Phase 2 of docs/goal-deps-updates-2026-07-09.md)

**Branch:** `codex/dep-global-hotkey`
**Do:** follow the charter's Phase 2 exactly (bump, fix API drift in
`src/bin/overlay_host/hotkeys.rs`, keep every hotkey registration logged at
boot). NO other dep changes in the same branch.
**Accept:** gate green; `cargo tree -i global-hotkey` shows 0.8.x; the boot
log still prints one "hotkey registered" line per table entry in
CLAUDE.md/AGENTS.md hotkey list.

---

Done tasks move to the bottom with a one-line result + commit hash.
