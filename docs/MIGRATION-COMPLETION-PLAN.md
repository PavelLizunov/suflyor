# Migration completion plan — slint/main → master cut

Document captures the remaining work to take `slint/main` from
"UI shell with stub callbacks" to "shippable v0.2.0 that replaces
React/Tauri v0.1.1". Companion to [PHASE-7-CUT-PLAN.md](PHASE-7-CUT-PLAN.md)
(which covers the irreversible delete step).

## Goal

`slint/main` reaches feature parity with master (v0.1.1), then the
Phase 7 cut deletes React/Tauri and merges to master as v0.2.0.

## Phases A-E (parallelisable where indicated)

### Phase A — Parallel investigations (3 agents simultaneously)

All three are READ-ONLY audits. Each produces a structured table the
later phases consume. Run in parallel.

| Agent | Input | Output |
|---|---|---|
| **A1: Tauri commands audit** | `src-tauri/src/lib.rs` | Table of every `#[tauri::command]` fn, classified as `extract` (move to Tauri-free module) / `port-slint-side` (UI command, no real backend) / `drop` (feature being removed). |
| **A2: Backend module Tauri-coupling** | `src-tauri/src/{audio,stt,ai,journal,kb,hotkeys,screenshot,config}.rs` | Per-module list of Tauri-specific imports + functions. Verdict: can each module compile standalone? If not, what's the minimal refactor? |
| **A3: i18n string survey** | `src/i18n.ts` + all `slint-experiment/ui/*.slint` | Mapping of (a) existing React/TS i18n keys, (b) hardcoded English strings in Slint, (c) which Slint files need `@tr()` wiring per migration. |

### Phase B — Setup (sequential after Phase A)

1. Create new crate `overlay-backend/` at repo root (or under `crates/`).
2. Update repo to Cargo workspace with members: `src-tauri/`, `slint-experiment/`, `overlay-backend/`.
3. Move audio/stt/ai/journal/kb/screenshot/config modules from
   `src-tauri/src/` into `overlay-backend/src/` per Phase A2 verdict.
   Tauri-coupled bits stay in src-tauri's wrappers.
4. `slint-experiment/Cargo.toml` adds path dep on `overlay-backend`.

### Phase C — Backend wiring in overlay-host

Replace every stub callback in `overlay-host.rs` with a real call:

| Callback | Today (stub) | Phase C target |
|---|---|---|
| `mic_toggle_clicked` | logs to stderr | `overlay_backend::audio::toggle_mic()` |
| `sys_toggle_clicked` | logs to stderr | `overlay_backend::audio::toggle_sys()` |
| `timer_toggle_clicked` | resets `session_secs` | `overlay_backend::runtime::start_session()` + `stop_session()` |
| `ai_model_cycle_clicked` | rotates string | `overlay_backend::config::set_ai_model()` |
| `spawn_tile_clicked` | sample markdown | `overlay_backend::ai::ask(prompt)` + render response |
| `result_activated` (palette) | sample blocks | `overlay_backend::kb::get(key)` + render |
| `bookmark_clicked` | stub log | `overlay_backend::journal::write_bookmark()` |
| Settings tab content (per panel) | static lists | per-panel data sourced from `overlay_backend::config` |

### Phase D — Final integration (parallel where independent)

| Agent / task | Independent? |
|---|---|
| D1: gettext setup — `slint-tr-extractor` → `i18n.pot` → `ru.po` + `en.po` → `slint_build::CompilerConfiguration::with_bundled_translations()`. Rewrite all `.slint` strings to `@tr("key")`. | Yes — only touches Slint files |
| D2: F4 global hotkey — `RegisterHotKey(F4)` via windows crate + message-pump bridge to slint::invoke_from_event_loop. | Yes — overlay-host hotkey wiring, no backend dep |
| D3: NSIS installer — replace tauri-build's installer with a hand-rolled NSIS script (or `cargo-nsis`). Same product ID as v0.1.1 so it upgrades in place. | Yes — independent of code |
| D4: review-agent re-audit | Yes — runs on the final diff |

### Phase E — Phase 7 cut

Per [docs/PHASE-7-CUT-PLAN.md](PHASE-7-CUT-PLAN.md): delete React/Tauri,
rename `slint-experiment` → `src-slint` (or keep), update CLAUDE.md,
build NSIS, install, on-machine smoke, merge to master, tag v0.2.0,
GitHub release.

## Estimated effort (agent-fast pace)

| Phase | Effort |
|---|---|
| A — parallel investigations | ~30 min wall (3 agents simultaneously) |
| B — backend extraction setup | 2-4 hours (depends on A2 verdict) |
| C — backend wiring | 3-5 hours (per-callback replacement) |
| D — i18n + hotkey + installer + review | 2-4 hours (parallel) |
| E — cut + ship | 1-2 hours (mechanical after preconditions) |
| **Total** | **~10-18 hours of agent work** spread across multiple sessions |

## Risks

- **A2 may surface that backend modules are heavily Tauri-coupled.**
  Mitigation: per-module refactor cost lands in B; if too high, hybrid
  strategy (keep Tauri shell, replace only UI surface) is the fallback.
- **gettext extraction in D1** is mechanical but tedious. Mitigation:
  let `slint-tr-extractor` do the heavy lifting, then translate.
- **NSIS installer rebuild** requires figuring out the right packaging
  story without `tauri-build`. Mitigation: `cargo-nsis` or vanilla NSIS
  script; both are well-trodden Windows paths.
- **Backend extraction breaks src-tauri's test suite.** Mitigation:
  preserve every test by moving it alongside the function it tests
  into `overlay-backend/tests/`.
