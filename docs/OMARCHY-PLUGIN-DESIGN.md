# Suflyor as an Omarchy Quattro plugin

Owner decision, 2026-10-25: on Omarchy, Suflyor becomes a Quattro plugin with
its whole interface in QML inside the running Omarchy shell, in the Omarchy
theme. The bar, tiles, Settings, archive, transcript, palette, help and setup
wizard all move. Windows and macOS keep the Slint interface. This document is
the plan for branch `claude/omarchy-plugin`, which starts from
`codex/omarchy-port` at 56e86b2.

## 1. Shape

```
Omarchy shell (one Quickshell process)          suflyord (Rust, user process)
┌───────────────────────────────────┐            ┌────────────────────────────────┐
│ io.github.pavellizunov.suflyor    │  JSON lines │ host logic from overlay_host   │
│  Service.qml ── Process ──────────┼──spawns───▶│ (unchanged) compiled against   │
│      └─ Socket ◀──────────────────┼────────────┤ generated remote windows       │
│  BarWidget.qml  (session, chips)  │            │ overlay-backend: audio, STT,   │
│  Tile layers    (answers)         │            │ AI, journal, KB, OCR, TTS      │
│  Panel.qml      (Settings, …)     │            │ sidecars suflyor-tts/-teratts  │
└───────────────────────────────────┘            └────────────────────────────────┘
        ▲ bindings.lua marker block: SUPER/F-keys → `suflyorctl <action>` → daemon
```

This is the pattern of the owner's telebar plugin: `TelebarService.qml` starts
`omagramd --with-parent` with `Process`, restarts it after an exit, and talks
newline-delimited JSON over a Quickshell `Socket`. Keybindings follow teratts
`bin/speak-bindings`: one owned marker block in `bindings.lua`, backed up and
rolled back when Hyprland rejects it.

## 2. Why the host logic stays

`src/bin/overlay_host*` is about 29,000 lines of tested behaviour: sessions,
tiles, the queue, Settings controllers, archive, local AI and diagnostics.
It uses the generated Slint API directly. Counted at 56e86b2: 1408 `set_*`,
246 `get_*`, 331 `on_*` and 45 `invoke_*` calls, 224 model uses, 134
`invoke_from_event_loop`, 134 `.window()` calls. Rewriting it for QML would
repeat the macOS port history and lose the guard tests.

Instead the daemon builds those modules against **remote windows**: Rust types
with the same names and methods as the Slint-generated ones (`TileWindow::new`,
`set_blocks`, `on_regenerate_clicked`, `as_weak`, `window().show()`). A remote
window stores its properties, sends every change to the shell and runs the
registered callback when the shell reports a click.

### Generated remote windows

- `slint-experiment/build.rs` already compiles `ui/index.slint`. For the daemon
  build, a generator walks the same files with `i-slint-compiler` 1.17 (present
  in the lockfile through `slint-build`). For each exported `Window` and global it
  emits: properties with types and `in`/`out`/`in-out` direction, callbacks with
  argument and return types, and the structs and enums they use.
- Output is a Rust module `remote_ui` with `set_x`, `get_x`, `on_x` and `invoke_x`.
  Properties of struct, enum and model type serialize with serde under their
  Slint (kebab-case) names, which QML uses as JSON keys.
- `slint` core stays a dependency for `SharedString`, `ModelRc`, `VecModel`,
  `Timer` and the event loop. It runs on a headless platform: no renderer, an
  event loop driven by a channel, `invoke_from_event_loop` intact. The
  `ui_shots` scratch harness proved that Slint runs under a custom platform on
  a GitHub runner.
- Models can be changed in place after `set_*`. The remote window keeps the
  last serialized value of each model property and diffs it once per event-loop
  turn, so in-place `VecModel` edits reach the shell without internal Slint APIs.
- `.window()` returns a remote window handle: `show`, `hide`, `is_visible`,
  `set_size`, `set_position`, `scale_factor`. Placement becomes a request the
  QML side resolves against the Omarchy screen. The Hyprland `hyprctl`
  placement in `native/linux/window.rs` does not run in this build.
- Logic that lives inside `.slint` files (bindings, `changed` handlers that fire
  callbacks, `@tr` texts, conditional copy) moves to QML. An audit lists every
  such handler per window before its QML version is written.

### Protocol

Socket `$XDG_RUNTIME_DIR/suflyor/shell.sock`, mode 0600, one JSON object per line.

