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
