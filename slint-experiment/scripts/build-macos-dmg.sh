#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-macos-dmg.sh must run on macOS" >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_root="$(cd "$script_dir/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$crate_root/target}"
mkdir -p "$target_dir"
target_dir="$(cd "$target_dir" && pwd -P)"
if [[ "$target_dir" == "/" ]]; then
  echo "refusing to package into the filesystem root" >&2
  exit 1
fi
install_guide="$crate_root/../docs/macos-install.md"
if [[ ! -s "$install_guide" ]]; then
  echo "required macOS installation guide is missing" >&2
  exit 1
fi

version="$(awk '
  /^\[package\]$/ { package = 1; next }
  package && /^\[/ { exit }
  package && /^version = "/ {
    value = $0
    sub(/^version = "/, "", value)
    sub(/".*$/, "", value)
    print value
    exit
  }
' "$crate_root/Cargo.toml")"
if [[ -z "$version" || "$version" == */* ]]; then
  echo "failed to read a safe package version" >&2
  exit 1
fi

export CARGO_TARGET_DIR="$target_dir"
export CARGO_BUILD_JOBS=2
if [[ -n "${SUFLYOR_MACOS_SIGN_IDENTITY:-}" ]]; then
  echo "note: packaging with the requested stable local signing identity" >&2
else
  echo "note: ad-hoc signing can require fresh macOS capture permissions after a rebuild" >&2
fi
"$script_dir/build-macos-app.sh" >&2

app_dir="$target_dir/Suflyor.app"
macos_dir="$app_dir/Contents/MacOS"
if [[ ! -d "$app_dir" ]]; then
  echo "packaged app is missing: $app_dir" >&2
  exit 1
fi
for executable in overlay-host suflyor-tts suflyor-teratts suflyor-mlx; do
  path="$macos_dir/$executable"
  if [[ ! -x "$path" || "$(lipo -archs "$path")" != "arm64" ]]; then
    echo "packaged executable must be thin arm64: $executable" >&2
    exit 1
  fi
done
codesign --verify --deep --strict --verbose=2 "$app_dir"

bundle_dir="$target_dir/bundle"
if [[ -L "$bundle_dir" ]]; then
  echo "refusing to package through a symlinked bundle directory" >&2
  exit 1
fi
mkdir -p "$bundle_dir"
bundle_dir="$(cd "$bundle_dir" && pwd -P)"
if [[ "$(dirname "$bundle_dir")" != "$target_dir" \
  || "$(basename "$bundle_dir")" != "bundle" ]]; then
  echo "refusing to package into an unexpected bundle directory" >&2
  exit 1
fi
dmg_name="Suflyor-${version}-macos-arm64.dmg"
dmg_path="$bundle_dir/$dmg_name"
staging_dir="$(mktemp -d "$target_dir/.dmg-staging.XXXXXX")"
mount_dir="$(mktemp -d "$target_dir/.dmg-mount.XXXXXX")"
tmp_dmg="$bundle_dir/.${dmg_name}.tmp.dmg"
attached=0
cleanup() {
  if [[ "$attached" -eq 1 ]]; then
    hdiutil detach "$mount_dir" -force >/dev/null 2>&1 || true
  fi
  rm -rf -- "$staging_dir" "$mount_dir"
  rm -f -- "$tmp_dmg"
}
trap cleanup EXIT

ditto "$app_dir" "$staging_dir/Suflyor.app"
ln -s /Applications "$staging_dir/Applications"
install -m 644 "$install_guide" "$staging_dir/Install Suflyor.txt"
rm -f -- "$tmp_dmg"
hdiutil create -quiet -fs HFS+ -format UDZO -imagekey zlib-level=9 \
  -volname "Suflyor $version" -srcfolder "$staging_dir" "$tmp_dmg"

hdiutil attach -quiet -readonly -nobrowse -mountpoint "$mount_dir" "$tmp_dmg"
attached=1
if [[ ! -d "$mount_dir/Suflyor.app" || ! -L "$mount_dir/Applications" \
  || "$(readlink "$mount_dir/Applications")" != "/Applications" \
  || ! -s "$mount_dir/Install Suflyor.txt" ]]; then
  echo "DMG does not contain the expected drag-install layout" >&2
  exit 1
fi
if ! cmp -s "$install_guide" "$mount_dir/Install Suflyor.txt"; then
  echo "DMG installation guide differs from its source" >&2
  exit 1
fi
for guide_marker in "Open Anyway" "Microphone" "Screen & System Audio Recording" "Accessibility"; do
  if ! grep -Fq "$guide_marker" "$mount_dir/Install Suflyor.txt"; then
    echo "DMG installation guide is incomplete: $guide_marker" >&2
    exit 1
  fi
done
codesign --verify --deep --strict --verbose=2 "$mount_dir/Suflyor.app"
hdiutil detach "$mount_dir" >/dev/null
attached=0
mv -f -- "$tmp_dmg" "$dmg_path"

bytes="$(stat -f %z "$dmg_path")"
sha256="$(shasum -a 256 "$dmg_path" | awk '{print $1}')"
printf 'dmg_path=%s\n' "$dmg_path"
printf 'dmg_bytes=%s\n' "$bytes"
printf 'dmg_sha256=%s\n' "$sha256"