| Direction | Message | Meaning |
| --- | --- | --- |
| daemon → shell | `{"t":"open","win":"tile-42","type":"TileWindow","props":{…}}` | A window was created; full state |
| daemon → shell | `{"t":"prop","win":"tile-42","name":"blocks","value":[…]}` | One property changed |
| daemon → shell | `{"t":"show"/"hide"/"close","win":"tile-42"}` | Visibility and lifetime |
| daemon → shell | `{"t":"invoke","win":"settings","name":"focus-search","args":[]}` | Rust called a UI function |
| shell → daemon | `{"t":"set","win":"settings","name":"profile-name-input","value":"x"}` | Two-way property edited in QML |
| shell → daemon | `{"t":"call","win":"tile-42","name":"regenerate-clicked","args":[],"id":7}` | Callback; replies `{"t":"ret","id":7,"value":…}` when it returns a value |
| shell → daemon | `{"t":"hello","shell":"omarchy","locale":"ru","theme":{…}}` | First message; daemon replies with every open window |

A shell restart reconnects and receives the full state again. The daemon keeps
running without a shell; sessions and recordings are not tied to the shell.

## 3. Shell side

- **Plugin** `io.github.pavellizunov.suflyor`, kinds `service`, `bar-widget`,
  `panel`, `keepLoaded: true`. The service owns the daemon process, the socket
  and the window registry and is the single source for every view.
- **Bar widget** replaces the Slint overlay bar on Omarchy: session timer and
  stop, microphone and system chips, ask, vision, stealth, status line. It uses
  `qs.Commons` `Color` and `Style` and follows the theme live.
- **Tiles** are layer-shell `PanelWindow`s created by the service, namespace
  `suflyor-tile`, keyboard focus on demand for follow-up input. Drag and pin
  change layer margins; the screen comes from the placement request.
- **Panel** hosts Settings with all 17 tabs, the archive, transcript, knowledge
  palette, help and the setup wizard, in the VPNRouter and Omarchy panel style.
- **Stealth.** Hyprland 0.56.2 lists `no_screen_share` among layer-rule effects
  (`LayerRuleEffectContainer.cpp`). The plugin owns a layer rule for namespaces
  `suflyor-*`, applied when stealth is on, so screen share and screencopy skip
  Suflyor surfaces. This is confirmed in Hyprland source only and is the first
  live check.
- **Text.** QML keeps the English msgids. A generated `ru.json` built from
  `slint-replay.po` gives the Russian strings, so `i18n_guard` keeps covering
  both front ends.
- **Hotkeys.** The 18 shortcuts the port already registers move into a
  `-- >>> suflyor bindings` block that runs `suflyorctl <action>` over the
  existing private command socket.

## 4. Stages

Each stage ends with source review, CI on all three systems, and an Omarchy
check on VM 107 when it is free.

| Stage | Result | How it is checked |
| --- | --- | --- |
| 0 | Generator for one small window (`TextAskWindow`), headless event loop, socket, a minimal plugin that shows that window's state, stealth layer rule | Rust unit tests in CI; live layer rule and screencopy on VM 107 |
| 1 | `suflyord` with the whole host compiled against remote windows; bar widget; tiles; hotkeys; menu entry | Existing host tests pass unchanged; live session, ask, vision and tiles |
| 2 | Settings, all 17 tabs, every callback | Tab-by-tab EN/RU captures against the Slint reference, callback audit |
| 3 | Archive, transcript and player, palette, help, wizard, recovery offer | Same |
| 4 | Plugin repository, release artifact with `suflyord`, sidecars and models on demand, one-command install, removal that keeps user data | Omarchy plugin validate, qmllint, fresh install and lifecycle on VM 107 |

Gpt-sol workers take bounded pieces (generator types, one QML view per brief,
`.slint` handler audits). I review each diff against the source and run the
gates.

## 5. Open points

- Callbacks with return values used synchronously by Rust (`invoke_*` that
  return data). The generator lists them; each gets a QML-side equivalent or a
  Rust-side replacement.
- Region selection keeps `slurp`; it is outside the shell, as now.
- The full-screen Vision capture must not contain Suflyor surfaces. With the
  layer rule on, screencopy skips them. Without stealth the existing
  hide-before-capture order applies.
- The macOS and Windows builds must not change. The remote-window build is a
  separate cargo feature and binary, checked by the existing guard suites.
