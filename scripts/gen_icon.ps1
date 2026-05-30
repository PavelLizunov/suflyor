# Generates the suflyor app icon from slint-experiment/assets/icon-source.png.
# Resizes the source into a soft rounded square at several sizes via
# System.Drawing, packs them into a multi-image PNG-compressed .ico, and also
# writes a 256px .png for the Slint window icon.
#
# Outputs:
#   slint-experiment/assets/icon.ico   (winres exe embed + NSIS shortcut)
#   slint-experiment/assets/icon.png   (Slint Window `icon` property)
#
# Re-run whenever icon-source.png changes. Run from project root:
#   powershell scripts/gen_icon.ps1

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$assetsDir = Join-Path $projectRoot "slint-experiment\assets"
$srcPath = Join-Path $assetsDir "icon-source.png"
if (-not (Test-Path $srcPath)) { throw "icon-source.png not found in $assetsDir" }
# Load into a fresh bitmap so the file handle is released immediately.
$srcImg = [System.Drawing.Image]::FromFile($srcPath)
$src = New-Object System.Drawing.Bitmap($srcImg)
$srcImg.Dispose()

function New-IconBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    # Soft rounded-square mask so the white plate gets gentle corners. Small
    # sizes use less rounding so the art stays crisp at 16/24 px.
    $rad = if ($size -le 24) { [double]$size * 0.12 } else { [double]$size * 0.20 }
    $d = 2 * $rad
    $w = [double]$size
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc(0, 0, $d, $d, 180, 90)
    $path.AddArc($w - $d, 0, $d, $d, 270, 90)
    $path.AddArc($w - $d, $w - $d, $d, $d, 0, 90)
    $path.AddArc(0, $w - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $g.SetClip($path)

    # Draw the source scaled to fill the square (it already has a white field).
    $rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
    $g.DrawImage($src, $rect)
    $g.ResetClip()

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
$src.Dispose()

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

Write-Host "Wrote icon.ico ($count sizes) + icon.png from icon-source.png to $assetsDir" -ForegroundColor Green
