# Capture the primary monitor to a PNG so the agent can Read it.
# DPI-aware so the capture matches physical pixels (user runs 125% scale).
# Usage: powershell -File capture_primary.ps1 -Out C:\path\shot.png
param([string]$Out = "C:\Users\x3d_mutant\Natively\overlay-mvp\slint-experiment\verify.png")

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DPIAware {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[void][DPIAware]::SetProcessDPIAware()

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$scr = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($scr.Width, $scr.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($scr.X, $scr.Y, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved $Out ($($scr.Width)x$($scr.Height))"
