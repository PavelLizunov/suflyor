# Suflyor v0.38.1-rc.7

This release candidate restores behaviour of v0.38.0 that the release candidates rc.3 to rc.6 broke. It changes no layout, colour or text of the interface.

## Withdrawn

- **The capsule bar of v0.38.1-rc.6.** The owner tested it in real use and rejected it: functions lost or moved, areas that could not be clicked, no drag handle, errors in the Russian text. The bar, its lock chip, auto-tiles controls, drag handle and tray chip are the ones of v0.38.1-rc.5. Settings written by rc.6 for the removed features are ignored.
- **Slint 1.18.1.** The interface library is 1.17.1 again, as in v0.38.0. With 1.18.1 the mouse wheel could not reach the end of Settings > AI providers: the last two switches were cut off. A machine without an OpenGL driver again needs `SLINT_BACKEND=winit-software` (ledger D-018).

## Fixes (behaviour of v0.38.0 restored)

- **Summary on the built-in local model** no longer hangs. It also blocked every later AI request.
- **The request sent to the model** is the one of v0.38.0 again: the instruction to give a short answer of at most 120 words was missing, and the wrapper of a typed question had changed, so copying a conversation included internal prompt text and a follow-up question could be answered on the old topic.
- **Deep lock** refuses a request at once. F3 answered after about three seconds.
- **Session journals**: a failed deletion of an old journal can no longer make the size limit delete recent sessions.
- **A start that is refused** because system audio is busy keeps the transcript of the previous session, so its summary can still be made.
- **Audio devices in Settings**: one device that cannot be read no longer hides the others.

## Kept from after v0.38.1-rc.5

- A request that cannot act says so: the archive shows its notices, and Shift+Alt+1 without a selection reports that nothing is selected (pull request #251).

## Known, not fixed here

- macOS, Settings > AI providers: the installed state and the buttons can belong to another local model than the selected one, and "Enable for text" changes a separately chosen Vision provider.
- Tray menu: a right click within a quarter of a second after the previous menu was requested is ignored.
- Auto-tiles miss a very short question that follows a statement in the same line (for example "... Как же?").

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
