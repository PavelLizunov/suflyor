# Upgrading

Suflyor has no migration tool and needs none: `config.json` is forward
compatible. A field added by a newer version takes its default value on the
next launch, and a field an older version does not know is ignored.

## Where your data lives

| Platform | Data directory |
|---|---|
| Windows | `%APPDATA%\suflyor\` |
| macOS | `~/Library/Application Support/suflyor/` |

It holds `config.json`, `sessions/`, `recordings/`, `catalog.sqlite` and
`overlay-host.log`. Installing and updating leave this directory untouched. The
Windows uninstaller asks whether to delete it together with the downloaded AI
models; answer No to keep your data.

Builds from before the rename kept their data in `%APPDATA%\overlay-mvp\`. The
app renames that directory to `suflyor` once, at startup. If the rename fails
(for example a file is held open by a second instance), the old directory keeps
being used and the rename is tried again on the next launch.

## Updating

**Windows.** Settings -> Updates -> "Check for updates", then "Update now". The
app downloads `suflyor-slint-setup.exe` from this repository's GitHub Releases
only, starts it and quits so the installer can replace the running binary. You
can also download the installer from
[Releases](https://github.com/PavelLizunov/suflyor/releases) and run it over
the installed version. The check sees stable releases only; a prerelease is
installed by hand.

**macOS.** Download the new DMG and replace the app in Applications. See
[docs/macos-install.md](docs/macos-install.md).

## Before you update

Settings -> "Export settings (incl. keys)..." writes the whole configuration to
a file you choose. That file contains your API keys: keep it private. "Import
settings..." restores it.

## Going back to an older version

Download the older installer from
[Releases](https://github.com/PavelLizunov/suflyor/releases) and run it. The
Windows installer is per-user, installs into `%LOCALAPPDATA%\suflyor-slint\` and
overwrites the files already there. Settings written by the newer version that
the older one does not know are ignored.

## Reporting a problem

Settings -> Diagnostics can copy a redacted report and collect a redacted log.
Open an issue at <https://github.com/PavelLizunov/suflyor/issues> and attach
them. Check that no API key or bearer token is in what you attach.

## Older notes

The per-version notes for v0.0.1 to v0.1.1 described the retired Tauri/React
build: its hotkeys, its MSI installer and its commands no longer exist. They
are kept in git:

```
git show 8a38baccc38ec317693be4dde376e958e8f3c242:UPGRADING.md
```
