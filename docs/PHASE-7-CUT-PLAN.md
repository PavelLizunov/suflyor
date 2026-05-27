# Phase 7 cut plan — removing React/Tauri from the repo

**Status:** Documented 2026-05-27 on slint/main. **DO NOT EXECUTE
this plan yet** — the Slint stack has feature-complete shells for
Phases 0-5 but no backend wiring (audio/stt/ai/journal/config calls
still live behind Tauri commands in `src-tauri/src/lib.rs`).
Executing the cut before backend wiring lands would leave the repo
unable to run.

## Pre-execution gate

Phase 7 may run only when ALL these have shipped to `slint/main`:

1. **Backend wiring complete** — every `#[tauri::command]` in
   `src-tauri/src/lib.rs` either:
   - has been deleted (the feature is fully ported / dropped), OR
   - has its underlying logic extracted into a Tauri-free module
     that the slint binary calls directly (e.g. `audio::start_mic`,
     `stt::transcribe`, `ai::ask`, `journal::write_event`).
2. **Slint binary stands up the full v0.1.1 feature set** — overlay
   bar + tiles + settings + replay + palette + hotkeys + autosession
   + auto-tiles + debrief + KB search. Verified via 6-layer gate
   (clippy / cargo test / Slint MCP tests / review-agent / live
   install + smoke / visual baseline).
3. **i18n migrated** — every user-facing string in the Slint UI
   either uses `@tr("key")` with a populated `i18n.pot` → `ru.po` +
   `en.po`, OR is bundled English-only with a tracked TODO for
   later. The current Slint shell hardcodes English strings; gettext
   wiring is Phase 6 proper.
4. **NSIS installer for the Slint binary** exists and installs to
   `%LOCALAPPDATA%\suflyor\` with the same exe name (`overlay-mvp.exe`)
   so existing user state directories (`%APPDATA%\overlay-mvp\`) are
   reused without migration.
5. **Smoke test on the user's machine** — primary 1920x1080 + secondary
   1200x1920 portrait at x=-1200. Spawn 3 tiles, switch monitors,
   toggle stealth, record a session, check Replay shows it.
6. **review-agent pass on the entire cut diff** — bigger than any
   single Phase commit; expect 10+ findings to address.

## Files to DELETE (React + Tauri sides)

```text
src/                                          # entire React frontend
├── App.tsx, Overlay.tsx, Tile.tsx, Settings.tsx, Replay.tsx,
├── i18n.ts, i18n.test.ts, clock.ts, clock.test.ts,
├── styles.css, main.tsx, vite-env.d.ts, __tests__/
src-tauri/                                    # entire Tauri shell
├── Cargo.toml, Cargo.lock, build.rs, tauri.conf.json,
├── capabilities/, icons/, gen/, target/,
├── src/main.rs, src/lib.rs (after backend modules moved out),
├── src/tray.rs, src/hotkeys.rs (after re-port to slint hotkey crate)
index.html
package.json, package-lock.json
vite.config.ts, vitest.config.ts
tsconfig.json, tsconfig.node.json
eslint.config.js, .eslintrc.cjs (if any)
node_modules/                                 # (gitignored anyway)
docker/static.Dockerfile                      # tsc / vitest no longer needed
docker/unit.Dockerfile                        # cargo test now in src-slint/
```

## Files to KEEP

```text
.git/                                         # obviously
.github/                                      # CI workflows (may need update)
.gitignore                                    # update: drop src-tauri/target,
                                              #         add slint-experiment/target/
                                              #         or src-slint/target/
