# Suflyor for Omarchy development

The working source is the repository's `omarchy-plugin/` directory. The installed
copy lives in `~/.config/omarchy/plugins/io.github.pavellizunov.suflyor/`.

Build the headless host, then install with the repository's existing installer:

```sh
cargo build --locked --no-default-features --features remote-ui \
  --manifest-path slint-experiment/Cargo.toml --bin overlay-host
scripts/install-omarchy-plugin.sh
omarchy-shell shell rescanPlugins
omarchy plugin enable io.github.pavellizunov.suflyor
```

The two speech sidecars are separate builds. Set `SUFLYOR_BUILD_DIR` to a folder
containing `overlay-host`, `suflyor-tts` and `suflyor-teratts` when installing all
three. Stop the running Suflyor daemon before replacing it; a new shell connection
alone does not replace a running binary. The plugin service is deliberately not
`keepLoaded`: Omarchy's plugin reload must replace its QML, not retain old code.

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
