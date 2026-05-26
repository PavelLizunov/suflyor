# Visual gate — layer 6 of the methodology.
#
# Adapted from vpnctl's scripts/visual_check.py (which uses headless Chrome
# over CDP). overlay-mvp lives in a Windows desktop window, so we use
# Win32 BitBlt to grab the primary display instead.
#
# Usage:
#   pwsh scripts/visual_check.ps1            # screenshot the currently running overlay
#   pwsh scripts/visual_check.ps1 -Install   # also: kill running, install latest NSIS, relaunch
#   pwsh scripts/visual_check.ps1 -KeepOpen  # don't kill overlay-mvp on exit
#
# The OUTPUT is the PNG path. In a Claude session, after this runs:
#   `Read C:\...\overlay-mvp\target\visual\overlay-{timestamp}.png`
# That Read is what gives Claude eyeballs on the actual pixels — without
# it, this script is just bookkeeping.

param(
    [switch]$Install,
    [switch]$KeepOpen
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$exePath = "$env:LOCALAPPDATA\suflyor\overlay-mvp.exe"
$visualDir = Join-Path $projectRoot "target\visual"
New-Item -ItemType Directory -Force -Path $visualDir | Out-Null

# Step 1 — optionally kill + reinstall
if ($Install) {
    Write-Host "Killing any running overlay-mvp..." -ForegroundColor Cyan
    Get-Process overlay-mvp -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 600

    $nsisDir = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis"
    $latest = Get-ChildItem -Path $nsisDir -Filter "suflyor_*_x64-setup.exe" -ErrorAction SilentlyContinue `
        | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $latest) {
        Write-Host "ERROR: no NSIS installer found in $nsisDir. Run 'npm run tauri build -- --bundles nsis' first." -ForegroundColor Red
        exit 11
    }
    Write-Host "Installing $($latest.Name)..." -ForegroundColor Cyan
    Start-Process -FilePath $latest.FullName -ArgumentList "/S" -Wait
    Start-Sleep -Milliseconds 1000

    if (-not (Test-Path $exePath)) {
        Write-Host "ERROR: installer ran but $exePath missing." -ForegroundColor Red
        exit 12
    }
    $exeInfo = Get-Item $exePath
    Write-Host "Installed: timestamp=$($exeInfo.LastWriteTime) size=$($exeInfo.Length)" -ForegroundColor Green
}

# Step 2 — make sure overlay-mvp is running (launch if not)
$proc = Get-Process overlay-mvp -ErrorAction SilentlyContinue
if (-not $proc) {
    if (-not (Test-Path $exePath)) {
        Write-Host "ERROR: $exePath does not exist. Run with -Install first." -ForegroundColor Red
        exit 13
    }
    Write-Host "Launching overlay-mvp..." -ForegroundColor Cyan
    Start-Process -FilePath $exePath
    Start-Sleep -Seconds 2
}

# Step 3 — wait for WebView2 to paint
Write-Host "Waiting 2s for WebView2 paint..." -ForegroundColor Cyan
Start-Sleep -Seconds 2

# Step 4 — capture primary display via Win32 BitBlt
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outPath = Join-Path $visualDir "overlay-$timestamp.png"
$bitmap.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

Write-Host ""
Write-Host "Screenshot saved: $outPath" -ForegroundColor Green
Write-Host "Size: $((Get-Item $outPath).Length) bytes ($($bounds.Width)x$($bounds.Height))" -ForegroundColor Green

# Step 5 — optionally clean up
if (-not $KeepOpen) {
    Write-Host "Killing overlay-mvp (-KeepOpen to skip)..." -ForegroundColor Cyan
    Get-Process overlay-mvp -ErrorAction SilentlyContinue | Stop-Process -Force
}

# Emit the path on the last line so callers can capture it
Write-Output $outPath
