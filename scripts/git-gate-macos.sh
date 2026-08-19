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
cargo check --locked --lib --bin slint-replay --bin markdown-spike --bin overlay-host \
  --manifest-path slint-experiment/Cargo.toml
cargo clippy --locked --lib --bin slint-replay --bin markdown-spike --bin overlay-host --tests \
  --manifest-path slint-experiment/Cargo.toml -- -D warnings
cargo test --locked --lib --manifest-path slint-experiment/Cargo.toml
cargo test --locked --bin slint-replay --manifest-path slint-experiment/Cargo.toml
cargo test --locked --bin overlay-host --manifest-path slint-experiment/Cargo.toml
# The two omitted integration tests create/show Slint windows and belong to live GUI QA.
for test in \
  codex_copy_guard \
  i18n_guard \
  icon_guard \
  lock_chip_geometry_guard \
  lock_chip_layout_guard \
  lock_mode_menu_guard \
  macos_app_packaging_guard \
  macos_capture_watchdog_guard \
  macos_global_hotkeys_guard \
  macos_popups_guard \
  macos_settings_guard \
  macos_text_ask_guard \
  macos_tile_manager_guard \
  macos_ui_callbacks_guard \
  native_lifecycle_guard \
  native_macos_status_guard \
  native_macos_window_guard \
  native_screen_guard \
  rc3_regression_guard \
  settings_reset_guard \
  tera_tts_layout_guard \
  tile_player_layout_guard \
  tray_guard \
  version_guard
do
  cargo test --locked --test "$test" --manifest-path slint-experiment/Cargo.toml
done

echo "=== macOS TTS sidecar ==="
cargo fmt --manifest-path suflyor-tts/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path suflyor-tts/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path suflyor-tts/Cargo.toml

echo "=== macOS WSOLA sidecar ==="
cargo fmt --manifest-path suflyor-wsola/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path suflyor-wsola/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path suflyor-wsola/Cargo.toml

echo "=== macOS TeraTTS sidecar ==="
cargo fmt --manifest-path suflyor-teratts/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path suflyor-teratts/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path suflyor-teratts/Cargo.toml

echo "All macOS compile-seam layers green."
