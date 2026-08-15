# RC16 tray, streaming math, and TTS-speed audit

## Conditions

- Host: Winbrat test VM; no product build or runtime was started on the owner workstation.
- Candidate: `v0.37.0-rc.16`, built with `ui-mcp`, `SLINT_EMIT_DEBUG_INFO=1`, and the `winit-software` renderer.
- Tile: production `TileWindow`, 800 x 600, English, synthetic algebra only, active player labelled 1.5x.
- Tray: production `overlay-host`; the real hidden notification-window callback received both legacy `WM_RBUTTONUP` and v4 `WM_CONTEXTMENU` for one context activation.
- Baseline: the reporter screenshots were inspected but are not committed; the privacy-safe RC15 baseline remains in `docs/audit-2026-08-15-math-player-tray/`.

## Evidence

- `math-player-accepted.png` — fractions, roots, scripts, a system of equations, three matrices, and the active 1.5x player.
- `tray-menu-accepted.png` — the custom themed tray menu opened through the production notification callback.

## Results

- Common algebra TeX is display-normalized without changing copy/persistence text. The accepted tile contains no raw `\\begin`, `\\end`, `\\frac`, or `\\sqrt`; matrix and `cases` rows remain distinct even when CommonMark unescapes a bare row separator.
- Inline/fenced code, URLs, and currency remain literal. A growing bare or explicitly delimited formula is withheld until its line/delimiter completes, while ordinary prose continues streaming.
- The tile player starts in the remembered 1.5x state. Piper and Tera controller tests verify that the saved speed is applied to a newly created player before its first audio; Winbrat has no default audio endpoint, so audible-speed judgment remains an owner acceptance item.
- Posting legacy and v4 context events back-to-back produced exactly one 196 x 136 menu at (604, 536). The menu is custom themed, within the desktop work area, and the duplicate event was suppressed.
- The global-hotkey smoke was not repeated because this diff does not touch shortcut registration or routing; the complete smoke on the immediately preceding RC15 base remains recorded in the baseline audit.
