#!/usr/bin/env bash
# Experimental native-port checks; run on the dedicated Linux worker.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"
export SLINT_EMIT_DEBUG_INFO=1
cargo check --locked --bin overlay-host --features ui-mcp --manifest-path "$root/slint-experiment/Cargo.toml"
cargo test --locked --features ui-mcp --test linux_platform --manifest-path "$root/slint-experiment/Cargo.toml"
# The clipboard round trip deliberately requires an isolated live Wayland session:
# cargo test --locked --features ui-mcp --test linux_platform --manifest-path slint-experiment/Cargo.toml -- --ignored
