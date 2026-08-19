#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-macos-app.sh must run on macOS" >&2
  exit 1
fi

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$crate_root/target}"
mkdir -p "$target_dir"
target_dir="$(cd "$target_dir" && pwd -P)"
if [[ "$target_dir" == "/" ]]; then
  echo "refusing to package into the filesystem root" >&2
  exit 1
fi
export CARGO_TARGET_DIR="$target_dir"
app_dir="$target_dir/Suflyor.app"
if [[ "$(dirname "$app_dir")" != "$target_dir" || "$(basename "$app_dir")" != "Suflyor.app" ]]; then
  echo "refusing to replace an unexpected app path: $app_dir" >&2
  exit 1
fi
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
binary="$target_dir/release/overlay-host"
sidecar_binary="$target_dir/release/suflyor-tts"
tera_binary="$target_dir/release/suflyor-teratts"

export CARGO_INCREMENTAL=0
cargo build --locked --release --bin overlay-host \
  --manifest-path "$crate_root/Cargo.toml"

cargo build --locked --release --manifest-path "$crate_root/../suflyor-tts/Cargo.toml"
cargo build --locked --release --manifest-path "$crate_root/../suflyor-teratts/Cargo.toml"

for executable in "$binary" "$sidecar_binary" "$tera_binary"; do
  if [[ ! -x "$executable" ]]; then
    echo "required executable missing after build: $executable" >&2
    exit 1
  fi
done

plutil -lint "$crate_root/macos/Info.plist"
plutil -lint "$crate_root/macos/entitlements.plist"

rm -rf -- "$app_dir"
mkdir -p "$macos_dir" "$resources_dir"
cp "$crate_root/macos/Info.plist" "$contents_dir/Info.plist"
install -m 755 "$binary" "$macos_dir/overlay-host"
install -m 755 "$sidecar_binary" "$macos_dir/suflyor-tts"
install -m 755 "$tera_binary" "$macos_dir/suflyor-teratts"

# Package AppIcon.icns
icon_source="$crate_root/assets/icon-source.png"
if [[ ! -f "$icon_source" ]]; then
  icon_source="$crate_root/assets/icon.png"
fi
if [[ ! -f "$icon_source" ]]; then
  echo "required app icon source is missing" >&2
  exit 1
fi
tmp_parent="$(mktemp -d)"
trap 'rm -rf -- "$tmp_parent"' EXIT
iconset_dir="$tmp_parent/AppIcon.iconset"
mkdir -p "$iconset_dir"
sips -z 16 16     "$icon_source" --out "$iconset_dir/icon_16x16.png" >/dev/null
sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_16x16@2x.png" >/dev/null
sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_32x32.png" >/dev/null
sips -z 64 64     "$icon_source" --out "$iconset_dir/icon_32x32@2x.png" >/dev/null
sips -z 128 128   "$icon_source" --out "$iconset_dir/icon_128x128.png" >/dev/null
sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_256x256.png" >/dev/null
sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_512x512.png" >/dev/null
sips -z 1024 1024 "$icon_source" --out "$iconset_dir/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$iconset_dir" -o "$resources_dir/AppIcon.icns"
if [[ ! -s "$resources_dir/AppIcon.icns" ]]; then
  echo "AppIcon.icns was not created" >&2
  exit 1
fi
rm -rf -- "$tmp_parent"
trap - EXIT

# Local-only ad-hoc signatures. Public distribution requires a separate owner-
# authorized Developer ID and notarization flow; this script never publishes.
codesign --force --sign - --options runtime "$macos_dir/suflyor-tts"
codesign --force --sign - --options runtime "$macos_dir/suflyor-teratts"
codesign --force --sign - --options runtime \
  --entitlements "$crate_root/macos/entitlements.plist" \
  "$app_dir"
codesign --verify --strict --verbose=2 "$macos_dir/suflyor-tts"
codesign --verify --strict --verbose=2 "$macos_dir/suflyor-teratts"
codesign --verify --deep --strict --verbose=2 "$app_dir"
plutil -lint "$contents_dir/Info.plist"

echo "$app_dir"
