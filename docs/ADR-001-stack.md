# ADR-001 — Stack decision: React/Tauri vs Iced/Slint/Dioxus

**Status:** ⚠️ **SUPERSEDED (2026-05-28).** This ADR decided to KEEP React/Tauri
(Variant D). That decision was reversed two days later: the WebView2 paint
issues this ADR named as the re-evaluation trigger materialised, and the whole
UI was rewritten in **Rust + Slint** (Phase 7 cut — see
`docs/PHASE-7-CUT-PLAN.md` / `docs/MIGRATION-PLAN-SLINT.md`). The current stack
is a pure-Rust two-crate layout (`overlay-backend` + `slint-experiment`) with
NO React, Tauri, WebView2, npm, Vite, or TypeScript. The analysis below is kept
verbatim as the historical record of why we first stayed and then left.

**Original status (2026-05-27):** Decided — keep React/Tauri (Variant D from the
suflyor GUI strictness spec) with the Tier 1-4 harness adopted in commits e33d69e
through this one. Re-evaluate in Q4 2026 if persistent WebView2 paint issues remain.

**Date:** 2026-05-27

## Context

After the 2026-05-26 marathon disaster (29 features in 6 h → 64/68
releases manually deleted by the user), we adopted the vpnctl 6-layer
testing methodology. The suflyor GUI strictness spec (kept in memory
`[[suflyor-gui-strictness-spec]]`) recommends one of four stack options
to reach "Rust-level rigor":

- **A. Iced** (pure Rust, Elm-architecture)
- **B. Slint** (declarative DSL + Rust)
- **C. Tauri + Dioxus** (React-like in Rust)
- **D. Keep React, tighten to the limit**

Variants A-C require a UI rewrite. Variant D adds tight type harness
on top of the existing React surface (~3500 lines of TSX, 12 settings
panels, 11+ tile chrome elements, F4 KB palette, Replay viewer).

## Decision

**Take Variant D.** Specifically:

1. Adopt the strict TS / ESLint / clippy baseline (Tiers 2 + 3 of the
   methodology, see CLAUDE.md).
2. Adopt the test pyramid (Tier 4): cargo test --lib, vitest, copy
   contract, docker scaffolds.
3. Adopt the blocking git-gate hook (Tier 1, already shipped commit
   6b8fb6d).
4. Defer Iced/Slint/Dioxus rewrite until the harness is shown to be
   insufficient — i.e. another paint/focus/multi-monitor regression
   that the new layers don't catch.

## Trade-offs

### Why NOT Iced/Slint/Dioxus today

- **~3500 lines of TSX** to port. Three months minimum for a careful
  rewrite. The user is solo and can't afford that window without
  also losing all v0.1.1 features in the meantime.
- **Markdown rendering** — Iced/Slint lack a maintained Markdown
  renderer with syntax highlighting + GFM tables comparable to
  react-markdown + remark-gfm + rehype-highlight. Building from
  scratch on `pulldown-cmark + syntect` is real work.
- **No way to validate the rewrite catches MORE bugs** than the
  Variant D harness. The marathon bugs were paint-timing, focus
  races, monitor geometry — these may persist in any GUI stack on
  Windows DWM. Spending 3 months on a rewrite without proof of
  improvement is unjustifiable for a pet project.

### What we accept by staying on React/Tauri

- `as`, `!`, `any` remain bypassable (the strict ESLint rules can be
  disabled with `// eslint-disable-next-line` — the strict TS
  `useUnknownInCatchVariables` doesn't apply to `.catch()`
  callbacks even with the rule, only to `try/catch`).
- WebView2 transparency + always_on_top remain paint-flaky on
  Windows DWM. Mitigated by the opaque-ish `rgba(20,22,30,0.92)`
  tile-root background pinned in `copy_contract.rs`.
- Imperative React effects with closure-captured state remain the
  bug surface for stale-data issues; mitigated by
  `react-hooks/exhaustive-deps: error` (with 2 documented
  `eslint-disable` exceptions tied to Tauri listener semantics).

### What Variant D gives us

- TypeScript `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
  `noPropertyAccessFromIndexSignature` — prevents the index-signature
  bugs we kept hitting in `Replay.tsx`.
- ESLint `consistent-type-assertions: never`, `no-explicit-any`,
  `no-non-null-assertion` — closes the `x as T`, `any`, `!` escape
  hatches at the language level.
- ESLint `no-restricted-syntax` for `Date.now()` / `Math.random()`
  with the `src/clock.ts` injection module — enables deterministic
  tests of time-dependent code.
- ESLint `react-hooks/exhaustive-deps: error` — catches stale
  closures, the bug class that produced the v0.0.85 reload+translate
  regression and the v0.1.2 palette-restore race.
- Rust `deny(unwrap_used, expect_used, panic, missing_docs)` —
  forces failure handling in production paths.
- `git-gate.ps1` — BLOCKS `git commit / push` when any of the above
  fail. The single piece of infra that turns the methodology from
  discipline into enforcement.

## Trigger to revisit

Re-open this ADR if **either** of these happens:

1. A user-facing regression escapes all the Tier 1-4 layers AND would
   have been caught by a Rust-native UI (compiler-enforced impossible
   states, no closure capture). Concrete example: a stale-closure bug
   in a Tauri listener that the `[[ref]]` pattern can't fix because
   the upstream Tauri API requires the closure to be `'static`.
2. The WebView2 paint-flakiness produces ≥ 3 user-reported bugs in a
   single quarter that aren't reproducible with native rendering.

In either case, prefer **Slint** as the rewrite target: declarative DSL
+ first-class headless test API + accessibility tree — best fit for
overlay-mvp's mostly-static UI. Iced would be the second choice if
Slint's DSL turns out to be too rigid for the markdown-heavy tiles.
