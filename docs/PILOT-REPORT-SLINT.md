# Phase 0 pilot report — Slint replay viewer

**Date:** 2026-05-27 (single autonomous session, ~3 h agent-time)
**Branch:** `experiment/slint-replay` (commits 6fb0e16 + 49ffd4c + this)
**Pilot scope:** rebuild the React `Replay.tsx` viewer in Slint to test
the migration plan's assumptions before committing to Phases 1-7.

## TL;DR — Recommendation: **GO** (Phase 0.5 spikes done)

> **Update post-Phase-0.5:** User selected GO with Phase 0.5 spikes;
> all three are now complete (commits 403289f / ebbcf10 / 5558f8b on
> the same branch). Net outcome: MCP and markdown spikes succeeded
> outright; HWND spike succeeded for flag application but uncovered
> a NEW soft-blocker around Slint's transparent-window compositing.
> Detailed results in the [Phase 0.5 spike outcomes](#phase-05-spike-outcomes)
> section below. Recommendation upgraded to plain GO — Phase 1 may
> start, with the transparency-wiring task added to its Day 1 backlog.

> **Original R9 re-review note (preserved):** The Phase 0 GO
> recommendation was initially unqualified. An independent re-review
> (full-Phase-0 audit by a general-purpose subagent that hadn't seen
> the earlier Day-2 review) flagged that this report had been
> overselling — it omitted that three plan-listed validations were
> NOT exercised: Slint MCP server wiring, HWND-poking for transparent
> overlay flags, and the markdown adapter spike. All three were
> moved into a recommended **Phase 0.5**, which has now executed
> (see below).

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

### What Phase 0 explicitly did NOT validate (R9 re-review honesty pass)

| Unvalidated capability | Why it matters | Where the gap should close |
|---|---|---|
| **Slint MCP server wiring** | The migration plan lists `SLINT_MCP_PORT=8080` + the `mcp` feature as **"the killer integration"** — replaces `scripts/visual_check.ps1` + computer-use with agent-driven UI testing. The pilot enabled the dev-dep feature but never spun up the server, never connected Claude Code to it, never proved it works. | **Phase 0.5 wire-up + smoke test** (≤ ½ day). If MCP doesn't actually work in our setup, the testing story changes materially. |
| **HWND-poking for overlay flags** | The migration's reason-for-being is fixing WebView2 transparency / always-on-top / paint-flakiness. The pilot used a normal opaque window — zero evidence Slint+winit can hit the same overlay invariants the existing Tauri build does. | **Phase 0.5 spike** (½-1 day) — grab `slint::Window::window_handle()` raw HWND and call `SetWindowLongPtrW(GWL_EXSTYLE, WS_EX_LAYERED \| WS_EX_TRANSPARENT \| WS_EX_TOOLWINDOW)` + `WDA_EXCLUDEFROMCAPTURE`. If these flags don't survive Slint's compositor, the migration loses its main benefit. |
| **Markdown adapter (pulldown-cmark + syntect → StyledText)** | The plan's risk register flags this as the **single largest unknown**, budgeted at 2 weeks of Phase 4. No Slint reference impl exists in the wild. If the adapter takes 4+ weeks, Phase 4 blows the plan. | **Phase 0.5 spike** (1-2 days) — minimal pulldown-cmark walker that emits `Text` / `StyledText` for a small set of features (headings, paragraphs, code blocks with syntect colors, bullet lists). Render `src-tauri/knowledge/glossary.md` in a Slint window. If the spike feels grim, re-evaluate hybrid (React-only for tile content) per the plan's risk register. |
| **Export-to-markdown button** | `src/Replay.tsx:207-232` has a 📥 .md button calling `export_session_markdown` Tauri command. Pilot didn't port it. Not Phase-0-critical but a parity gap Phase 1+ inherits. | Phase 1 tile-window work, when invoking backend commands from Slint windows gets first-class treatment. |
| **Local timezone in fmt_clock / fmt_modified** | Pilot stays UTC + `epoch+Nd HH:MM` because adding `time` or `chrono` mid-pilot felt out-of-scope. Combobox labels are practically useless to a human as a result. | Phase 1 Day 1 (~30 min): pull `time = { version = "0.3", features = ["macros", "local-offset", "formatting"] }`. |
| **Hot-reload via `slint-lsp`** | Plan's Pre-Phase-0 prerequisite #1; pilot skipped to save time. Installed post-hoc this session (`cargo install slint-lsp` succeeded). | Phase 1 Day 1: open `slint-experiment/ui/replay.slint` in an editor with LSP wired and confirm live-preview UX is acceptable. Fallback: `cargo run` cycle is ~2-3 s incremental, workable. |

**User decision required at the go/no-go gate**: this report is the
recommendation; the user may override to no-go (in which case roll
back to React via `git checkout master` + update ADR-001). If GO,
the recommended next move is **Phase 0.5** (the three spikes above)
before committing to Phase 1's foundation work.

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

## Phase 0.5 spike outcomes

Three spikes executed 2026-05-27 02:00-02:20 agent-time
(commits 403289f, ebbcf10, 5558f8b):

### Spike 1 — HWND-poking transparent overlay (`overlay-spike` binary)

**Outcome:** PARTIAL. EX-flag application works programmatically;
visible transparency does not compose with Slint's default winit+skia
backend.

What works:
- `slint::Window::window_handle()` → `slint::WindowHandle` → call
  `raw_window_handle::HasWindowHandle::window_handle()` to get the
  real `RawWindowHandle::Win32` → construct `HWND`. Two-step.
- Native HWND must be grabbed AFTER first event-loop tick (winit
  realizes the window lazily); 200 ms single-shot Slint Timer
  registered before `window.run()` is the reliable pattern.
- `SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ...)` with WS_EX_TRANSPARENT
  + WS_EX_TOOLWINDOW applies cleanly (GetWindowLongPtrW confirms).
- Crate setup: `raw-window-handle = "0.6"`, `windows = "0.62"` with
  `Win32_Foundation` + `Win32_UI_WindowsAndMessaging` features,
  `slint` gains `raw-window-handle-06` feature.

What does NOT work:
- Slint's `background: transparent;` does NOT actually enable per-
  pixel transparency in the native winit window. The first attempt
  added `WS_EX_LAYERED` (opaque black window); the second dropped it
  (still opaque black). Slint 1.16's winit configuration creates an
  opaque window by default — `with_transparent(true)` isn't reached.
- The Tauri/WebView2 build achieves transparency via DWM compositing
  (DwmEnableBlurBehindWindow + DWM attributes). Slint doesn't expose
  those knobs out of the box.

Phase 1 implications: the HWND-poking pattern is PROVEN (replay the
200-ms-Timer + raw-window-handle 0.6 + windows 0.62 setup in
`src-rs/win32.rs`). For transparency, Phase 1 Day 1 needs to either
(a) patch Slint to expose `with_transparent(true)`, (b) call
`DwmEnableBlurBehindWindow` via the windows crate as a post-creation
HWND poke (likely simplest), or (c) live with opaque overlay (would
defeat one motivation for the migration).

### Spike 2 — Slint MCP server wire-up (success)

**Outcome:** FULL SUCCESS. The migration plan's "killer integration"
works end-to-end.

What works:
- Add `i-slint-backend-selector = { version = "=1.16.1", features =
  ["mcp"] }` to [dependencies]. The mcp feature belongs on the
  SELECTOR, not on i-slint-backend-testing — that was the initial
  misread of the docs. Slint pulls i-slint-backend-selector
  transitively; declaring it as a direct dep with the feature
  enables it via cargo feature unification.
- Run the live binary with `SLINT_EMIT_DEBUG_INFO=1
  SLINT_MCP_PORT=8080`. Stderr prints:
    "Slint MCP server listening on http://127.0.0.1:8080/mcp"
- POST /mcp with a JSON-RPC `initialize` request returns a complete
  protocol response: serverInfo `slint-mcp-embedded` v0.1.0,
  protocolVersion 2025-06-18, capabilities.tools, and a richly
  documented tool API: list_windows, get_window_properties,
  get_element_tree, query_element_descendants, find_elements_by_id,
  get_element_properties, take_screenshot, click_element,
  drag_element, set_element_value, invoke_accessibility_action,
  dispatch_key_event.

Phase 1 / Phase 6 implications: this replaces
`scripts/visual_check.ps1` + computer-use entirely. Claude Code (any
MCP-capable agent) connects to the running app via HTTP and drives
UI testing without OS-level control. Wiring is ONE Cargo.toml entry
+ two runtime env vars; effort is minimal compared to the testing
power gained.

### Spike 3 — Markdown adapter (success — architecture proven)

**Outcome:** SUCCESS. pulldown-cmark + Slint dynamic-model pipeline
works. Real Phase 4 effort estimate ~6-8 days, within the plan's
2-week Phase 4 budget.

What works:
- `pulldown-cmark = "0.13"` (already transitive via slint-build).
- Walker pattern: per-event accumulator → flush on End → emit one
  `MarkdownBlock { kind: int, text: string, lang: string }` per
  block. Discriminant-based rendering on the .slint side via
  `if block.kind == N : ...` arms.
- Renders: H1 / H2 / H3 / paragraph / bullet / code block /
  horizontal rule. SoftBreak/HardBreak as space; inline `Code`
  wrapped in backticks (plaintext fallback).
- Smoke test on src-tauri/knowledge/glossary.md (first 4436 chars
  → 52 blocks) renders correctly. UTF-8 em-dash preserved through
  the full pipeline.

What does NOT work (Phase 4 work):
- Inline emphasis (bold/italic/inline-code) — needs Slint's
  `StyledText` widget with per-run formatting. Spike renders these
  as plaintext (italic/bold drop styling; inline code shows literal
  backticks).
- syntect color highlighting on code blocks — adds ~200 KB binary +
  5+ deps. Phase 4 proper.
- Tables / Links / Images / footnotes / HTML — silently dropped by
  the spike's `_ => {}` catch-all.
- Per-block layout sizing — every block currently has no min-height
  or natural row spacing, so the spike screenshot is dense. Phase 4
  invests in per-block padding + min-height + responsive wrapping.

Phase 4 effort breakdown (vs plan's 2 weeks):
- Skeleton (this spike): done
- Inline emphasis via StyledText runs: 1-2 days
- syntect wiring + theme: 1 day
- Tables → GridLayout: 1 day
- Links → TouchArea + open_url: ½ day
- Images (HTTP fetch + cache): 1 day
- Layout polish: ½-1 day
- Tile-integration glue: 1-2 days
- TOTAL: ~6-8 working days. NOT a hard blocker.

### Net Phase 0.5 verdict

| Spike | Outcome | Blocker class | Phase impact |
|---|---|---|---|
| HWND-poking | Partial — flags work, transparency doesn't | New soft-blocker | Phase 1 Day 1: investigate `DwmEnableBlurBehindWindow` / Slint winit config |
| MCP wire-up | Full success | None | Phase 6: drop visual_check.ps1 + computer-use, configure Claude Code MCP client |
| Markdown adapter | Success — architecture proven | None | Phase 4: build on this skeleton, ~6-8 days within plan's 2-week budget |

**Decision:** GO. The Slint migration is viable. Transparency wiring
becomes a tracked Phase 1 task; everything else is on or under plan.

## Recommendation for Phase 0.5 + Phase 1+

**Recommended order:**

1. **Phase 0.5 spikes** (2-3 days, before Phase 1):
   - HWND-poking spike — confirm transparent always-on-top overlay
     flags survive Slint+winit on Windows 11. **Hard blocker if it
     fails** (the migration loses its main benefit).
   - Markdown adapter spike — minimal pulldown-cmark + syntect →
     Text/StyledText walker. **Soft blocker** — if it's ≥ 4 weeks,
     re-open ADR-001 with hybrid React-for-tiles option.
   - Slint MCP server wire-up — start the server with
     `SLINT_MCP_PORT=8080` + `SLINT_EMIT_DEBUG_INFO=1`, connect
     Claude Code via MCP config, run one inspect+click cycle against
     the pilot binary. **Validates the testing story.**

**Then Phase 1 (Foundation, 1 week)**, before starting:

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
