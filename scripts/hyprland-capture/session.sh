#!/usr/bin/env bash
# Select the private compositor for the next uwsm login without restarting this session.
set -euo pipefail

data_home=${XDG_DATA_HOME:-$HOME/.local/share}
state_home=${XDG_STATE_HOME:-$HOME/.local/state}
entry="$data_home/wayland-sessions/hyprland.desktop"
launcher="$data_home/suflyor-hyprland/start"
state="$state_home/suflyor-hyprland"
marker='# Suflyor clean capture session override'

owns_entry() {
  [[ -f "$entry" ]] && grep -Fxq -- "$marker" "$entry"
}

case ${1:-} in
  enable)
    binary=$(realpath -- "${2:?Pass the installed patched Hyprland binary}")
    [[ -x "$binary" ]] || { printf 'Not executable: %s\n' "$binary" >&2; exit 1; }
    if [[ -e "$entry" ]] && ! owns_entry; then
      printf 'An unrelated user session override already exists: %s\n' "$entry" >&2
      exit 1
    fi
    mkdir -p "$(dirname -- "$entry")" "$(dirname -- "$launcher")" "$state"
    printf '%s\n' "$binary" > "$state/binary"
    {
      printf '#!/usr/bin/env bash\nset -euo pipefail\n'
      printf 'binary=%q\n' "$binary"
      printf 'state=%q\n' "$state"
      printf 'if [[ -e "$state/disabled" || ! -x "$binary" ]]; then\n'
      printf '  exec /usr/bin/start-hyprland "$@"\nfi\n'
      printf 'exec /usr/bin/start-hyprland --path "$binary" "$@"\n'
    } > "$launcher"
    chmod 755 "$launcher"
    rm -f -- "$state/disabled"
    temporary=$(mktemp "$(dirname -- "$entry")/.hyprland-suflyor.XXXXXX")
    {
      printf '[Desktop Entry]\n%s\n' "$marker"
      printf 'Name=Hyprland\nComment=Hyprland with Suflyor clean capture\n'
      printf 'Exec="%s"\n' "$launcher"
      printf 'Type=Application\nDesktopNames=Hyprland\nKeywords=tiling;wayland;compositor;\n'
    } > "$temporary"
    chmod 644 "$temporary"
    mv -- "$temporary" "$entry"
    printf 'Selected for the next uwsm login. The running compositor was not restarted.\n'
    ;;
  disable)
    mkdir -p "$state"
    touch "$state/disabled"
    if owns_entry; then
      rm -- "$entry"
    fi
    printf 'Stock Hyprland selected for the next login. The running compositor was not restarted.\n'
    ;;
  status)
    printf 'Next uwsm login: '
    if owns_entry && [[ ! -e "$state/disabled" ]]; then
      printf 'Suflyor clean capture\n'
    else
      printf 'stock or other configuration\n'
    fi
    printf '\nRunning compositor (not a capture test):\n'
    hyprctl version
    ;;
  *)
    printf 'Usage: %s enable /absolute/path/to/Hyprland | disable | status\n' "$0" >&2
    exit 2
    ;;
esac
