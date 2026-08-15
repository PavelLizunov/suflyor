# RC15 math, player, and tray UI audit

## Fixed conditions

- Host: Winbrat test VM (never the owner workstation)
- Renderer: `winit-software`
- Slint MCP: feature-enabled debug build, debug info emitted during UI compilation
- MCP endpoint: loopback port 9123 on Winbrat
- Theme/language: dark theme, English and Russian
- Tile state: 800 x 600, identical synthetic matrix Markdown and active read-aloud state
- Tray state: production overlay hidden, themed tray menu opened through the real Win32 notification callback

The synthetic tile harness uses the production `TileWindow` and Markdown parser. Tray and hotkey checks use the production `overlay-host` built from the same staged product tree. The evidence contains no keys, URLs, user paths, real session titles, or transcript text.

## Evidence

| Surface | Before | After |
| --- | --- | --- |
| Matrix Markdown and active player | `before-math-player.png` | `after-math-player.png` |
| Russian player geometry and formula rendering | — | `after-math-player-ru.png` |
| Themed tray restore menu | `before-tray-menu.png` | `after-tray-menu.png` |

## Results

- TeX `\[...\]` and `\(...\)` delimiters are normalized before Markdown parsing, every `pmatrix` in a fragment is handled, and code spans/fences remain literal. The final captures contain no raw `\begin`, `\end`, `\sum`, or accidental oversized heading.
- The 60 px player has 8 px outer insets, a separate status row, primary `-10 / play-pause / +15` transport, and secondary stop/speed controls. English and Russian layouts have the same unclipped geometry.
- The production tray menu is exactly 196 x 136 at (827, 556) on the 1024 x 768 QA desktop. Its bottom is at 692, above the work-area bottom at 728; the window is themed, frameless, topmost, and excluded from the taskbar.
- The legacy `WM_RBUTTONUP` plus v4 `WM_CONTEXTMENU` pair produces exactly one menu opening. Ten consecutive hide -> menu -> left-click restore cycles produced ten openings and ten restores. Restore closed the popup first, removed the temporary icon, and never left a stale action capable of hiding the visible bar.
- The complete global shortcut smoke dispatched `F1`, `F3`, `F4`, `F6`, `F7`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`, `Shift+F9`, `Shift+Alt+1`, `Shift+Alt+2`, and `Shift+Alt+3`. Capture surfaces were cancelled with Escape; `Shift+Alt+1` copied 31 characters from a disposable selected-text window.
- Winbrat has no default system-audio endpoint, so device-dependent capture/audio output remains an external VM limitation rather than a verified route.
