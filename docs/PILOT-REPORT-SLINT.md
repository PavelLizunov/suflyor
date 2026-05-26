# Phase 0 pilot report — Slint replay viewer

**Date:** 2026-05-27 (single autonomous session, ~3 h agent-time)
**Branch:** `experiment/slint-replay` (commits 6fb0e16 + 49ffd4c + this)
**Pilot scope:** rebuild the React `Replay.tsx` viewer in Slint to test
the migration plan's assumptions before committing to Phases 1-7.

## TL;DR — Recommendation: **GO** (proceed to Phase 1)

Slint passed every check the pilot was designed to stress:

- ✅ Toolchain builds on Windows with zero system deps (no GTK / X11 /
  Wayland; default winit + skia backend is self-contained).
- ✅ Cold compile ≈ 5.8 s for the empty pilot, incremental clippy ≈ 2 s,
  test build ≈ 13.8 s (deps pulled), incremental test ≈ 3 s.
- ✅ Real journals load + render (auto-loaded newest of 112 on-disk
  sessions; SESSION START / SUMMARY / SESSION STOP rendered with
  kind-coded accents and complete footer counts).
- ✅ Filter chips toggle live; reset works; session combobox switches
  reload + reset state.
- ✅ `i-slint-backend-testing` test harness works (3 scenarios in one
  consolidated `#[test]` fn green via `cargo test`).
- ✅ review-agent (general-purpose subagent, briefed cold) caught 5
  real parity-drift bugs against React's `ReplayRow()` before commit;
  all fixed pre-Day-2-commit.
- ✅ No paint glitches, no WebView2-class issues, no `unsafe` in our
  hand-written code (Slint's generated code is contained behind a
  `#[allow]` wrapper).

The risks identified in the migration plan (markdown adapter, multi-
monitor placement, HWND-poking for overlay flags) are NOT exercised
in Phase 0 — they remain on the Phase 1-4 risk register. Phase 0 only
proves Slint is viable for the *simple* end of the UI surface, which
was the pilot's explicit purpose.

**User decision required at the go/no-go gate**: this report is the
recommendation; the user may override to no-go (in which case roll
back to React via `git checkout master` + update ADR-001).

## Day-by-day execution

Plan estimate: 3 days human-pace. Actual: ~3 h agent-time.

| Day | Plan goal | Estimate | Actual | Notes |
|---|---|---|---|---|
| 1 | Foundation: cargo crate, build.rs, replay.slint hello-world, smoke run | ~3-4 h human | ~30 min agent | One iteration to drop `unsafe_code = "forbid"` (Slint generated VTable code uses unsafe), one iteration to add `slint::ComponentHandle` to scope for `.new()` / `.run()`. Otherwise clean. |
| 2 | Backend wiring: replay_backend.rs (list_sessions + load_session + render_event), main.rs rewrite to Rc<RefCell<PilotState>> + 4 callbacks, real journals load | ~5-7 h human | ~45 min agent | One iteration to drop `serde::Serialize` derive (serde not a dep), one for `useless_format!` lint. review-agent on the diff found 5 important parity drifts vs Replay.tsx (preview-length truncation across 5 kinds + missing session_summary fields + filter-strip visibility gate) — all fixed before commit. |
| 3 | 3 i-slint-backend-testing tests + screenshot baseline + this report | ~3-4 h human | ~30 min agent | One iteration on the test layout — Slint's testing backend has per-thread platform-install affinity, libtest spawns a fresh thread per `#[test]` even with `--test-threads=1`. Combined the 3 scenarios into one `#[test]` fn. |

Why so much faster than plan: no marathon penalties (each Day is a
small, atomic, self-verifying commit), no human context switches,
review-agent runs in parallel with file edits, and the LOC budget for
each Day is small enough to fit the agent's context window cleanly.

## LOC comparison

