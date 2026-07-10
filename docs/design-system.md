# suflyor UI convention

The source of truth is split by role:

- `ui/theme.slint` owns semantic colours and fonts.
- `ui/metrics.slint` owns dimensions, spacing, radii, typography sizes and motion.
- `assets/icons/README.md` owns the SVG grid and stroke convention.

## Spacing

Use the shared scale, not new magic numbers:

| Role | Token | Value |
| --- | --- | --- |
| tight inline gap | `Metrics.space-xs` | 4 px |
| control/row gap | `Metrics.space-sm` | 8 px |
| card padding/section gap | `Metrics.space-md` | 12 px |
| window/content padding | `Metrics.space-lg` | 16 px |

Existing calibrated geometry outside this scale may stay when it is part of a fixed HWND, pointer target or waveform layout. Do not introduce another spacing value without a measured reason.

## Controls and text

- Reuse `control-sm/md/lg` and `icon-button-size`; hover must not change geometry.
- Use `font-caption`, `font-label`, `font-body`, and `font-heading` by semantic role.
- Form labels sit above their fields; hints use the muted text colour.
- Buttons with the same role have the same height and retain a visible focus/click target.
- Long RU/EN text wraps or scrolls; it must not expand a fixed window off screen.

## Verification

Any UI change requires a Slint compile, the full gate, and a live CopyFromScreen check of the changed surface. Changes to tile/bar/window dimensions also require checking the paired Rust/Win32 sizing constants.
