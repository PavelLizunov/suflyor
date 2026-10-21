#!/usr/bin/env bash
# Install a prebuilt native candidate; user data and models are never removed.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
action="${1:-install}"
prefix="${SUFLYOR_PREFIX:-$HOME/.local/lib/suflyor}"
bin_dir="${SUFLYOR_BIN_DIR:-$HOME/.local/bin}"
config="${XDG_CONFIG_HOME:-$HOME/.config}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
source_dir="${SUFLYOR_BUILD_DIR:-${CARGO_TARGET_DIR:-$root/overlay-backend/target}/debug}"
hypr_config="${SUFLYOR_HYPR_CONFIG:-$HOME/.config/hypr/hyprland.lua}"
if [[ -z "${SUFLYOR_BUILD_DIR:-}" && ! -x "$source_dir/overlay-host" && -x "$root/slint-experiment/target/debug/overlay-host" ]]; then
    source_dir="$root/slint-experiment/target/debug"
fi
if [[ "$action" = uninstall ]]; then
    systemctl --user disable --now suflyor.service 2>/dev/null || true
    rm -f "$config/systemd/user/suflyor.service" "$data/applications/suflyor.desktop" "$data/icons/hicolor/256x256/apps/suflyor.png"
    python3 - "$hypr_config" <<'PY'
from pathlib import Path
import sys
p=Path(sys.argv[1])
if p.is_file():
 s=p.read_text();start=s.find("-- BEGIN SUFLYOR NATIVE\n");end=s.find("-- END SUFLYOR NATIVE\n",start)
 if start>=0 and end>=0:p.write_text(s[:start]+s[end+len("-- END SUFLYOR NATIVE\n"):])
PY
    rm -f "$prefix/suflyor-hyprland.lua" "$prefix/launch"
    command -v hyprctl >/dev/null && hyprctl reload >/dev/null || true
    command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" || true
    [[ ! -L "$bin_dir/suflyor" || "$(readlink "$bin_dir/suflyor")" != "$prefix/overlay-host" ]] || rm "$bin_dir/suflyor"
    for name in overlay-host suflyor-tts suflyor-teratts; do rm -f "$prefix/$name"; done
    systemctl --user daemon-reload
    printf 'Removed Suflyor executables and launcher; configuration and models retained.\n'
    exit
fi
[[ "$action" = install || "$action" = autostart || "$action" = no-autostart ]] || { printf 'Usage: %s [install|autostart|no-autostart|uninstall]\n' "$0" >&2; exit 2; }
if [[ "$action" != install ]]; then
    if [[ "$action" = autostart ]]; then systemctl --user enable suflyor.service; else systemctl --user disable suflyor.service; fi
    exit
fi
if [[ "$(uname -m)" = x86_64 ]] && ! grep -qw avx2 /proc/cpuinfo; then
    printf 'Suflyor Linux ONNX speech runtime requires an AVX2-capable CPU.\n' >&2
    exit 1
fi
for tool in pactl parec paplay grim slurp wl-copy wl-paste hyprctl xdg-open tesseract python3; do
    command -v "$tool" >/dev/null || { printf 'Missing required native tool: %s\n' "$tool" >&2; exit 1; }
done
[[ -f "$hypr_config" ]] || { printf 'Missing Hyprland Lua config: %s\n' "$hypr_config" >&2; exit 1; }
for name in overlay-host suflyor-tts suflyor-teratts; do
    [[ -x "$source_dir/$name" ]] || { printf 'Missing native executable: %s\n' "$source_dir/$name" >&2; exit 1; }
done
mkdir -p "$prefix" "$bin_dir" "$config/systemd/user" "$data/applications" "$data/icons/hicolor/256x256/apps"
install -m644 "$root/scripts/suflyor-hyprland.lua" "$prefix/suflyor-hyprland.lua"
install -m644 "$root/slint-experiment/assets/icon.png" "$data/icons/hicolor/256x256/apps/suflyor.png"
for name in overlay-host suflyor-tts suflyor-teratts; do
    install -m755 "$source_dir/$name" "$prefix/.$name.next"
    mv "$prefix/.$name.next" "$prefix/$name"
done
ln -sfn "$prefix/overlay-host" "$bin_dir/suflyor"
python3 - "$prefix" "$config" "$data" "$hypr_config" <<'PY'
import pathlib,sys,shlex,json
prefix,config,data,hypr=map(pathlib.Path,sys.argv[1:])
launch=prefix/"launch"
launch.write_text("#!/bin/sh\nexport XDG_CONFIG_HOME="+shlex.quote(str(config))+"\n"+shlex.quote(str(prefix/"overlay-host"))+" --command show 2>/dev/null && exit 0\nexec systemctl --user start suflyor.service\n")
launch.chmod(0o755)
if not hypr.is_file():raise SystemExit("Hyprland Lua configuration is unavailable")
s=hypr.read_text()
if "-- BEGIN SUFLYOR NATIVE\n" not in s:
 hypr.write_text(s+"\n-- BEGIN SUFLYOR NATIVE\ndofile("+json.dumps(str(prefix/"suflyor-hyprland.lua"))+ ")\n-- END SUFLYOR NATIVE\n")
def unit(value): return str(value).replace('\\','\\\\').replace('"','\\"').replace('%','%%')
(config/'systemd/user/suflyor.service').write_text('''[Unit]
Description=Suflyor native Wayland assistant
After=graphical-session.target pipewire-pulse.service
PartOf=graphical-session.target
[Service]
Type=exec
ExecStart="%s"
Environment=SLINT_BACKEND=winit-software
Environment=WINIT_UNIX_BACKEND=wayland
Environment="XDG_CONFIG_HOME=%s"
Restart=on-failure
RestartSec=3
KillMode=control-group
TimeoutStopSec=10
[Install]
WantedBy=graphical-session.target
''' % (unit(prefix/'overlay-host'),unit(config)))
exe=str(prefix/'launch').replace('\\','\\\\').replace('"','\\"').replace('`','\\`').replace('$','\\$').replace('%','%%')
(data/'applications/suflyor.desktop').write_text('''[Desktop Entry]
Type=Application
Name=Suflyor
Comment=Meeting and interview assistant
Exec="%s"
Terminal=false
Categories=Utility;Office;
StartupWMClass=suflyor
Icon=suflyor
''' % exe)
PY
systemctl --user daemon-reload
systemctl --user link "$config/systemd/user/suflyor.service" >/dev/null
hyprctl reload >/dev/null
errors=$(hyprctl configerrors)
[[ -z "$errors" ]] || { printf '%s\n' "$errors" >&2; exit 1; }
command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" || true
printf 'Installed native binaries and launcher. Start: systemctl --user start suflyor.service\n'
printf 'Optional autostart: bash scripts/install-linux.sh autostart\n'
