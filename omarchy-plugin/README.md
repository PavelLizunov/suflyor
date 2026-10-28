# Suflyor for Omarchy development

The working source is the repository's `omarchy-plugin/` directory. The installed
copy lives in `~/.config/omarchy/plugins/io.github.pavellizunov.suflyor/`.

Build the headless host, then install with the repository's existing installer:

```sh
cargo build --locked --no-default-features --features remote-ui \
  --manifest-path slint-experiment/Cargo.toml --bin overlay-host
scripts/install-omarchy-plugin.sh
# First install only; leaves an existing widget where the user placed it.
omarchy bar put io.github.pavellizunov.suflyor
```

The two speech sidecars are separate builds. Set `SUFLYOR_BUILD_DIR` to a folder
containing `overlay-host`, `suflyor-tts` and `suflyor-teratts` when installing all
three. The installer replaces the artifacts, asks the running daemon to quit and
reloads plugin code through the existing shell. The service starts the new binary.
The widget remains in Omarchy's saved bar section and index, with its settings.
For updates, run the installer without disabling/re-enabling the plugin: disabling
removes its bar entry, so a later enable uses the manifest's default position.
The plugin service is deliberately not `keepLoaded`: Omarchy's plugin reload
must replace its QML, not retain old code.

## Capture hiding

Stealth applies a named Hyprland rule to Suflyor card-sized surfaces and a
separate visual-only bar mark. Other Omarchy widgets stay outside the rule.
Hyprland uses full namespace matches; the rule is recreated after its configuration
reloads, and rule application is not reported as a successful capture test.

Stock Hyprland 0.56 masks protected surface bounds with **black rectangles**,
including transparent pixels. Settings, archive and the bar popup therefore use
bounded surfaces, with input-only dismissal layers outside the rule.

An optional [Hyprland 0.56.2 clean-capture build](../scripts/hyprland-capture/README.md)
adapts upstream PR #16061 to render monitor/region capture without protected
windows, layers or fadeouts, leaving the local output unchanged. It installs
separately and is selected for the next uwsm login; installing it does not change
the running compositor. Mirrored outputs still use upstream black masking.
Neither rule application nor a successful compositor build proves actual capture
behavior. Other capture paths may ignore exclusion. Suflyor also temporarily
conceals its surfaces for its own screenshots; check the intended capture
application before relying on either route.

## Hold-to-ask shortcuts

Hold **Super+F9** to record a microphone question, or **Super+Shift+F9** to
record system audio. Release the shortcut to transcribe and ask using the current
providers. These use the existing bar push-to-talk callbacks and 30-second
lost-release watchdog; they do not start a session or a model. Super is the
Windows key. The shortcuts are listed in Settings > Hotkeys and Help (F1).

Hyprland bindings owned by another application are left alone. In particular,
Omarchy's bare F9 dictation may keep that key, making Suflyor's bare F9 ask
unavailable; Settings > Diagnostics reports conflicts. The two Super chords are
separate from bare F9 and require both press and release bindings. Registration
is not evidence of a completed recording/transcription/answer.

## Web lookup

The local model uses the existing read-only `web_search` (DuckDuckGo Lite) and
`open_url` page-text fetcher when both agent tools and web search are enabled.
Fast mode no longer overrides this opt-in: manual/PTT/follow-up and automatic
answers may use at most two web calls within a 15-second lookup budget, then
produce the same short streamed answer without reasoning. Model generation is
not included in that lookup budget. With web off, fast answers still use one
plain request. Definitions retain their tool-free path.

The model chooses whether a lookup is needed; ordinary knowledge questions are
not forcibly sent to search. Explicit search/URL requests are described in its
system guidance. The existing public-address, redirect and DNS restrictions
remain in place. Page fetches do not start a browser or JavaScript. Search and
page text are bounded; a captcha, timeout or server without tool support is
not a successful web lookup. Automatic answers with web enabled bypass the
ten-minute answer cache, and lookup progress reaches the tile. No live search,
model tool-call quality or latency acceptance is implied by installation.

## Model speed

Settings > AI providers > Model speed measures the already loaded local model.
Choose a short, medium or long synthetic input, then Measure model speed. It makes
one streamed request with at most 128 output tokens and a three-minute limit,
without tools, reasoning, conversation data, engine startup or configuration changes. End
a live session first; a deep-locked managed model remains locked.

Prefill and decode use this request's engine timings; unsupported metrics remain
unavailable instead of being inferred from total HTTP time. The result includes
actual input/output/cache token counts, first-token latency, engine version when
known and the saved context preset. The last sample is kept locally in
`model-speed.json` for that endpoint, model, context and engine version. It is an
observed sample, not a cold-load benchmark or a guarantee under other GPU loads.
First-token and total time may include server queuing or model loading.
Hiding Settings keeps the measurement running; Cancel stops it. Closing Settings
also leaves engine updating and model downloading running in their own workers.

## Model disk

Settings > AI providers > Model storage accepts an existing absolute directory
on a mounted disk, by typing the path or choosing a folder. Saving only records
the choice; restart Suflyor to apply it. No existing files are moved or deleted.
The machine-local choice is `models-dir` beside Suflyor's `config.json`; it is
not part of portable profile exports. An empty choice retains the old locations.

Under a selected directory, local AI uses `llama.cpp/`, speech recognition uses
`gigaam-v3/` and `whisper.cpp/`, voices use `tts/`, and speaker models use `diar/`.
An explicitly configured speech-recognition model path still takes precedence.
Both speech sidecars receive the same voice directory from the host.

For existing Strata weights, select the directory containing `Strata-data/`.
Strata setup receives that data directory and can reuse its model files. A model
directory is not a runnable engine installation: the separate Strata folder
control expects a Linux installation with `.venv/bin/python`, `serve/server.py`
and `strata-<model>.json`. Its adjacent path field avoids a native picker entirely.
A separately running Strata server can instead be selected with Search again or
Connect by address.

Native file and folder pickers run asynchronously. While one is open, the host
asks the plugin to hide its layer-shell surfaces so an ordinary portal window
is not covered by them. Choosing or cancelling restores the surfaces. A second
picker is ignored until the first finishes.
