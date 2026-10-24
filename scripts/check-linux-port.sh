#!/usr/bin/env bash
# Experimental native-port checks; run on the dedicated Linux worker.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"
export SLINT_EMIT_DEBUG_INFO=1
cargo check --locked --no-default-features --lib --bin overlay-host --features ui-mcp --manifest-path "$root/slint-experiment/Cargo.toml"
# Integration tests build all eligible binaries, even with explicit --bin selection.
# Disable only our demo-binaries feature; dependency defaults stay unchanged.
CARGO_BUILD_JOBS=1 cargo test --locked --no-default-features --features ui-mcp --lib --bin overlay-host --test linux_platform --manifest-path "$root/slint-experiment/Cargo.toml"
for filter in capabilities::tests:: linux_installers_reject_windows_assets update::tests::; do
    cargo test --lib --manifest-path "$root/overlay-backend/Cargo.toml" "$filter"
done
# The clipboard round trip deliberately requires an isolated live Wayland session:
# cargo test --locked --no-default-features --features ui-mcp --test linux_platform --manifest-path slint-experiment/Cargo.toml -- --ignored
printf 'Linux port gate PASS\n'
