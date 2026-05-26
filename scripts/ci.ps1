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

# Layer 1a — fmt check (most common CI killer per vpnctl methodology)
Run-Step "Layer 1a: cargo fmt --check" {
    & $cargoExe fmt --manifest-path src-tauri/Cargo.toml --all -- --check
}

# Layer 1b — clippy (workspace = lib + bins + tests)
Run-Step "Layer 1b: cargo clippy -D warnings" {
    & $cargoExe clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
}

# Layer 2 — cargo test (260+ unit + integration)
Run-Step "Layer 2: cargo test --lib" {
    & $cargoExe test --manifest-path src-tauri/Cargo.toml --lib --quiet
}

# Layer 3a — copy contract (canonical strings frozen)
Run-Step "Layer 3a: copy contract tests" {
    & $cargoExe test --manifest-path src-tauri/Cargo.toml --test copy_contract --quiet
}

# Layer 3b — TypeScript correctness (catches noUnusedLocals + type errors)
Run-Step "Layer 3b: npx tsc --noEmit" {
    & npx tsc --noEmit
}

Write-Host ""
Write-Host "All gating layers green. Run scripts/visual_check.ps1 before pushing." -ForegroundColor Green
exit 0
