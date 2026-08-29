# Tile selection geometry audit

- Date: 2026-08-29
- Baseline: `c8fbc58389a283b8c7ef061f7c3e4c3b6d764230`
- Environment: `windows-worker`, exact Slint UI-MCP debug binary, synthetic empty application data

## Conditions

- Tile size: default 460 x 360 logical pixels.
- Synthetic text only; no real transcript, profile, endpoint, credential, or session title.
- Before/after captures must use the same text, theme, language, tile size, selection mode, and scroll position.
- Inspect separately: rich-view user bubble, whole-answer Select text mode, and mouse selection highlight.

## Baseline evidence

The exact baseline was built with `CARGO_INCREMENTAL=0`, `SLINT_EMIT_DEBUG_INFO=1`, `--locked`, and `--features ui-mcp`. F6 was dispatched through native Windows input in the active test desktop.

- `before-c8fbc583-rich-view.png` — the 460 x 360 tile in normal rich view. Its body viewport is 249 px high, while the short selectable text itself is only 28 px high.
- `before-c8fbc583-select-mode.png` — the same tile after `... -> Select text`. MCP geometry reports the whole-answer `TextEdit` as 436 x 249 px and its internal text-input viewport as 412 x 225 px, despite the same short wrapped text.

This reproduces the oversized selection surface and confirms `vertical-stretch: 1` on the whole-answer `TextEdit` as the affected path. The rich-view `SelectableText` and bounded capture editor are not the source of this baseline defect.

## Pending evidence

Matching candidate screenshots and geometry are required after the approved fix. macOS parity remains pending until `mac-worker` returns.
