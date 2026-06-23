# overlay-mvp local CI runner — fmt + clippy + tests for both crates.
# Run BEFORE every commit (the .claude/hooks/git-gate.ps1 hook runs the
# same checks automatically on commit/push).
#
# Covered: cargo fmt --check, clippy --all-targets -D warnings, test --lib
#   for slint-experiment AND overlay-backend.
#
# Not covered here (do manually): review-agent pass
# (docs/REVIEW_AGENT_PROMPT.md) + a live smoke run of the overlay.
#
# Exit code: 0 = green, non-zero = first failing step.
# Run from project root:  powershell scripts/ci.ps1

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

# Disk hygiene (added 2026-06-19): skip incremental for gate builds. clippy
# --all-targets + test spawn many target/debug/incremental/<hash> dirs that
# cargo never GCs; they reached 281 GB by 2026-06-19. The gate isn't an edit
# loop, so incremental is pure waste here. Interactive `cargo run` (no env)
# keeps incremental. Mirror of the same line in .claude/hooks/git-gate.ps1.
$env:CARGO_INCREMENTAL = "0"

$cargoExe = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargoExe)) {
    Write-Host "ERROR: cargo not found at $cargoExe" -ForegroundColor Red
    exit 10
}

function Run-Step($name, $block) {
    Write-Host ""
    Write-Host "=== $name ===" -ForegroundColor Cyan
    $start = Get-Date
    & $block
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "FAIL: $name (exit $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    $elapsed = [math]::Round(((Get-Date) - $start).TotalSeconds, 1)
    Write-Host "PASS: $name (${elapsed}s)" -ForegroundColor Green
}

# Phase 7 cut: the React/Tauri (src-tauri) + `npx tsc` layers were removed
# with the stack. The product is now slint-experiment + overlay-backend.

# --- slint-experiment (UI + orchestration) ---
Run-Step "slint fmt --check" {
    & $cargoExe fmt --manifest-path slint-experiment/Cargo.toml --all -- --check
}
Run-Step "slint clippy -D warnings" {
    & $cargoExe clippy --manifest-path slint-experiment/Cargo.toml --all-targets -- -D warnings
}
# NOT --lib: it skips tests/ (i18n_guard + any guard test). Run the full suite.
Run-Step "slint test" {
    & $cargoExe test --manifest-path slint-experiment/Cargo.toml --quiet
}

# --- overlay-backend (shared logic) ---
Run-Step "backend fmt --check" {
    & $cargoExe fmt --manifest-path overlay-backend/Cargo.toml --all -- --check
}
Run-Step "backend clippy -D warnings" {
    & $cargoExe clippy --manifest-path overlay-backend/Cargo.toml --all-targets -- -D warnings
}
Run-Step "backend test" {
    & $cargoExe test --manifest-path overlay-backend/Cargo.toml --quiet
}

# --- suflyor-tts (read-aloud sidecar — shipped in the installer) ---
# Build into the shared slint target dir so the cached sherpa-onnx native lib is
# reused (a cold suflyor-tts/target build re-downloads it from GitHub).
$env:CARGO_TARGET_DIR = Join-Path $projectRoot "slint-experiment\target"
Run-Step "tts fmt --check" {
    & $cargoExe fmt --manifest-path suflyor-tts/Cargo.toml --all -- --check
}
Run-Step "tts clippy -D warnings" {
    & $cargoExe clippy --manifest-path suflyor-tts/Cargo.toml --all-targets -- -D warnings
}
Run-Step "tts test" {
    & $cargoExe test --manifest-path suflyor-tts/Cargo.toml --quiet
}
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "All gating layers green." -ForegroundColor Green
exit 0
