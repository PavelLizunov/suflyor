# Regression scout — v0.7.0 → v0.8.1 wave → v0.8.2 fixes

Autonomous bug-scout + regression check of the resilience/escalation release
wave (Поток A/B/C/D: stealth bar-flash, error visibility, emergency restart,
cloud escalation, sticky-cloud). Requested while the user was away, full
autonomy. Method: four adversarial scout agents over distinct surfaces +
one inline owner-analysis of the restart/singleton surface (its agent died
on an API rate-limit), then ground-truth verification of every claim against
the code before any change, then fixes + independent review + live smoke.

## Surfaces swept

1. **AI routing / tile continuation** — `AskRoute{Text,Vision,Cloud}` refactor,
   cloud escalation, sticky-cloud `LiveRoute`, all 5 `stream_chat` call sites.
2. **Restart / single-instance / startup-stealth** — named-mutex singleton,
   `--relaunch` lifecycle, teardown, park-before-show.
3. **Error visibility / health** — `HealthSignals` + `last_ai_err_ms`,
   `TileKind::Error`, auto-tile error path, debounce, session resets.
4. **Slint UI / i18n / release hygiene** — bar/tile glyphs, RU `.po`, version
   sync, installer asset name, auto-updater compatibility.

## Verified CLEAN (no change needed)

- **Restart never kills a user-started llama-server.** `local_ai_servers`
  holds ONLY processes the in-app installer spawned (`extend(res.servers)`);
  a manually-started server is untracked, so teardown can't reach it.
- **Restart can't strand the user.** Child waits 8 s on the singleton; parent
  exit (server-kill ≈ ms + `tokio shutdown_timeout(2 s)`) ≈ 2–3 s. ~3× margin.
  Crash recovery is covered by `WAIT_ABANDONED`.
- **Glyphs.** ⚠ (U+26A0) renders as a color triangle on Slint+skia (empirically
  rendered; control glyphs ✕/⟳ boxed in the same harness, proving the test).
  🔄 🧠 🎤 ⏹ are color-emoji and render.
- **Version + auto-update.** `Cargo.toml` / `Cargo.lock` / installer
  `PRODUCT_VERSION` in sync; asset name `suflyor-slint-setup.exe` unchanged;
  `update.rs` untouched → testers update v0.8.x cleanly.
- **Secret hygiene.** Auto-tile error tiles use `classify_ai_error` (static
  strings only); raw chains reach only the local log + journal, never a
  screen-shared tile. Confirmed across STT / vision / voice error surfaces.
- **Routing.** F9→Text, Shift+F9→Cloud, PTT→Text, F8→Vision throughout,
  follow-up/regen/voice read the live route at click time; Vision→Cloud is
  structurally impossible; vision follow-ups keep the screenshot.

## Fixed in v0.8.2

| ID | Sev | Surface | Fix |
|----|-----|---------|-----|
| C1 | High | health/bar | Gate the "⚠ AI недоступен" text+color SET on `timer_active`. A stale `{ai:down}` tick landing after `session:stopped` (emitter already aborted) could otherwise strand the bar red over an idle session forever. |
| M1 | High | auto-tile | Re-check `session_gen` on the error path (after log+journal) before marking AI down / spawning the error tile — mirrors the success path. Stops a slow failed call from a stopped session poisoning the next one. |
| N2 | Low | stop_session | Reset `last_ai_err_ms` + `last_ai_error_tile_ms` on stop, symmetric with start (defense-in-depth for M1). |
| MAJOR-2 | High | cost | Sticky-cloud follow-ups/regenerates are billable after escalation; emit the same non-blocking `cost:cap-hit` warning `fire_f9_ask` already did (was silent mid-conversation). |
| M1-UI | High | bar layout | Hide the four secondary chips while a confirm is armed so the RU "Перезапустить? Да Нет" + wide AI-down status + close-all chip stop pushing the cancel button off the fixed-width bar. |
| m1 | Med | bar | Quit and restart confirms are now mutually exclusive. |

Each fix verified against ground truth before writing, then by an independent
review-agent (verdict SHIP — the C1 color half-fix it flagged was folded in),
then gated: clippy `-D warnings` + tests (backend 170, slint 20) + fmt, both
crates. Live smoke (Win32): v0.8.2 boots clean, bar on primary 1080×78, RU
un-armed layout intact (PrintWindow), secrets masked in the log.

## Intentionally NOT changed (with rationale)

- **AI-down latch never ages out within a live session** (`health.rs:81`).
  By design: the user explicitly asked for immediate, sticky down-signaling
  ("чтобы сразу понимал"). In a live interview the next successful auto-tile
  clears it within seconds; aging it out risks re-introducing the silent stop
  the feature was built to fix.
- **Escalate / Shift+F9 with an UNCONFIGURED cloud bridge** shows a generic
  "bridge unreachable" tile instead of "configure the bridge in Settings".
  Real but a UX-polish gap, and not reachable for this user (their cloud
  bridge is configured). A pre-flight guard mirroring the auto-detector path
  is the clean follow-up.
- Vision-follow-up after toggling vision off mid-conversation; phantom
  micro-cost on a failed empty-bridge escalation; `meeting:ending` vs AI-down
  text flicker at end-of-session — all narrow/low-severity, left as-is.

## Result

Release **v0.8.2** published with the installer asset. No CRITICAL defects
escaped the wave; the six fixes close the genuine cross-session/stuck-state
bugs and the one cost footgun. HEAD = `b195c4d`.
