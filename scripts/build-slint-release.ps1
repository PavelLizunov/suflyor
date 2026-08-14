# Phase D3 — build the Slint binary in release mode + report the
# artifact location + size + timestamp. Optionally produces the NSIS
# installer when makensis.exe is available.
#
# Usage:
#   pwsh scripts/build-slint-release.ps1            # build only
#   pwsh scripts/build-slint-release.ps1 -Installer # also run makensis
#
# After build, install the exe via:
#   1. Copy target/release/overlay-host.exe somewhere on PATH
#   OR
#   2. Run scripts/slint-installer.nsi via makensis to make an NSIS
#      installer (target/release/bundle/suflyor-slint-setup.exe).

param([switch]$Installer)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$crate = Join-Path $projectRoot "slint-experiment"

$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Write-Host "[build-slint-release] cargo build --release --bin overlay-host" -ForegroundColor Cyan
Set-Location $crate
& cargo build --release --bin overlay-host
if ($LASTEXITCODE -ne 0) {
    Write-Host "build failed: exit $LASTEXITCODE" -ForegroundColor Red
    exit $LASTEXITCODE
}

$exe = Join-Path $crate "target\release\overlay-host.exe"
if (-not (Test-Path $exe)) {
    Write-Host "ERROR: build succeeded but $exe missing" -ForegroundColor Red
    exit 11
}

# Read-aloud TTS sidecar (separate process — its onnxruntime can't share a
# binary with the app's ort/GigaAM STT). Build it into the SAME target dir so it
# lands beside overlay-host.exe (CARGO_TARGET_DIR reuses the cached sherpa lib).
Write-Host "[build-slint-release] cargo build --release suflyor-tts (read-aloud sidecar)" -ForegroundColor Cyan
$env:CARGO_TARGET_DIR = Join-Path $crate "target"
& cargo build --release --manifest-path (Join-Path $projectRoot "suflyor-tts\Cargo.toml")
$sidecarExit = $LASTEXITCODE
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
if ($sidecarExit -ne 0) {
    Write-Host "sidecar build failed: exit $sidecarExit" -ForegroundColor Red
    exit $sidecarExit
}
$sidecar = Join-Path $crate "target\release\suflyor-tts.exe"
if (-not (Test-Path $sidecar)) {
    Write-Host "ERROR: sidecar build succeeded but $sidecar missing" -ForegroundColor Red
    exit 12
}
Write-Host "  sidecar: $sidecar" -ForegroundColor Green

# RC17 — experimental TeraTTSv2 read-aloud sidecar (ort ONNX graphs; separate
# process, same reason as suflyor-tts). Same shared target dir: the ort
# prebuilt is already cached by the host build.
Write-Host "[build-slint-release] cargo build --release suflyor-teratts (Tera sidecar)" -ForegroundColor Cyan
$env:CARGO_TARGET_DIR = Join-Path $crate "target"
& cargo build --release --manifest-path (Join-Path $projectRoot "suflyor-teratts\Cargo.toml")
$teraExit = $LASTEXITCODE
Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
if ($teraExit -ne 0) {
    Write-Host "teratts sidecar build failed: exit $teraExit" -ForegroundColor Red
    exit $teraExit
}
$teraSidecar = Join-Path $crate "target\release\suflyor-teratts.exe"
if (-not (Test-Path $teraSidecar)) {
    Write-Host "ERROR: teratts build succeeded but $teraSidecar missing" -ForegroundColor Red
    exit 14
}
Write-Host "  teratts sidecar: $teraSidecar" -ForegroundColor Green

# DirectML EP (GigaAM GPU): ort links DMLCreateDevice1 at process startup, so
# Windows builds must ship the matching DirectML redistributable. Older Windows
# 10 releases have a system DirectML.dll without that export and otherwise fail
# before the app can fall back to CPU. ort records the exact downloaded binary
# in its build output and may leave a 0-byte symlink beside the exe; materialize
# that target as a regular file for NSIS.
$dmlDst = Join-Path $crate "target\release\DirectML.dll"
$ortBuildDir = Join-Path $crate "target\release\build"
$dmlSource = Get-ChildItem -Path (Join-Path $ortBuildDir "ort-sys-*\output") -File -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    ForEach-Object {
        foreach ($line in Get-Content -LiteralPath $_.FullName) {
            if ($line -match '^cargo:rustc-link-search=native=(.+)$') {
                $candidate = Join-Path $Matches[1] "DirectML.dll"
                if ((Test-Path -LiteralPath $candidate) -and (Get-Item -LiteralPath $candidate).Length -gt 0) {
                    $candidate
                }
            }
        }
    } |
    Select-Object -First 1
if (-not $dmlSource) {
    Write-Host "ERROR: matching DirectML.dll not found in ort build output" -ForegroundColor Red
    exit 13
}
Remove-Item -LiteralPath $dmlDst -Force -ErrorAction SilentlyContinue
Copy-Item -LiteralPath $dmlSource -Destination $dmlDst
$dmlInfo = Get-Item -LiteralPath $dmlDst
Write-Host "  DirectML.dll: $([math]::Round($dmlInfo.Length / 1MB, 2)) MB ($($dmlInfo.VersionInfo.FileVersion))" -ForegroundColor Green
$info = Get-Item $exe
$sizeMb = [math]::Round($info.Length / 1MB, 2)
Write-Host ""
Write-Host "Release binary built:" -ForegroundColor Green
Write-Host "  Path : $exe"
Write-Host "  Size : $sizeMb MB"
Write-Host "  Built: $($info.LastWriteTime)"

if ($Installer) {
    Write-Host ""
    Write-Host "[build-slint-release] running NSIS installer" -ForegroundColor Cyan
    $candidates = @(
        "C:\Program Files (x86)\NSIS\makensis.exe",
        "C:\Program Files\NSIS\makensis.exe",
        "$env:USERPROFILE\scoop\apps\nsis\current\makensis.exe",
        # Phase E7 — reuse the NSIS the Tauri bundler already downloaded
        # (avoids a separate NSIS install on the build machine).
        "$env:LOCALAPPDATA\tauri\NSIS\makensis.exe",
        "$env:LOCALAPPDATA\tauri\NSIS\Bin\makensis.exe"
    )
    $makensis = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $makensis) {
        Write-Host "ERROR: makensis.exe not found. Install NSIS via:" -ForegroundColor Red
        Write-Host "  scoop install nsis    OR    winget install NSIS.NSIS" -ForegroundColor Yellow
        exit 12
    }
    # Pre-create the bundle output dir so makensis doesn't fail with
    # "opening output file" on first run (review-agent finding 2026-05-27).
    $bundleDir = Join-Path $crate "target\release\bundle"
    New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null
    $nsi = Join-Path $PSScriptRoot "slint-installer.nsi"
    # NOTE: invoke makensis via Start-Process (not the `&` call operator).
    # Under `powershell -File`, `& $makensis ...` left the parser in a state
    # that bound the *next* statement as an argument and threw a bogus
    # SwitchParameter cast error. Start-Process side-steps it entirely.
    $proc = Start-Process -FilePath $makensis -ArgumentList @("/V2", $nsi) -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Host "makensis failed: exit $($proc.ExitCode)" -ForegroundColor Red
        exit $proc.ExitCode
    }
    Write-Host "Installer built: target\release\bundle\suflyor-slint-setup.exe" -ForegroundColor Green
}
