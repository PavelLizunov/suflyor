# overlay-mvp local CI runner — layers 1, 2, 3 of the methodology.
# Mirrors what vpnctl's `just ci` does. Run BEFORE every commit.
#
# Layers covered:
#   1. cargo clippy --all-targets -- -D warnings
#   2. cargo test --lib  (Rust unit + integration)
#   3. cargo test --test copy_contract  (canonical strings)
#      AND  npx tsc --noEmit               (TypeScript correctness)
#
# What this script does NOT do (separate scripts):
#   4. review-agent — manual Agent call, see docs/REVIEW_AGENT_PROMPT.md
#   5. live install + smoke — scripts/visual_check.ps1
#   6. visual gate — Claude reads scripts/visual_check.ps1's PNG output
#
# Exit code: 0 = green, non-zero = first failing layer.
# Run from project root:  pwsh scripts/ci.ps1

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

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
Run-Step "slint test --lib" {
    & $cargoExe test --manifest-path slint-experiment/Cargo.toml --lib --quiet
}

# --- overlay-backend (shared logic) ---
Run-Step "backend fmt --check" {
    & $cargoExe fmt --manifest-path overlay-backend/Cargo.toml --all -- --check
}
Run-Step "backend clippy -D warnings" {
    & $cargoExe clippy --manifest-path overlay-backend/Cargo.toml --all-targets -- -D warnings
}
Run-Step "backend test --lib" {
    & $cargoExe test --manifest-path overlay-backend/Cargo.toml --lib --quiet
}

Write-Host ""
Write-Host "All gating layers green." -ForegroundColor Green
exit 0
