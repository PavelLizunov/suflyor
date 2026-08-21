#!/bin/zsh
set -euo pipefail

root_dir="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

cargo fmt --manifest-path "$root_dir/Cargo.toml" -- --check
cargo clippy --locked --manifest-path "$root_dir/Cargo.toml" -- -D warnings
plutil -lint "$root_dir/Info.plist" "$root_dir/entitlements.plist"
"$root_dir/scripts/build-app.sh"
