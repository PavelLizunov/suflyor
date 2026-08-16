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

rm -rf "$app_dir"
mkdir -p "$macos_dir" "$resources_dir"
cp "$crate_root/macos/Info.plist" "$contents_dir/Info.plist"
cp "$binary" "$macos_dir/overlay-host"
chmod 755 "$macos_dir/overlay-host"

codesign --force --sign - --options runtime \
  --identifier com.ninitux.suflyor.macos "$app_dir"
codesign --verify --deep --strict --verbose=2 "$app_dir"

echo "$app_dir"
