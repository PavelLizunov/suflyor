# Post-mortem: v0.0.34 P0 infinite-overlay-grow bug

**Date:** 2026-05-26
**Severity:** P0 (app unusable on first launch)
**Time to detect:** ~30 seconds (user caught it immediately)
**Time to fix + ship hotfix:** ~25 minutes (v0.0.35 published)
**Root cause:** misuse of `Element.scrollWidth` for intrinsic width measurement
**Long-term mitigation:** `RELEASE_CHECKLIST.md` 6-gate verification — no
release without a computer-use smoke test that catches dimension churn

## Timeline

- 13:50 — v0.0.34 starts. Goal: make overlay bar manually resizable
  (user couldn't drag it). Two refactors:
  1. Remove the 50%-of-screen cap so chips don't get hidden
  2. Switch ResizeObserver from `entry.contentRect.width` (which always
     equals window width) to `bar.scrollWidth` for "intrinsic" measurement
- 14:05 — v0.0.34 passes Gate 1 (255 tests, clippy, tsc all clean)
- 14:08 — v0.0.34 passes Gate 2 (NSIS built)
- 14:10 — v0.0.34 committed + tagged + pushed + released **without**
  Gates 3-6 (no install, no smoke test, no resize verification, no quit
  check)
- 14:35 — user installs v0.0.34, launches, reports «окно уехало в
  бесконечность»

## What `bar.scrollWidth` actually returns

The W3C spec: `scrollWidth` is the maximum of `clientWidth` and the
content overflow. **It equals `clientWidth` when content fits inside
the element.**

For our `.overlay-bar` flex container:
- Bar's CSS width is implicit (stretches to fill parent `.overlay-root`,
  which fills the window)
- Children sum to ~470 px natural
- Window is, say, 720 px wide
- Bar's clientWidth ≈ 720
- Children fit in 720 → `scrollWidth` = 720 (= clientWidth)

**Not 470.** The whole "intrinsic measurement" was wrong from the start.

## The infinite-grow logic

```js
const intrinsic = bar.scrollWidth;          // ← 720, not 470
const needed = intrinsic + 50;              // ← 770
if (needed > currentW + 4) setSize(needed); // 770 > 724 → grow to 770
// Window resizes → bar reflows → scrollWidth recomputes = 770
// Next RO fire: intrinsic = 770, needed = 820, > 774 → grow to 820
// ... forever
```

The user reported the overlay window expanded until it hit the screen
edge (then visually clipped — Tauri/WebView2 allowed expansion past the
monitor).

## Why static checks passed

- Unit tests don't run the WebView2 reflow → can't observe scrollWidth
  behavior
- TypeScript types `scrollWidth: number` — no semantic info on what it
  represents
- Clippy doesn't analyze frontend JS
- Vite build just bundles, doesn't execute

The only signal would have been **actually launching the binary and
watching it for a few seconds**.

## The fix in v0.0.35

```js
// Real intrinsic = sum of children offsetWidth + gaps + padding.
// With `.overlay-bar > * { flex-shrink: 0 }`, each child's offsetWidth
// IS its natural width regardless of window size. Stable.
let intrinsic = children.reduce((s, c) => s + c.offsetWidth, 0)
  + gap * (children.length - 1) + padL + padR;

// Hard cap so even a future bug can't escape the monitor.
const max = Math.max(520, screen.availWidth - 20);
const needed = Math.min(intrinsic + 50, max);

if (needed > currentW + 4) setSize(needed);
```

Plus a one-shot `initialFitDoneRef` that allows shrink on the FIRST
ResizeObserver fire of a session — auto-corrects users with persisted
oversized state from v0.0.34.

## Secondary issue uncovered: the spacer

While fixing this, I discovered `.status-text { flex: 1 1 0 }` was
deliberately a spacer pushing other bar children to the right. As a
side-effect, `status-text.offsetWidth` equaled all the available bar
space (~600 px) rather than the width of the "Listening" text itself
(~80 px). So **even the corrected children-sum measurement was wrong**
until I changed `.status-text` to `flex: 0 0 auto`.

Lesson: any flex-stretched child invalidates a sum-based intrinsic
measurement. The right invariant is: every bar child must be sized to
its content.

## Lessons learned

1. **Static checks are necessary but not sufficient.** A clean test
   suite, clean clippy, and clean tsc don't catch runtime layout
   feedback loops. *Especially* not for code that uses
   `Element.scrollWidth` / `getBoundingClientRect` / browser reflow.
2. **`scrollWidth` is NOT intrinsic width.** It is a max over
   clientWidth and overflow. For a flex container with shrinkable
   children, it almost always equals clientWidth. The actual intrinsic
   width of children must be computed differently — usually by summing
   `offsetWidth` of children that explicitly opt out of shrinking via
   `flex-shrink: 0`.
3. **Resize observers + setSize is a feedback loop.** Any
   `RO → measure → setSize` chain that uses the resized dimension as
   input to the next size is infinitely recursive. The fix is either:
   - Measure something that doesn't depend on the current size (e.g.
     intrinsic children sum)
   - Skip the setSize if delta is below a threshold (we had +4 px but
     `+50` made it useless)
   - Throttle / debounce (band-aid, doesn't fix root cause)
4. **A 5-second visual smoke test would have caught this.** The bar
   grew from 520 to 1900 in less than 1 second. Any human or tool
   looking at it for 2 seconds would see "this is wrong".
5. **The hotfix path matters.** Once v0.0.34 shipped, users who
   installed it had persisted oversized window state. The hotfix needs
   to NOT rely on the user "fixing" it manually. v0.0.35's one-shot
   initial fit (allow shrink on first RO fire of session) handles this
   automatically.

## Action items

| # | Action | Status |
|---|--------|--------|
| 1 | `RELEASE_CHECKLIST.md` with 6 gates | ✅ Done |
| 2 | Gate 4 must include "dimensions stable over 5 sec" check | ✅ Codified |
| 3 | Gate 5 must include resize test for layout changes | ✅ Codified |
| 4 | Future similar bugs caught by gate 4 — verify via spot-check | ⏳ Ongoing |
| 5 | Add a runtime safety: panic if window dimensions change >10× in 1 second | 🔮 Future |
| 6 | Add a hidden DOM measurement element approach for truly bullet-proof intrinsic-width | 🔮 Future |

## What we won't change

- **Resize observers in general** — still the right tool for HEIGHT
  auto-growth. Just need to be careful about what we measure.
- **Tauri/WebView2** — they did nothing wrong. Our JS asked them to
  setSize a window to ever-larger values, they obediently complied.
- **The +50 padding requirement** — user explicit ask, kept it.
- **The grow-only / no-50%-cap policy** — both still correct, just
  needed to measure intrinsic correctly.
