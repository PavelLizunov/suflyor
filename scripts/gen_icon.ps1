# Generates the suflyor app icon (no network, no external assets).
# Renders a dark rounded-square with a cyan "S" at several sizes via
# System.Drawing, packs them into a multi-image PNG-compressed .ico,
# and also writes a 256px .png for the Slint window icon.
#
# Outputs:
#   slint-experiment/assets/icon.ico   (winres embed + NSIS shortcut)
#   slint-experiment/assets/icon.png   (Slint Window `icon:` property)
#
# Re-run only when the icon design changes. Run from project root:
#   powershell scripts/gen_icon.ps1

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$assetsDir = Join-Path $projectRoot "slint-experiment\assets"
if (-not (Test-Path $assetsDir)) {
    New-Item -ItemType Directory -Path $assetsDir | Out-Null
}

# Palette (matches the app: dark #14161E bg, #2A2C3A border, #6CF cyan).
$bg = [System.Drawing.Color]::FromArgb(255, 0x16, 0x18, 0x22)
$border = [System.Drawing.Color]::FromArgb(255, 0x2E, 0x4A, 0x5A)
$fg = [System.Drawing.Color]::FromArgb(255, 0x66, 0xCC, 0xFF)

function New-IconBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)

    # Rounded-square plate inset slightly so the corners breathe.
    $inset = [double]$size * 0.06
    $rad = [double]$size * 0.24
    $x = $inset
    $y = $inset
    $w = [double]$size - 2 * $inset
    $h = [double]$size - 2 * $inset
    $d = 2 * $rad
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()

    $brush = New-Object System.Drawing.SolidBrush($bg)
    $g.FillPath($brush, $path)
    $brush.Dispose()

    if ($size -ge 32) {
        $penW = [Math]::Max(1.0, [double]$size * 0.025)
        $pen = New-Object System.Drawing.Pen($border, $penW)
        $g.DrawPath($pen, $path)
        $pen.Dispose()
    }

    # Centered bold "S".
    $fontSize = [double]$size * 0.6
    $font = New-Object System.Drawing.Font("Segoe UI", $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $rect = New-Object System.Drawing.RectangleF(0, [single](-$size * 0.04), [single]$size, [single]$size)
    $textBrush = New-Object System.Drawing.SolidBrush($fg)
    $g.DrawString("S", $font, $textBrush, $rect, $fmt)
    $textBrush.Dispose()
    $font.Dispose()
    $fmt.Dispose()
    $path.Dispose()
    $g.Dispose()
    return $bmp
}

function Get-PngBytes($bmp) {
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $ms.Dispose()
    return ,$bytes
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$pngList = @()
foreach ($s in $sizes) {
    $bmp = New-IconBitmap $s
    $pngList += , (Get-PngBytes $bmp)
    if ($s -eq 256) {
        $bmp.Save((Join-Path $assetsDir "icon.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    }
    $bmp.Dispose()
}

# Assemble the .ico: ICONDIR (6) + N*ICONDIRENTRY (16) + concatenated PNGs.
$ico = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ico)
$count = $sizes.Count
$bw.Write([uint16]0)      # reserved
$bw.Write([uint16]1)      # type = icon
$bw.Write([uint16]$count) # image count

$offset = 6 + (16 * $count)
for ($i = 0; $i -lt $count; $i++) {
    $s = $sizes[$i]
    $len = $pngList[$i].Length
    $dim = if ($s -ge 256) { 0 } else { $s }  # 0 means 256 in the ICO spec
    $bw.Write([byte]$dim)   # width
    $bw.Write([byte]$dim)   # height
    $bw.Write([byte]0)      # palette count
    $bw.Write([byte]0)      # reserved
    $bw.Write([uint16]1)    # color planes
    $bw.Write([uint16]32)   # bits per pixel
    $bw.Write([uint32]$len) # bytes of PNG data
    $bw.Write([uint32]$offset)
    $offset += $len
}
foreach ($png in $pngList) {
    $bw.Write($png)
}
$bw.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $assetsDir "icon.ico"), $ico.ToArray())
$bw.Dispose()
$ico.Dispose()

Write-Host "Wrote icon.ico ($count sizes) + icon.png to $assetsDir" -ForegroundColor Green
