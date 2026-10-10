# Suflyor v0.38.1-rc.5

This release candidate starts the design work: colours of state come from the theme, sizes come from the design tokens, and the build now checks both. It also fixes what the first live check of answer tiles found.

## Interface

- **State pill of the bar**: its colour follows the colour scheme. In the light scheme the state word was close to unreadable (1.6:1); it now keeps 4.5:1 in all four schemes.
- **Tile badges and the "AI streaming" mark**: the same rule; in the light scheme they were at 2.2:1.
- **Question window, setup wizard, help, session recovery**: fields and buttons follow the colour scheme. The question window showed a white field and the wizard showed light grey buttons on a dark window.
- **Russian interface**: the tile title, badge, state label, placeholder and error text, the row buttons of the archive, and the title of a re-ask tile are translated.
- **Tile labels**: a finished answer says "done" without "(stop)"; a notice is labelled as a notice, not as an error.
- **Sizes**: 342 places that repeated a token's value now use the token. The render is unchanged: 56 captures before and after are identical pixel for pixel.

## Fixes

- **F3 (re-ask)** works after an answer to F9 or to a typed question. Before, it had nothing to re-ask and did nothing. With nothing to re-ask it now says so in a tile.
- **A server that never answers** is reported as a timeout, not as a refused connection.
- **Silent uninstall** (`uninstall.exe /S`) no longer waits on the "delete your data too?" question, and keeps the data. In the interactive uninstall the default button is now "No".

## For contributors

- `slint-experiment/tests/design_guard.rs`: a window that uses std-widgets must take their palette from the theme; the host must not build colours in Rust; the number of size and colour literals that bypass the tokens may only go down.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