CLAUDE.md                                     # rewrite per below
README.md                                     # rewrite per below
docs/                                         # all ADRs + plan + report
NIGHT_RUN_PLAN.md                             # historical log
.claude/                                      # hooks + autonomous rules
scripts/                                      # update visual_check.ps1 to use Slint MCP
sessions/                                     # gitignored user state
slint-experiment/                             # PROMOTE TO REPO ROOT (rename to "src-slint/")
```

## Files to MOVE (the actual restructure)

```text
slint-experiment/             →  src-slint/
slint-experiment/Cargo.toml   →  Cargo.toml (root)  (after merging settings)
slint-experiment/src/*        →  src-slint/src/*
slint-experiment/ui/*         →  src-slint/ui/*
slint-experiment/src/bin/*    →  src-slint/src/bin/* (or eliminate spike bins)
```

If keeping `slint-experiment/` is preferred (less churn), the cut
just deletes the React/Tauri stuff and leaves the Slint crate where
it sits. Naming-wise though, `slint-experiment` is a misnomer once
it's the canonical UI.

## Backend extraction (the hard part — pre-Phase-7 work)

Each Tauri command in `src-tauri/src/lib.rs` needs the same treatment:

1. Move the function body into the relevant `src-tauri/src/<module>.rs`
   as a non-Tauri function (no `#[tauri::command]`, no
   `tauri::WebviewWindow` parameter, no `assert_overlay`).
2. Delete the Tauri wrapper that previously called it.
3. Add the new function to a fresh `crates/overlay-backend/` crate
   (or expose the existing `audio.rs` / `stt.rs` / etc. as a lib
   without `tauri` in their dep tree).
4. `src-slint/` adds a `[dependencies]` entry on the new backend
   crate via path dep.
5. Update overlay-host binary to call the backend functions where
   it currently has stubs (mic_toggle → backend::audio::start_mic,
   ai_model_cycle → backend::ai::set_model, spawn_tile → backend
   ::ai::ask + KB lookup, etc.).

Estimated effort:
- Backend extraction: 3-5 days (touches every module, requires careful
  test re-validation since src-tauri tests assume Tauri's runtime).
- Wiring slint UI to backend: 2-3 days (every callback in
  overlay_host.rs gets a real implementation).
- i18n migration: 1-2 days (extract → po → bundle).
- Final cut + NSIS installer + verification: 1-2 days.

**Total Phase 7 effort: ~10 working days** of careful work.

## Cut execution order (once all preconditions met)

```text
1.  git checkout slint/main
2.  Verify all preconditions above (manual gate, not automated).
3.  Move slint-experiment/ → src-slint/ (rename) OR commit to leaving
    it as-is.
4.  Move backend modules to crates/overlay-backend/ (or the agreed
    layout).
5.  Update root Cargo.toml (workspace with src-slint/ + crates/
    members).
6.  Delete src/, src-tauri/, package.json, vite.config.ts, etc.
    per "Files to DELETE" list above.
7.  Update .gitignore (drop src-tauri entries, keep target excludes).
8.  Update CLAUDE.md — drop Tier 2/4 React-Vitest sections, add
    Slint testing patterns (MCP server, accessible-label queries,
    cargo test --bin overlay-host).
9.  Update README.md if any.
10. Update scripts/visual_check.ps1 to launch overlay-host via Slint
    MCP screenshot instead of the prior NSIS install dance.
11. Update git-gate.ps1 to point at the new manifest paths.
12. cargo build (full workspace) — must be clean.
13. cargo clippy --workspace --all-targets -- -D warnings — clean.
14. cargo test --workspace — all green.
15. NSIS installer build (replaces tauri-build's installer dance).
16. Install + 6-layer smoke verify.
17. Bump to v0.2.0. Commit with detailed message ("PHASE 7 CUT —
    React + Tauri removed; pure Slint + Rust release"). Push.
18. Merge slint/main → master (force or merge per user preference).
19. GitHub release v0.2.0 with the new NSIS installer.

This is intentionally a single irreversible step from v0.1.1
React/Tauri to v0.2.0 Slint+Rust. There's no hybrid "Tauri + Slint
side by side" state — the migration plan rejected that for
clarity, and the cut commit is the moment of truth.

## Rollback strategy

If anything is wrong post-cut:

- Master at the v0.1.1 commit before merge is the rollback point.
- `git checkout master && git reset --hard <pre-merge-sha>` reverts
  the cut entirely. Slint work survives on the slint/main branch.
- The user's existing v0.1.1 NSIS installer continues to work; the
  v0.2.0 installer can be uninstalled if released prematurely.

## Open questions for the user before Phase 7 executes

1. Workspace layout: `crates/overlay-backend/` + `src-slint/` at
   root, OR keep `slint-experiment/` as the single crate with
   backend modules under `slint-experiment/src/backend/`?
2. Should the v0.2.0 installer ship under the same NSIS product ID
   so it cleanly upgrades v0.1.1 installs? (Recommended yes.)
3. Are there v0.1.1 features the user explicitly wants DROPPED in
   the migration (translate-tile button, debrief, ai-bridge probe,
   etc.) rather than ported? The kickoff said "ask before dropping
   a feature" — this is the consolidated ask.
