#!/usr/bin/env bash
# Install Suflyor as the Omarchy plugin io.github.pavellizunov.suflyor from a
# prebuilt `--features remote-ui` overlay-host (docs/OMARCHY-PLUGIN-DESIGN.md).
# User data, configuration, models and the Omarchy bar placement are retained.
#
#   scripts/install-omarchy-plugin.sh [install|uninstall]
#
# SUFLYOR_BUILD_DIR: directory with the remote-ui overlay-host and, when built,
# suflyor-tts / suflyor-teratts (default: slint-experiment/target/debug).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
action="${1:-install}"
id="io.github.pavellizunov.suflyor"
plugins="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/plugins"
plugin_dir="$plugins/$id"
prefix="${SUFLYOR_PLUGIN_PREFIX:-$HOME/.local/lib/suflyor-omarchy}"
build="${SUFLYOR_BUILD_DIR:-$root/slint-experiment/target/debug}"

if [[ "$action" = uninstall ]]; then
    # The shell's plugin registry notices the removed directory; suflyord exits
    # with its socket gone from the next shell start.
    pkill -x suflyord 2>/dev/null || true
    rm -rf "$plugin_dir"
    for name in suflyord suflyor-tts suflyor-teratts; do rm -f "$prefix/$name"; done
    rmdir "$prefix" 2>/dev/null || true
    printf 'Removed the Suflyor Omarchy plugin; configuration and models retained.\n'
    exit
fi
[[ "$action" = install ]] || { printf 'Usage: %s [install|uninstall]\n' "$0" >&2; exit 2; }

if [[ "$(uname -m)" = x86_64 ]] && ! grep -qw avx2 /proc/cpuinfo; then
    printf 'Suflyor Linux ONNX speech runtime requires an AVX2-capable CPU.\n' >&2
    exit 1
fi
command -v omarchy >/dev/null || { printf 'Omarchy is not installed: the plugin needs the Omarchy shell.\n' >&2; exit 1; }
for tool in pactl parec paplay grim slurp wl-copy wl-paste hyprctl xdg-open tesseract; do
    command -v "$tool" >/dev/null || { printf 'Missing required native tool: %s\n' "$tool" >&2; exit 1; }
done
[[ -x "$build/overlay-host" ]] || {
    printf 'No overlay-host in %s. Build it with:\n  cargo build --no-default-features --features remote-ui --manifest-path slint-experiment/Cargo.toml --bin overlay-host\n' "$build" >&2
    exit 1
}
# The plugin build must not draw its own windows: check it serves the shell socket.
grep -qa 'remote-ui platform is not installed' "$build/overlay-host" || {
    printf '%s/overlay-host is not a --features remote-ui build.\n' "$build" >&2
    exit 1
}

mkdir -p "$prefix"
# Optional verified CUDA speech libraries remain outside the shell plugin tree.
if [[ -d "$build/speech-gpu" ]]; then
    cp -R "$build/speech-gpu" "$prefix/.speech-gpu.new"
    rm -rf "$prefix/speech-gpu"
    mv "$prefix/.speech-gpu.new" "$prefix/speech-gpu"
fi
if [[ -d "$build/nemotron" ]]; then
    rm -rf "$prefix/.nemotron.new"
    cp -R "$build/nemotron" "$prefix/.nemotron.new"
    rm -rf "$prefix/nemotron"
    mv "$prefix/.nemotron.new" "$prefix/nemotron"
fi
install -m 0755 "$build/overlay-host" "$prefix/suflyord.new"
mv -f "$prefix/suflyord.new" "$prefix/suflyord"
for name in suflyor-tts suflyor-teratts; do
    if [[ -x "$build/$name" ]]; then
        install -m 0755 "$build/$name" "$prefix/$name.new"
        mv -f "$prefix/$name.new" "$prefix/$name"
    fi
done

# Omarchy rejects plugin trees with symlinks, so the plugin is a plain copy and
# bin/suflyord is a launcher script, not a link to the prefix.
staging="$plugins/.$id.new"
rm -rf "$staging"
mkdir -p "$plugins"
# A fresh source URL avoids Qt retaining old nested types in the shared shell.
# The full source tree is versioned together, while the stable root owns assets.
ui_revision="$(find "$root/omarchy-plugin" -type f -name '*.qml' -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -c1-16)"
ui_name="ui-$ui_revision"
cp -R "$root/omarchy-plugin" "$staging"
mkdir -p "$staging/$ui_name"
cp -R "$root/omarchy-plugin/." "$staging/$ui_name/"
rm -f "$staging/$ui_name/AGENTS.md"
python3 - "$staging/manifest.json" "$ui_name" <<'PY'
import json, sys
from pathlib import Path
p = Path(sys.argv[1])
m = json.loads(p.read_text())
m["entryPoints"] = {key: f"{sys.argv[2]}/{value}" for key, value in m["entryPoints"].items()}
p.write_text(json.dumps(m, indent=2) + "\n")
PY
# Service resolves bin and locale files relative to its versioned source.
rm -f "$staging/AGENTS.md"
mkdir -p "$staging/bin"
cat > "$staging/bin/suflyord" <<EOF
#!/bin/sh
exec "$prefix/suflyord" "\$@"
EOF
chmod 0755 "$staging/bin/suflyord"
cp -R "$staging/bin" "$staging/$ui_name/bin"
if find "$staging" -type l | grep -q .; then
    printf 'Refusing to install: the plugin tree contains a symlink.\n' >&2
    rm -rf "$staging"
    exit 1
fi
rm -rf "$plugin_dir"
mv "$staging" "$plugin_dir"

# Restart only after replacing the artifacts. Disabling the plugin would delete
# its bar entry (including its section, index and settings); the native reload
# keeps that entry and lets the existing service start the newly installed host.
if "$prefix/suflyord" --command quit 2>/dev/null; then
    printf 'Requested restart of the existing Suflyor daemon.\n'
fi
if ! omarchy-shell shell rescanPlugins >/dev/null 2>&1; then
    printf 'Shell reload unavailable; the new plugin loads on the next shell start.\n' >&2
fi
printf 'Installed %s in %s and suflyord in %s; existing bar placement retained.\n' "$id" "$plugin_dir" "$prefix"
printf 'For a first install, enable it with: omarchy bar put %s\n' "$id"
