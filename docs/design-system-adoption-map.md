# Design-system adoption map — Этап 1 current state (2026-06-03, post-v0.9.1)

Companion to `docs/slint-design-system-and-safe-redesign-plan.md`. A **data snapshot**
of which UI surfaces already route through the declarative token globals (`Theme`
colours + the new `Metrics` sizes) vs still carry raw literals — so the surface-by-
surface migration (plan §7 Этап 1 / Этап 4) can be planned and tracked. Re-run the greps
in **Method** after each migration to watch the numbers move.

## Token globals
- `ui/theme.slint` — 16 colour roles × 4 schemes (the source of truth for colour).
- `ui/metrics.slint` — **NEW (Track 3)**: size / spacing / typography / border / motion /
  opacity tokens. The dimensional sibling of `Theme`.

## Adoption per surface
`Theme.` / `Metrics.` = number of uses of each token global in the file; `raw hex` =
`#rrggbb` literals **outside** theme.slint (a colour-token-gap indicator — but some are
LEGITIMATE fixed-colour exceptions, so glance per-line before migrating).

| Surface | §6 risk | `Theme.` | `Metrics.` | raw hex | State |
|---|---|---|---|---|---|
| recover_offer | Low | 11 | 12 | 0 | **fully on tokens** (Track 3) |
| wizard | Medium | 33 | 23 | 0 | colour clean; **sizes done** (batch 2) |
| text_ask | Medium | 4 | 3 | 0 | colour clean; **sizes done** (batch 2) |
| help | Low | 19 | 16 | 1 | sizes done (Track 3); 1 stray hex |
| palette | Medium | 15 | 17 | 1 | colour ~done; **sizes done** (batch 2) |
| tile | High | 38 | 0 | 3 | colour ~done; sizes pending |
| settings_panel | Medium | 111 | 0 | 6 | colour ~done; sizes pending; **84 KB → split first (Этап 3)** |
| overlay_bar | High | 35 | 0 | 15 | heavy Theme use BUT 15 raw-colour exceptions to review |
| capture_overlay | Critical | 0 | 0 | 14 | **no Theme** — but colours are likely INTENTIONALLY fixed (selection rect / dim wash, theme-independent by design). Confirm; do not migrate blindly |
| replay | Low | 0 | 0 | 15 | **no Theme at all** — hardcoded dark; genuinely ignores all 4 schemes incl Light Frost. Real colour-migration candidate |

(`ui/overlay_spike.slint` + `ui/markdown_spike.slint` are dev SPIKES, not shipped UI — ignored.)

## Reading
- **Colour-token adoption is already broad.** Every SHIPPED surface except `replay` and
  `capture_overlay` uses `Theme`; `wizard` / `text_ask` / `recover_offer` are fully clean.
- **The one real colour gap is `replay`** (0 `Theme`, hardcoded dark) — it will not follow
  a theme switch (e.g. stays dark in Light Frost). `capture_overlay`'s raw colours are most
  likely by-design (a fullscreen selection overlay should look the same in every theme);
  verify before touching.
- **Size-token (`Metrics`) adoption now spans 5 surfaces** (help, recover_offer, text_ask, palette, wizard). Extending it is
  **value-preserving (zero visual change)** and can proceed surface-by-surface; the
  `Metrics` global already holds the full taxonomy, so a migration is a mechanical
  equal-value swap + a glance.
- **`overlay_bar`'s 15 raw-colour exceptions** are the next colour cleanup worth a look
  (High risk → run the §8 visual matrix per change).

## Suggested order (low risk first, per plan §7 Этап 4)
- **Sizes** (value-preserving, safe to make + confirm with a build + a glance): done for
  help / recover_offer / text_ask / palette / wizard; remaining `tile → settings (split
  first, Этап 3) → overlay_bar`.
- **Colours** (per-theme visual change — needs the §8 matrix on Light Frost + a dark
  theme): `replay` (worst) → `overlay_bar` exceptions → `capture_overlay` only if its
  fixed colours are judged wrong.

## Method
```
rg '#[0-9a-fA-F]{3,8}' ui/*.slint -c     # raw hex per file
rg 'Theme\.'   ui/*.slint -c             # colour-token uses per file
rg 'Metrics\.' ui/*.slint -c             # size-token uses per file
```
Captured 2026-06-03, post-v0.9.1 (commit 82c9c3f).
