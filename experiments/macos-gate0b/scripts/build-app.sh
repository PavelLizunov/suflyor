#!/bin/zsh
set -euo pipefail

root_dir="$(cd "$(dirname "$0")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root_dir/target}"
app_dir="$target_dir/Suflyor Gate 0B.app"
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
sign_identity="${SIGN_IDENTITY:--}"

export CARGO_INCREMENTAL=0
cargo build --locked --release --manifest-path "$root_dir/Cargo.toml"

rm -rf "$app_dir"
mkdir -p "$macos_dir"
cp "$root_dir/Info.plist" "$contents_dir/Info.plist"
cp "$target_dir/release/suflyor-macos-gate0b" "$macos_dir/suflyor-macos-gate0b"

codesign --force --sign "$sign_identity" --options runtime \
    --entitlements "$root_dir/entitlements.plist" \
    --identifier com.ninitux.suflyor.dev "$app_dir"
codesign --verify --deep --strict --verbose=2 "$app_dir"

echo "$app_dir"
