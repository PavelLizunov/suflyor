#!/usr/bin/env bash
# Install a prebuilt native candidate; user data and models are never removed.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
action="${1:-install}"
prefix="${SUFLYOR_PREFIX:-$HOME/.local/lib/suflyor}"
bin_dir="${SUFLYOR_BIN_DIR:-$HOME/.local/bin}"
config="${XDG_CONFIG_HOME:-$HOME/.config}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
source_dir="${SUFLYOR_BUILD_DIR:-$root/overlay-backend/target/debug}"
if [[ "$action" = uninstall ]]; then
    systemctl --user disable --now suflyor.service 2>/dev/null || true
    rm -f "$config/systemd/user/suflyor.service" "$data/applications/suflyor.desktop"
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
for tool in pactl parec paplay grim slurp wl-copy wl-paste hyprctl xdg-open tesseract; do
    command -v "$tool" >/dev/null || { printf 'Missing required native tool: %s\n' "$tool" >&2; exit 1; }
done
for name in overlay-host suflyor-tts suflyor-teratts; do
    [[ -x "$source_dir/$name" ]] || { printf 'Missing native executable: %s\n' "$source_dir/$name" >&2; exit 1; }
done
mkdir -p "$prefix" "$bin_dir" "$config/systemd/user" "$data/applications"
for name in overlay-host suflyor-tts suflyor-teratts; do
    install -m755 "$source_dir/$name" "$prefix/.$name.next"
    mv "$prefix/.$name.next" "$prefix/$name"
done
ln -sfn "$prefix/overlay-host" "$bin_dir/suflyor"
python3 - "$prefix" "$config" "$data" <<'PY'
import pathlib,sys
prefix,config,data=map(pathlib.Path,sys.argv[1:])
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
Restart=on-failure
RestartSec=3
KillMode=control-group
TimeoutStopSec=10
[Install]
WantedBy=graphical-session.target
''' % unit(prefix/'overlay-host'))
exe=str(prefix/'overlay-host').replace('\\','\\\\').replace('"','\\"').replace('`','\\`').replace('$','\\$').replace('%','%%')
(data/'applications/suflyor.desktop').write_text('''[Desktop Entry]
Type=Application
Name=Suflyor
Comment=Meeting and interview assistant
Exec="%s"
Terminal=false
Categories=Utility;Office;
StartupWMClass=suflyor
''' % exe)
PY
systemctl --user daemon-reload
command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" || true
printf 'Installed native binaries and launcher. Start: systemctl --user start suflyor.service\n'
printf 'Optional autostart: bash scripts/install-linux.sh autostart\n'
