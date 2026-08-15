#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
export CARGO_INCREMENTAL=0

if [[ "$(uname -m)" != "arm64" ]]; then
  echo "macOS gate requires an Apple Silicon arm64 host" >&2
  exit 1
fi

echo "=== macOS backend ==="
cargo fmt --manifest-path overlay-backend/Cargo.toml --all -- --check
cargo clippy --manifest-path overlay-backend/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path overlay-backend/Cargo.toml

echo "=== macOS portable Slint/UI ==="
cargo fmt --manifest-path slint-experiment/Cargo.toml --all -- --check
cargo check --locked --lib --bin slint-replay --bin markdown-spike \
  --manifest-path slint-experiment/Cargo.toml
cargo clippy --locked --lib --bin slint-replay --bin markdown-spike \
  --manifest-path slint-experiment/Cargo.toml -- -D warnings
cargo test --locked --lib --manifest-path slint-experiment/Cargo.toml
cargo test --locked --bin slint-replay --manifest-path slint-experiment/Cargo.toml

echo "=== macOS TTS sidecar ==="
cargo fmt --manifest-path suflyor-tts/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path suflyor-tts/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path suflyor-tts/Cargo.toml

echo "All macOS compile-seam layers green."
