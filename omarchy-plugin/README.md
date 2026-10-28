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
