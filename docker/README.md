# docker/ — hermetic test containers (Tier 4)

Per the suflyor GUI strictness spec § 6 — each test layer runs in its
own image so dependency drift can't affect results.

## What's scaffolded

| File | Layer | Status | Run |
|---|---|---|---|
| `static.Dockerfile` | 1 + 3 | scaffolded | `docker build -f docker/static.Dockerfile -t overlay-static . && docker run --rm overlay-static` |
| `unit.Dockerfile`   | 2     | scaffolded | `docker build -f docker/unit.Dockerfile   -t overlay-unit   . && docker run --rm overlay-unit` |

## What's NOT scaffolded (and why)

| Spec layer | overlay-mvp status | Reason |
|---|---|---|
| component | covered by `unit.Dockerfile` (vitest runs all *.test.{ts,tsx}) | not a separate image — the surface is small enough that Vitest's `jsdom` env handles both pure-fn and component tests |
| visual | NOT scaffolded | Tauri 2 on Linux uses WebKit2GTK; the user's overlay window only ships against WebView2 on Windows. Visual-diff PNGs from a non-Windows render would diverge on font/AA/DWM rendering. See [scripts/visual_check.ps1](../scripts/visual_check.ps1) for the local Windows BitBlt equivalent. |
| e2e | NOT scaffolded | Same: tauri-driver + WebdriverIO would test the WebKit2GTK build, not the WebView2 build the user runs. Defer until either (a) we run a Windows CI container, or (b) we add a WebKit2GTK build target with its own visual baseline. |
| mutation | NOT scaffolded | `cargo-mutants` works but the baseline run on 260 tests = hours. Nightly-only per spec. Add when there's time to triage survivors. |
| fuzz | NOT scaffolded | No clear fuzz target right now (most inputs are KB strings; fuzzed input is a UI ergonomics decision not a safety one). |
| soak | NOT scaffolded | Memory regressions surface in 60-min user sessions; Docker can't simulate WASAPI capture. Defer to dogfooding. |
| perf | NOT scaffolded | Bundle-size budget already enforced by `npm run tauri build` (warns above 500 KB). FPS budget needs a real WebView2 render. |

## CI integration

There is no remote CI yet. The local equivalents:

| Local | Replaces |
|---|---|
| `npm run ci` | static.Dockerfile (layers 1+3) |
| `npm test` + `cargo test --lib + --test copy_contract` | unit.Dockerfile (layer 2) |
| `npm run lint` | (in ci.ps1) |
| `.claude/hooks/git-gate.ps1` | merge gate (blocks `git commit` / `git push`) |

When remote CI is added, point the workflow at `static.Dockerfile` +
`unit.Dockerfile` and skip the docker layer when running on the
developer's Windows machine.
