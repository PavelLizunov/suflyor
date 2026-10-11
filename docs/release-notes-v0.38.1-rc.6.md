# Suflyor v0.38.1-rc.6

This release candidate replaces the overlay bar. The bar is rebuilt in capsules around the mascot, everything that is not needed during a call moved into one menu, and the bar now fits itself to the screen. No function was removed; some are reached by a different route, listed below.

## The bar

- **Layout.** The mascot is a circle spanning both rows; its ring shows the state (grey idle, green recording, amber paused or muted, red failure) and it is the handle for dragging the bar. Controls are grouped in capsules: state, voice, session, tiles, window. The second row shows the session name and the last line with the speaker's word ("you:" / "them:"), or a quiet "listening…" while a session runs without text.
- **Stealth is always visible**, in every form of the bar, with a word in the full bar ("hidden" / "visible"). A stealth failure is red throughout and blinks.
- **One auto-tiles switch** replaces the lock chip and the flame: silent, on questions, on every line. The unloaded local AI is shown separately as a red "AI off" in the second row, and is loaded or unloaded from the menu.
- **Three forms chosen by the screen**: full (874 px), compact (586 px) and a new mini form (408 px). The bar takes the largest form that fits the monitor it is on, never larger than the one you chose, and re-checks when the resolution, the scale or the monitor changes. It returns to a visible monitor if it ends up off-screen.
- **The bar remembers where you put it**, per monitor, after you drag it.
- **Sizes are declared in one place** (`ui/bar_spec.slint`): every region has a fixed width and the bar's width is their sum. The full bar is 406 px narrower than before.

## The "more" menu

- The **⋯** chip opens a menu with what left the bar: screenshot question, restore a closed tile, session archive, meeting summary, knowledge base, settings, help, hide to tray, load or unload the local AI, restart, quit. Restart and quit ask inside their own row.
- **Sound**: microphone and system audio can be switched from the menu, so they are reachable in the compact and mini forms too.
- **Pinning.** Any action except restart, quit and the AI load can be pinned to the bar's second row (up to eight) and unpinned from the same menu. With nothing pinned the bar shows a "pin" hint that opens the menu.
- Hotkeys are shown next to their actions. They are unchanged.

## Where things went

| Was on the bar | Now |
|---|---|
| Stealth, microphone, system audio, ask, grab, start/stop, pause, new tile, write a question, compact | On the bar |
| Lock chip (3 modes) and "aggressive" flame | The auto-tiles switch; unloading the AI is in the menu |
| Close all tiles | A small × next to the tile counter, shown while tiles are open |
| Screenshot, archive, summary, settings, help, tray, restart, quit | The ⋯ menu; pin the ones you use |
| Model and speed readout, "AI streaming" mark | Removed from the bar: the mascot turns into the penguin while the AI answers |

## Fixes found on the way

- Small text meets the contrast norm in the places an audit found below it (the pin hint, the confirmation button, the label of a recording button).
- Pinned icons and menu rows respond to hover and press; a failed pin save is shown in the menu.
- The auto-tiles menu is placed inside the work area and can no longer fail on a very narrow screen.
- Progress and acknowledgement statuses ("starting model…", "sys ok") are shown in the second row.

## Not verified on the test machine — please check these first

The test machine has no audio device and no local model, and runs at 100 % scale. These were checked by tests and by reading the code, not by eye:

- a running session with real audio: the green ring, the session name, the lines with "you:" / "them:", "listening…", pause;
- holding **ask** / **grab**, the swipe-up lock, and the red recording state;
- the red blinking stealth failure, and the red "AI off" with loading and unloading the local model;
- Windows scale above 100 % (125–200 %), a second monitor, and the mini form at its true size;
- macOS (compiled in CI only).

## For contributors

- New tests: `overlay_bar_geometry` (order, separation and hit-testing of every control in all three forms and both languages), `bar_spec_guard`, `bar_menu_guard`, `bar_menu_interaction`, `bar_position_guard`, `bar_slop_guard`; they are in the UI-only gate lists.
- `docs/DEFECT-LEDGER.md`: D-034 to D-050, defects of the tile, the transcript and the archive found by code maps; none is fixed in this release.
- In Slint, a child with an explicit size inside a non-layout parent is centred unless `x`/`y` are set. This hid the state capsule for part of this work; the geometry test now fails on it.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
