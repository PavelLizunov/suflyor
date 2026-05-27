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
        "$env:USERPROFILE\scoop\apps\nsis\current\makensis.exe"
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
    & $makensis "/V2" $nsi
    if ($LASTEXITCODE -ne 0) {
        Write-Host "makensis failed: exit $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    $installer = Join-Path $crate "target\release\bundle\suflyor-slint-setup.exe"
    if (Test-Path $installer) {
        $iInfo = Get-Item $installer
        Write-Host ""
        Write-Host "Installer built:" -ForegroundColor Green
        Write-Host "  Path : $installer"
        Write-Host "  Size : $([math]::Round($iInfo.Length / 1MB, 2)) MB"
    }
}
