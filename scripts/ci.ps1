# suflyor local CI runner — fmt + clippy + tests for all five crates.
# Run BEFORE every commit (the .claude/hooks/git-gate.ps1 hook runs the
# same checks automatically on commit/push).
#
# Covered: cargo fmt --check, clippy --all-targets -D warnings, test
#   for slint-experiment, overlay-backend, suflyor-wsola, suflyor-tts,
#   AND suflyor-teratts.
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

# Memory hygiene (2026-06-23): cap parallel rustc jobs for the gate. A COLD
# `cargo test` (e.g. right after a toolchain bump / cargo clean, when the
# artifact cache is empty) codegens the 4 heavy Slint bins (overlay-host,
# slint-replay, overlay-spike, markdown-spike) at once; at the default job
# count that exhausts RAM (rustc-LLVM ERROR: out of memory). -j2 fits. This
# only constrains the gate — interactive `cargo run`/`build` (no env set) keeps
# full parallelism, and it never hits this because it rebuilds ONE crate. Keep
# an explicit caller limit for low-memory or concurrently-used machines.
if (-not $env:CARGO_BUILD_JOBS) {
    $env:CARGO_BUILD_JOBS = "2"
}

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
# Rust test executables live under target/debug/deps. The statically linked
# DirectML provider imports DMLCreateDevice1 before main; older Windows builds
# have a system DirectML.dll without that export, so stage ort's matching
# redistributable beside the test executables just like the release build does.
Run-Step "stage DirectML for slint tests" {
    $dmlSource = Join-Path $projectRoot "slint-experiment\target\debug\DirectML.dll"
    if (-not (Test-Path $dmlSource)) {
        throw "matching DirectML.dll not found after the Slint build"
    }
    Copy-Item -LiteralPath $dmlSource -Destination (Join-Path $projectRoot "slint-experiment\target\debug\deps\DirectML.dll") -Force
}
# NOT --lib: it skips tests/ (i18n_guard + any guard test). Run the full suite.
Run-Step "slint test" {
    & $cargoExe test --manifest-path slint-experiment/Cargo.toml --quiet
}
Run-Step "slint ui-mcp feature check" {
    # QA-only rot guard: default builds intentionally omit the embedded MCP
    # server, so this path needs its own fast compile check. Not a release feature.
    & $cargoExe check --locked --manifest-path slint-experiment/Cargo.toml --bin overlay-host --features ui-mcp
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

Run-Step "wsola fmt --check" {
    & $cargoExe fmt --manifest-path suflyor-wsola/Cargo.toml --all -- --check
}
Run-Step "wsola clippy -D warnings" {
    & $cargoExe clippy --manifest-path suflyor-wsola/Cargo.toml --all-targets -- -D warnings
}
Run-Step "wsola test" {
    & $cargoExe test --manifest-path suflyor-wsola/Cargo.toml --quiet
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

# --- suflyor-teratts (experimental TeraTTSv2 read-aloud sidecar, RC17) ---
# Same shared target dir: its ort prebuilt download is reused by the host's
# ort/GigaAM artifacts instead of re-downloading.
Run-Step "teratts fmt --check" {
    & $cargoExe fmt --manifest-path suflyor-teratts/Cargo.toml --all -- --check
}
Run-Step "teratts clippy -D warnings" {
    & $cargoExe clippy --manifest-path suflyor-teratts/Cargo.toml --all-targets -- -D warnings
}
Run-Step "teratts test" {
    & $cargoExe test --manifest-path suflyor-teratts/Cargo.toml --quiet
}
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "All gating layers green." -ForegroundColor Green
exit 0