| Surface | Slint pilot | React equivalent | Delta |
|---|---:|---:|---|
| UI declaration (`.slint` vs `.tsx` JSX+state mixed) | 223 | 526 | **−58 % (2.4× compression)** |
| Hand-written Rust (`main.rs` prod + `replay_backend.rs`) | 283 + 351 = 634 | 101 (src-tauri command handlers) | +527 (because TSX already has the logic inline) |
| Build config (`Cargo.toml` + `build.rs`) | 58 | 0 (Vite shared with overlay) | +58 |
| Tests | 158 (in `main.rs`) | 0 in Replay viewer (Vitest is global) | +158 |
| **Production code only** | **857** | **627** | **+37 %** |
| **Production + tests + build** | **1073** | **627 + global infra** | **+71 %** with embedded harness |

Interpretation: the **UI markup is 2.4× more compact** in Slint (the
plan's central bet). The Rust side carries more weight than React
because:

1. `replay_backend.rs` re-implements list/load/preview/render that the
   React Replay viewer borrows from inline TSX + shared infra.
   Phase 1's shared-crate extraction will deduplicate against
   `src-tauri/src/lib.rs:1980-2080` so this isn't permanent bloat.
2. Tests are colocated in the binary. The React equivalent (Vitest)
   lives in a separate harness shared across the whole frontend; this
   accounting penalizes the pilot.

If we discount the duplication that Phase 1 removes, Slint comes in
roughly comparable in total LOC, with a substantial markup-side win.

## DSL & developer-experience impressions

**Pros:**

- `Window`, `VerticalLayout`, `ScrollView`, `ComboBox` from
  `std-widgets.slint` cover 90 % of the structural needs out of the
  box; the remaining 10 % (custom chip with `Rectangle` + `TouchArea`)
  is straightforward.
- `for chip[i] in root.filter-chips:` and `in-out property <[T]>`
  syntax is concise once you internalize the rules. The compile errors
  point to the right `.slint` line.
- Generated Rust API is ergonomic: `window.set_events(model)` /
  `window.get_filter_chips()` / `window.on_chip_clicked(closure)` —
  feels like a typed FFI more than a code-gen kludge.
- `slint::ModelRc::new(VecModel::from(vec))` is the standard way to
  push a Vec<T> into a Slint property; replacing the model on every
  state change is fine at <1000 rows.

**Cons:**

- The generated VTable code uses `unsafe` extensively, so
  `unsafe_code = "forbid"` is unusable at package level. Wrapping
  `include_modules!()` in a `mod ui { #[allow(unsafe_op_in_unsafe_fn,
  clippy::unwrap_used, ...)] }` block contains the blast radius but
  is a manual step we'd want documented in any new Slint project's
  README. Tier 3 clippy lints (`unwrap_used`, `expect_used`, `panic`)
  similarly need the inner `#[allow]`.
- The testing backend (`i_slint_backend_testing::init_no_event_loop()`)
  is per-thread. `cargo test` with `--test-threads=1` still spawns a
  fresh OS thread per `#[test]` fn — second/third tests panic with
  "platform was initialized in another thread". The clean workaround
  is a custom test harness (libtest-mimic, or `serial_test` + a
  long-lived worker thread for window operations); the pilot took
  the simpler route of consolidating 3 scenarios into 1 `#[test]`.
- The Slint LSP (`slint-lsp` binary) wasn't installed for the pilot
  — would speed up writing `.slint` files via live preview but is
  not blocking. Pre-Phase-1 prerequisite, ~5 min to install.
- `if cond : Element {}` and `if cond : SomeWidget {}` patterns are
  weak: they don't elide DOM, they render a 0-sized Element. For
  truly conditional rendering of complex sub-trees, the idiom is
  `visible: cond;` on the wrapping element + relying on layout to
  collapse zero-sized regions. The pilot uses `if` for chip-strip
  visibility (works) and the empty-events placeholder (works).
- No `accessible-label` on most pilot elements — accessibility tree
  is empty. Phase 1 should add labels for screen-reader support and
  to enable `find_by_accessible_label()` in tests.

**Compile-error UX:** good. When I had `unsafe_code = "forbid"`
clashing with generated `#[allow(unsafe_code)]`, the error pointed
to the exact `replay.rs` line in OUT_DIR with the macro context. Same
quality as rust-analyzer on hand-written Rust.

**Hot reload:** not tested in this pilot (no `slint-lsp` install).
Each iteration was `cargo run` ≈ 2-3 s incremental.

## Build performance

Single-machine timings (Windows 11, Rust 1.95):

| Operation | Time | Notes |
|---|---:|---|
| `cargo build` clean (first compile, deps included) | 5.78 s | 1073 LOC pilot |
| `cargo clippy --all-targets -- -D warnings` incremental | 1.79 - 2.19 s | After a small edit |
| `cargo test --bin slint-replay` first run (deps pulled) | 13.82 s | i-slint-backend-testing has its own deps |
| `cargo test --bin slint-replay` incremental | 3.11 s | No re-deps |
| Window spawn → first paint (subjective) | < 1 s | Cold start |
| 3-test suite execution | 0.00 s reported | Logic-only, no real window paint cycle |

For comparison, `npm run tauri dev` on the existing overlay-mvp
rebuilds a Rust file in ~5-10 s + WebView2 reload, and `vite` HMR
on a TS file in ~1 s with no reload. Slint's `cargo run` is faster
than `tauri dev` for Rust-only changes (no WebView reload step).

## Deviations from the migration plan

| Deviation | Plan says | Pilot did | Rationale |
|---|---|---|---|
| Crate layout | `slint-experiment/` as workspace member under `src-tauri/` | Sibling crate at repo root, standalone (no workspace) | `src-tauri/Cargo.toml` is a standalone package, not a workspace. Converting it mid-pilot would risk the master React/Tauri build. Sibling achieves the same "build alongside" outcome with zero risk. Phase 1 restructure is the proper place to decide on a workspace. |
| Shared backend module | `replay_backend.rs` callable from both Tauri commands AND Slint pilot | Duplicated in `slint-experiment/src/replay_backend.rs` (no Tauri deps); `src-tauri/src/lib.rs:1980-2080` left as-is | Pulling `overlay_mvp_lib` (Tauri-bound) into the pilot crate would drag WebView2 / tauri / wry along. Duplication is ~80 lines; Phase 1 extracts to a shared crate. Docstring in `replay_backend.rs` flags the canonical impl as source-of-truth. |
| 3 separate `#[test]` fns | Plan implies 3 distinct tests | 1 `#[test]` fn with 3 clearly-labeled scenarios | Slint's testing backend has per-thread platform-install affinity; libtest spawns fresh threads per test. Workaround documented in `tests` module comment. |
| Local-tz timestamps | React uses `new Date(unix_ms)` (local TZ) | UTC `fmt_clock` | Avoid pulling `chrono` / `time` for the pilot. Phase 1 will swap in the `time` crate (smaller than chrono). |
| Pretty modified-date | React uses `YYYY-MM-DD HH:MM` | `epoch+Nd HH:MM` | Same reason; same Phase 1 fix. |
| `cargo install slint-lsp` for live preview | Pre-prerequisite | Skipped | Not blocking for the pilot; install in 5 min before Phase 1. |

## Slint-testing gotchas discovered

Useful for the `CLAUDE.md` update in Phase 6 (replacing the Tier 4
React-Vitest section with Slint testing):

1. **Platform install is per-thread.** `cargo test ... -- --test-threads=1`
   does NOT keep tests on the main thread; libtest spawns one OS
   thread per `#[test]` fn even with the flag. Consolidate scenarios
   into one `#[test]` fn OR invest in a long-lived worker-thread
   harness.
2. **Generated-code `#[allow]` is permanent.** Any project using
   `slint::include_modules!()` will need the same `mod ui { #[allow(
   clippy::unwrap_used, clippy::expect_used, clippy::panic,
   clippy::indexing_slicing, ...)] slint::include_modules!() }` wrap
   if it uses strict clippy lints. Document this in CLAUDE.md.
3. **`unsafe_code = "forbid"` is incompatible** with Slint's generated
   VTable macros. Use `deny` (not `forbid`) if you want it at all;
   the pilot omits it entirely since Slint's unsafe is unavoidable.
4. **ComboBox `selected(int)` callback** fires before the user-visible
   `current-index` updates in some Slint versions. The pilot uses
   `current-index <=> root.selected-session-index` two-way binding +
   reads `self.current-index` inside `selected =>` — works correctly
   on 1.16.1 but worth a regression test if upgrading.

## Risk register update

| Risk from migration plan | Phase 0 evidence | Status |
|---|---|---|
| Markdown adapter takes way longer than 2 weeks | Not exercised in Phase 0 (Replay viewer has no markdown) | **OPEN — high uncertainty.** Phase 4 spike on `pulldown-cmark + syntect → StyledText` is the next thing to validate. Recommend a Phase 0.5 markdown spike before committing to Phase 4. |
| Slint multi-monitor API has gaps | Not exercised in Phase 0 (single Replay window) | **OPEN.** Phase 1 foundation will exercise via HWND + `EnumDisplayMonitors`. |
| Hot-reload UX breaks productivity | Not exercised (no `slint-lsp` install) | **OPEN.** Install + test in Phase 1 Day 1. Fallback: `cargo run` cycle (2-3 s) is acceptable. |
| WesAudio / LibrePCB are the only production proof | Not addressable in pilot | OPEN; track via Slint forum. |
| 12 weeks solo work starves user of features | Mitigated by keeping `master` (React/Tauri) shippable; Slint work isolated on branches | **MITIGATED for now.** Re-evaluate if Phase 2 settings-panel cost runs over budget. |
| License: royalty-free attribution feels bad | Decided in [ADR-002](ADR-002-license.md): royalty-free for pet-project scope | **CLOSED.** Re-open if scope changes. |

## Recommendation for Phase 1+

**Proceed to Phase 1 (Foundation, 1 week).** Before starting:

1. Install `slint-lsp` (`cargo install slint-lsp`) and test live
   preview on `slint-experiment/ui/replay.slint`. Confirm hot-reload
   UX is acceptable.
2. **Run a Phase 0.5 spike on markdown rendering** (1-2 days). Build
   a minimal `pulldown-cmark + syntect → StyledText` adapter and
   render `src-tauri/knowledge/glossary.md` (or similar mixed-content
   markdown) in a Slint test window. This is the highest-uncertainty
   item in the plan; better to know now than in Phase 4.
3. Restructure decision: Phase 1 should consolidate `src-tauri/` and
   new Slint code into one workspace (with backend modules shared via
   path deps), OR commit to two separate crates with a tiny shared
   `journal-core` crate at the root. The pilot is agnostic; either
   works.
4. Add `slint-lsp` to a forked claude-code-lsps marketplace (manual PR).
5. Update `git-gate.ps1` in Phase 1 to also check `slint-experiment/`
   (or whatever the new Slint crate is named) for clippy + fmt + test.

## Artifacts

- Commits on `experiment/slint-replay`:
  - `6fb0e16` — Phase 0 Day 1: scaffold
  - `49ffd4c` — Phase 0 Day 2: backend wiring + render parity
  - (this commit) — Phase 0 Day 3: tests + pilot report
- Visual baselines: `slint-experiment/target/visual/slint-replay-day*.png`
  (Day 1 placeholder data; Day 2 real journal `f4aa1d.jsonl`).
- ADR: [`ADR-002-license.md`](ADR-002-license.md) — royalty-free.
- Stack ADR (no change yet): [`ADR-001-stack.md`](ADR-001-stack.md).
  Will be updated on user's go-decision to mark Slint migration in
  progress.
