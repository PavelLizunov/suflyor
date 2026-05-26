# Release Verification Methodology

**Established 2026-05-26 after v0.0.34 shipped a P0 infinite-grow bug
that user caught immediately on first launch («окно уехало в
бесконечность»).** Cause: built + committed + tagged + pushed + released
WITHOUT actually running the binary even once. Tests passed; static
checks passed; but the dynamic resize-loop only manifests at runtime.

Going forward, **every release MUST pass all 6 gates before push**.
No exceptions. Tests + clippy + tsc are necessary but NOT sufficient.

---

## Gate 1 — Static checks (necessary, not sufficient)

```bash
cargo test --lib                    # must pass 255+ tests
cargo clippy --all-targets -- -D warnings
npx tsc --noEmit
```

All three exit 0. **Pass:** proceed. **Fail:** fix before going further.

---

## Gate 2 — Build

```bash
npm run tauri build -- --bundles nsis
```

Must produce `src-tauri/target/release/bundle/nsis/suflyor_X.Y.Z_x64-setup.exe`.
**Pass:** proceed. **Fail:** rebuild after fixing.

---

## Gate 3 — Install on test environment

The user's working machine is the test environment (we have no separate
CI/staging). Install the new `.exe` via one of:

- **Live install** through the in-app updater: Settings → 🔧 Обновления
  → 🚀 Скачать и установить (one-click). Works once a release is on
  GitHub — chicken-and-egg for first install.
- **Manual install** for unreleased versions: run the local
  `suflyor_X.Y.Z_x64-setup.exe` directly. Quit any running instance
  first via tray → Quit OR `taskkill /F /IM overlay-mvp.exe`.
- **For computer-use sessions:** before pushing the release, use
  `mcp__computer-use__open_application` + screenshot to verify the
  EXISTING (pre-release) build still works and capture baseline. After
  the new build is installed (manually or via one-click), screenshot
  again and compare.

**Pass:** new version is running. **Fail:** investigate startup crash
report in `%APPDATA%\overlay-mvp\crash-report.txt`.

---

## Gate 4 — Smoke test (visual)

Take a screenshot via `mcp__computer-use__screenshot`. Verify:

1. **Overlay bar appears** at sane position (top of primary monitor by
   default) and sane size (~520-1000 px wide depending on chips).
2. **Status indicator** is rendered (🟢 Listening or similar).
3. **Hotkey strip** is visible (`F3·F4·F6·F8·F9·F10·F11·ℹ`).
4. **Gear icon** is visible at the right edge.
5. **No off-screen window:** the bar's right edge must be ≤ screen
   width. The bar's left edge must be ≥ 0.
6. **No window flicker / continuous resize:** wait 5 seconds and screenshot
   again — the bar must have the same dimensions. Catches infinite-grow
   bugs (like v0.0.34 had).

**Pass:** all 6 visual checks. **Fail:** version is broken, do NOT push.

---

## Gate 5 — Feature-specific verification

For any release that changes interactive behavior, manually exercise
the changed surface:

- **Settings changes:** click ⚙, navigate the new panels, verify each
  field still saves (toggle + Save + close + reopen, value persists).
- **Hotkey changes:** trigger each hotkey, verify the expected action.
- **Tile changes:** F6 manual spawn, verify tile appears at the right
  size + position.
- **Update flow changes:** trigger the one-click updater path (against
  a previous-version GitHub release) and verify it doesn't hang at the
  toast / fallback paths.

**Pass:** all changed surfaces still work. **Fail:** specific
regression — fix and rebuild.

---

## Gate 6 — Quit cleanly

Click tray → Quit OR ⚙ → ✕ Выйти. Confirm dialog must say the right
label (v0.0.31 fixed this — «Выйти» not «Удалить»). Process must
terminate within 2 seconds.

Verify no orphan processes:

```bash
tasklist | grep overlay-mvp     # must show 0 instances
```

**Pass:** clean termination. **Fail:** investigate; orphan processes
hold global hotkeys and break next launch.

---

## After all 6 gates pass

ONLY THEN:

```bash
git add <files>
git commit -m "vX.Y.Z - ..."
git tag -a vX.Y.Z -m "..."
git push origin master
git push origin vX.Y.Z
gh release create vX.Y.Z <path-to-nsis-installer> --title "..." --notes "..."
```

---

## What this didn't catch

- **Live audio capture / Whisper / Claude** — these need a real call,
  which is impractical for every release. Trust unit tests for these.
- **Performance regressions** — no benchmarking in the loop. Trust
  user reports.
- **Memory leaks over long sessions** — would need 60+ minute soak,
  out of scope.

## Anti-pattern reference

What v0.0.34 did wrong (the bug we want to prevent recurring):

```js
const intrinsicBarW = bar.scrollWidth;  // <-- equals offsetWidth when fits
const neededW = intrinsicBarW + 50;
if (neededW > currentW + 4) setSize(neededW, ...);
// After grow: scrollWidth = newWidth, neededW = newWidth + 50, > current + 4
// → setSize again → ∞
```

What v0.0.35 does right:

```js
// Sum children widths + gaps + padding — STABLE regardless of bar size
let intrinsic = children.reduce((s, c) => s + c.offsetWidth, 0)
  + gap * (children.length - 1) + padL + padR;
// + hard safety: never exceed screen width
const maxAllowedW = Math.max(520, screen.availWidth - 20);
const neededW = Math.min(intrinsic + 50, maxAllowedW);
if (neededW > currentW + 4) setSize(neededW, ...);
```
