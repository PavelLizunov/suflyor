# Release Verification Methodology

**Established 2026-05-26 after a release shipped a P0 bug the user caught
on first launch** (built + committed + tagged + pushed WITHOUT running the
binary once). Tests + clippy are necessary but NOT sufficient — some bugs
only manifest at runtime. Every release MUST pass all gates before push.

---

## Gate 1 — Static checks

```powershell
powershell scripts\ci.ps1     # fmt + clippy -D warnings + tests (slint + overlay-backend)
```

Exit 0. **Fail:** fix before going further.

---

## Gate 2 — Build the installer

```powershell
powershell scripts\build-slint-release.ps1 -Installer
```

Must produce `slint-experiment\target\release\bundle\suflyor-slint-setup.exe`.

---

## Gate 3 — Install on the test machine

No separate CI/staging — the dev machine is the test environment. The
installer goes to `%LOCALAPPDATA%\suflyor-slint\` (user-level, coexists
with any prior build). Quit a running instance first:
`Get-Process overlay-host | Stop-Process -Force`.

---

## Gate 4 — Smoke test (visual)

Launch + screenshot (`scripts\capture_window.ps1 -TitleLike "overlay-mvp (Slint)"`
or `capture_primary.ps1`). Verify:

1. Overlay bar appears at a sane position + size; right edge ≤ screen width, left ≥ 0.
2. Status pill + chips render; no missing-glyph boxes.
3. Wait ~5 s, screenshot again — dimensions stable (no resize loop / flicker).

**Fail:** broken — do NOT push.

---

## Gate 5 — Feature-specific verification

Exercise whatever changed:
- **Settings:** open ⚙, navigate the panel, toggle + Save + reopen → value persists.
- **Hotkeys:** F3 / F4 / F6 / F9 trigger the expected action.
- **Tiles:** F6 / +тайл spawns a tile at a sane size/position; drag, pin, close work.
- **Push-to-record:** hold 🎤/🔊 → tile with answer.

---

## Gate 6 — Quit cleanly

⚙ → ✕ or the bar's X. Process terminates within ~2 s:

```powershell
Get-Process overlay-host -ErrorAction SilentlyContinue    # 0 instances
```

Orphan processes hold the global hotkeys and break the next launch.

---

## After all gates pass

```powershell
git add <files>
git commit -m "vX.Y.Z - ..."
git push origin master
gh release create vX.Y.Z slint-experiment\target\release\bundle\suflyor-slint-setup.exe --title "..." --notes-file notes.md
```

## What this does NOT catch

- Live audio / Whisper / Claude round-trips — need real calls; trust unit tests + occasional manual checks.
- Performance / memory over long sessions — out of scope; trust user reports.
