# Suflyor v0.38.1-rc.7

This release candidate takes the bar back to the one in v0.38.1-rc.5. The capsule bar of v0.38.1-rc.6 is withdrawn.

## Why

The owner tested v0.38.1-rc.6 in real use and rejected it: some functions stopped working or moved to places where they could not be found, the bar showed empty areas that could not be clicked, the handle for dragging the bar was gone, the Russian text had errors, and the bar no longer matched the menus and the tiles. The automated checks had passed; they did not show any of this.

## What changed against v0.38.1-rc.6

- **Bar**: the layout, the lock chip, the auto-tiles controls, the drag handle and the tray chip are the ones of v0.38.1-rc.5.
- **Removed**: the "more" menu with pinning, the three bar forms chosen by monitor width, the single auto-tiles switch, and the memory of the bar position.
- **Settings file**: values written by v0.38.1-rc.6 for the removed features are ignored. Nothing has to be deleted by hand.

## Kept from after v0.38.1-rc.5

- A request that cannot act says so: the archive shows its notices, and Shift+Alt+1 without a selection reports that nothing is selected (pull request #251).

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
