# Codex handoff — resume this session & push to git

Read `AGENTS.md` first (build/gate commands + hard rules). This file is the
live state and the exact push runbook. Written 2026-07-10.

---

## 1. Where you are right now

- **Checkout is SHARED** with a Claude Code session — the same working dir is
  driven by two agents. Always `git status` / `git log` before acting; never
  assume the branch or index is what you left.
- **Current branch:** `codex/quality-2026-07-10`
- **HEAD:** `b18e9d1` (`docs(quality): record final build verification`)
- **Relationship to origin/master (`bdd992a`):** this branch is a **linear**
  extension — **15 commits ahead, 0 behind**, no divergence. It fast-forwards
  cleanly. It is **NOT pushed** to origin yet.
- **Version:** `0.33.0` (unchanged — quality work accumulates; no bump until a
  release is decided). v0.33.0 is already the published GitHub release.

## 2. What is done (two workstreams, both already committed)

**A. Hermes v0.34.0 — on origin/master (`bdd992a`), NOT released.**
Waiting for the owner's explicit "релизь". These are the commits:
- `7e20e21` — "Проверить" uses the agentic 180s budget (10s false-negative fix)
- `6f06332` — one-click "Взять ключ из локального Hermes" + usage-scenario card
- `618a383` — ASCII glyphs (skia tofu fix) on the Hermes tab
- Acceptance: `docs/retest-hermes-v0.33.0.html`

**B. Quality Q1–Q5 + deps — the 15 commits on THIS branch.**
Code-complete; owner audio/visual acceptance still pending (see §5).
- `a13f671` Q1 TTS number normalization (`overlay-backend/src/tts_normalize.rs`, 16 tests)
- `c7c9b95` Q2.1 diarization segment post-merge (`suflyor-tts/src/diar.rs`)
- `36a0ae3` Q2.3 auto speaker-count sweep
- `f5eb3fb` Q3 icons unified/disambiguated + `slint-experiment/tests/icon_guard.rs`
- `5bd38bf` Q4 shared spacing metrics + `docs/design-system.md`
- `26e81ad` Q5.1 WSOLA crossfade widened
- `9b55ad5` Slint 1.16.1 → **1.17.1**
- `af22431` global-hotkey 0.8 · `610347a` rfd 0.17 · `1448bb4` reqwest 0.13 ·
  `c86644b` rodio+timestretch · `c64a0bc` sha2 0.11 · `045342d` deps quick wins
- `712d9e7` + `b18e9d1` execution evidence + UI gallery
- Plan + evidence: `docs/goal-quality-2026-07-10.md`. Acceptance checklists:
  `docs/retest-quality-q{1..5}-*.html`.

## 3. Gate status of this branch's HEAD

**Verified GREEN at `b18e9d1` on 2026-07-10** (Claude ran `scripts/ci.ps1`):
all 9 layers pass — fmt/clippy/test for backend + slint + tts, incl. the new
`icon_guard` and the existing `i18n_guard` (they run inside the slint test
layer). Re-run it yourself right before pushing (the tree is shared, so it may
have moved). Command:
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
Push only when it prints "All gating layers green."

## 4. How to push (exact runbook)

```sh
# 0. one-time per clone: route hooks (agent-agnostic gate)
git config core.hooksPath .githooks

# 1. confirm you are where you think you are
git status
git log --oneline -3            # expect HEAD = b18e9d1 (or later)

# 2. full gate MUST be green (see §3)
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1

# 3. push the branch (NOT master). pre-push hook re-runs clippy+tests.
git push -u origin codex/quality-2026-07-10

# 4. open a PR against master (do NOT merge it yourself)
gh pr create --base master --head codex/quality-2026-07-10 \
  --title "Quality Q1-Q5 + deps backlog" \
  --body "See docs/goal-quality-2026-07-10.md. Owner acceptance pending (retest-quality-q*.html). Do not merge until owner signs off."
```

## 5. What must NOT happen (guardrails)

- **Do NOT `gh release`, do NOT push tags, do NOT bump the version.** Releases
  are owner-triggered only, after explicit "релизь".
- **Do NOT merge to master.** Push the branch + PR; the owner reviews and
  merges. (Deps jumps like Slint 1.17 / reqwest 0.13 and every UI change need
  human sign-off — that is the whole point of the retest HTMLs.)
- **Do NOT commit `.claude/**`.** `git status` shows
  `.claude/hooks/git-gate.ps1` and `.claude/settings.json` as modified — those
  are the owner's intentionally-local disk-hygiene settings. Leave them.
- **Do NOT commit the untracked bulk artifacts:** `docs/reference-shots/*`
  (Codex's icon-repair iteration screenshots, ~5 MB throwaway), `.codex/`,
  `.agents/`. If you want to keep a FEW final reference shots, add them
  deliberately by path — never `git add -A` / `git add .` here.
- Use `git add <explicit paths>`, never a blanket add — the shared checkout is
  full of untracked scratch.

## 6. What remains after the push (owner's court)

1. Owner reviews the PR; CI + security workflows must go green on the branch.
2. Owner does audio/visual acceptance via the 5 `docs/retest-quality-q*.html`
   (Q1 = listen to number phrases; Q2 = mark 20 diarized lines; Q3 = eyeball
   the icon gallery; Q4 = eyeball the UI gallery; Q5 = listen to 2x/3x player).
3. On sign-off: merge to master, then bump + release **only on explicit
   "релизь"** — likely one combined release carrying Hermes v0.34.0 (§2.A) +
   the quality/deps work (§2.B).

## 7. Backlog not yet started (for a future session)

- `docs/AGENT_TASKS.md` — T1–T4 were the delegable quality tasks; T1–T3 are now
  effectively done by workstream B, T4 (global-hotkey) too. Re-scope before use.
- Hermes agent-memory ТЗ: assessed as mostly covered by the live bridge; the
  only real delta (`/suflyor-sync` in the plugin) is deferred — see the
  assessment in the chat log / `docs/goal-hermes-integration-2026-07-09.md`.
- Memory scorer → FTS5 migration (`docs/memory-architecture.md`) when facts
  grow past ~150–200.
