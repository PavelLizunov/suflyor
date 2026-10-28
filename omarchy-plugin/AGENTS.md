# omarchy-plugin

The Omarchy Quattro plugin `io.github.pavellizunov.suflyor`: Suflyor's whole
interface in QML inside the running Omarchy shell. The logic runs in suflyord
(the overlay-host binary built with `--features remote-ui`). Design and
protocol: `docs/OMARCHY-PLUGIN-DESIGN.md`.

## How a view talks to suflyord

Every Slint window the host creates arrives as a `RemoteWindow` (`win`). A view
gets `win` and `service` (the plugin service) and never talks to the socket
itself.

| Need | Code |
| --- | --- |
| Read a property | `win.prop("kebab-name", fallback)` (re-evaluates on change) |
| Edit an `in-out` property | `service.client.set(win.winId, "kebab-name", value)` |
| Fire a callback | `service.client.call(win.winId, "callback-name", [args])` |
| A callback that returns a value | `service.client.call(win.winId, name, args, function (value, error) {…})` |
| Rust calls a UI function (`invoke_*`) | `Connections { target: service.client; function onInvoked(w, name, args) { if (w === win) … } }` |
| Visible text | `service.tr("English msgid", [args])` |

The property and callback names, directions and types are in
`tools/remote-ui/schema.json` (generated from the `.slint` files). Struct
fields use the Slint kebab-case names (`display-text`), enums their kebab
values (`"cyan"`), colors `#rrggbbaa`.

## Porting a Slint window

- Port from the `.slint` source, not from screenshots. Keep every state the
  window can show: each `if` branch, each conditional text, each disabled
  state. Logic inside the `.slint` file (bindings, `changed … =>` handlers,
  `init =>`, functions that fire callbacks) moves into the QML view.
- Keep the English text of every `@tr("…")` exactly, including punctuation and
  placeholders, so `locales/ru.json` (built from the Slint catalog) applies.
  A string the Slint window does not have goes into `locales/plugin.ru.json`
  with its Russian text.
- Look like Omarchy, not like the Slint theme: colors from `qs.Commons`
  (`Commons.Color.foreground`, `.background`, `.accent`, `.urgent`, `.muted`,
  `.menu.*`, `.bar.*`), spacing `Style.space(px)` and `Style.spacing.*`, type
  `Style.font.*`, motion `Style.duration(ms)` respecting `Style.reduceMotion`.
  Reuse `qs.Ui` controls (`Button`, `TextField`, `Toggle`, `Dropdown`,
  `PanelSectionHeader`, `PanelSeparator`) where one fits.
- Reuse Suflyor's original head/penguin brand and action SVGs through `icons/`.
  Native Nerd Font glyphs in `Style.font.family` remain the fallback for actions
  without a matching product asset. Icons supplement localized labels.
- A view starts no process, reads no file and holds no timer that polls.
  Anything that needs the system goes through a callback to suflyord.
- No symlinks in this directory (Omarchy rejects plugin trees with links).

## Checks

- `qmllint` against the shell imports: map `qs` to `/usr/share/omarchy/shell`
  (a directory containing a `qs` link is enough) and add `/usr/lib/qt6/qml`.
  Warnings about members of the dynamic shell objects (`bar.shell`,
  `Style.font.*` on `QObject`) are expected; every other warning is fixed.
- `python3 tools/remote-ui/po2json.py --check` after a `.po` change.
- Live checks run in an Omarchy session through the existing shell; never
  start a second Quickshell.
