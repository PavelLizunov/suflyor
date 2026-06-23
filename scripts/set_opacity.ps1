param([double]$Value = 0.5)
$path = Join-Path $env:APPDATA "suflyor\config.json"
if (-not (Test-Path $path)) { Write-Output "config not found: $path"; exit 1 }
$json = Get-Content $path -Raw | ConvertFrom-Json
$json.tile_body_opacity = $Value
# Re-serialize. ConvertTo-Json depth 10 to preserve nested arrays.
$json | ConvertTo-Json -Depth 10 | Set-Content $path -Encoding UTF8
Write-Output "tile_body_opacity set to $Value"
