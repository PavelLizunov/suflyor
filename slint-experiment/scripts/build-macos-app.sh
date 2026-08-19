#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-macos-app.sh must run on macOS" >&2
  exit 1
fi

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$crate_root/target}"
app_dir="$target_dir/Suflyor.app"
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
binary="$target_dir/release/overlay-host"

export CARGO_INCREMENTAL=0
cargo build --locked --release --bin overlay-host \
  --manifest-path "$crate_root/Cargo.toml"

cargo build --locked --release --manifest-path "$crate_root/../suflyor-tts/Cargo.toml"
cargo build --locked --release --manifest-path "$crate_root/../suflyor-teratts/Cargo.toml"

rm -rf "$app_dir"
mkdir -p "$macos_dir" "$resources_dir"
cp "$crate_root/macos/Info.plist" "$contents_dir/Info.plist"
cp "$binary" "$macos_dir/overlay-host"

sidecar_binary="$crate_root/../suflyor-tts/target/release/suflyor-tts"
if [[ ! -f "$sidecar_binary" ]]; then
  sidecar_binary="$target_dir/release/suflyor-tts"
fi
if [[ -f "$sidecar_binary" ]]; then
  cp "$sidecar_binary" "$macos_dir/suflyor-tts"
  chmod 755 "$macos_dir/suflyor-tts"
fi

tera_binary="$crate_root/../suflyor-teratts/target/release/suflyor-teratts"
if [[ ! -f "$tera_binary" ]]; then
  tera_binary="$target_dir/release/suflyor-teratts"
fi
if [[ -f "$tera_binary" ]]; then
  cp "$tera_binary" "$macos_dir/suflyor-teratts"
  chmod 755 "$macos_dir/suflyor-teratts"
fi

chmod 755 "$macos_dir/overlay-host"

# Package AppIcon.icns
icon_source="$crate_root/assets/icon-source.png"
if [[ ! -f "$icon_source" ]]; then
  icon_source="$crate_root/assets/icon.png"
fi
if [[ -f "$icon_source" ]]; then
  tmp_parent="$(mktemp -d)"
  iconset_dir="$tmp_parent/AppIcon.iconset"
  mkdir -p "$iconset_dir"
  sips -z 16 16     "$icon_source" --out "$iconset_dir/icon_16x16.png" >/dev/null 2>&1 || true
  sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_16x16@2x.png" >/dev/null 2>&1 || true
  sips -z 32 32     "$icon_source" --out "$iconset_dir/icon_32x32.png" >/dev/null 2>&1 || true
  sips -z 64 64     "$icon_source" --out "$iconset_dir/icon_32x32@2x.png" >/dev/null 2>&1 || true
  sips -z 128 128   "$icon_source" --out "$iconset_dir/icon_128x128.png" >/dev/null 2>&1 || true
  sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_128x128@2x.png" >/dev/null 2>&1 || true
  sips -z 256 256   "$icon_source" --out "$iconset_dir/icon_256x256.png" >/dev/null 2>&1 || true
  sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_256x256@2x.png" >/dev/null 2>&1 || true
  sips -z 512 512   "$icon_source" --out "$iconset_dir/icon_512x512.png" >/dev/null 2>&1 || true
  sips -z 1024 1024 "$icon_source" --out "$iconset_dir/icon_512x512@2x.png" >/dev/null 2>&1 || true
  iconutil -c icns "$iconset_dir" -o "$resources_dir/AppIcon.icns"
  rm -rf "$tmp_parent"
fi

codesign --force --sign - --options runtime \
  --entitlements "$crate_root/macos/entitlements.plist" \
  --identifier com.ninitux.suflyor.macos "$app_dir"
codesign --verify --deep --strict --verbose=2 "$app_dir"

echo "$app_dir"
