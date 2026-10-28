# Stage the hash-pinned native Nemotron diarization runtime, isolated from ONNX.
param(
    [string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'slint-experiment\target\release\nemotron'),
    [string]$CacheDir = (Join-Path (Split-Path -Parent $PSScriptRoot) 'slint-experiment\target\nemotron-cache')
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$url = 'https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.2.0/nemo-speech-0.2.0-windows-x86_64-cuda.zip'
$expectedBytes = 102183465
$expectedHash = '9ffe5e842c39a7d3211f3b2c3b3aa1bca8c0b2455d96d961ed5f22ec5108f98d'
New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null
$archive = Join-Path $CacheDir 'nemo-speech-0.2.0-windows-x86_64-cuda.zip'
if (-not (Test-Path -LiteralPath $archive)) {
    $partial = "$archive.download"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $partial -TimeoutSec 600
    Move-Item -LiteralPath $partial -Destination $archive -Force
}
if ((Get-Item -LiteralPath $archive).Length -ne $expectedBytes -or
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) {
    Remove-Item -LiteralPath $archive -Force
    throw 'Nemotron archive does not match the pinned release; run setup again to download it.'
}
# Re-extract even when a destination exists: its presence does not verify its bytes.
$stage = Join-Path $CacheDir ([Guid]::NewGuid().ToString('N'))
try {
    Expand-Archive -LiteralPath $archive -DestinationPath $stage
    $source = Join-Path $stage 'nemo-speech-0.2.0-windows-x86_64-cuda'
    foreach ($file in @('bin\nemo-speech.exe', 'bin\ggml-cpu.dll', 'bin\ggml-cuda.dll', 'share\licenses\nemo-speech\LICENSE')) {
        if (-not (Test-Path -LiteralPath (Join-Path $source $file))) { throw "Nemotron archive is missing $file" }
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
    if (Test-Path -LiteralPath $Destination) { Remove-Item -LiteralPath $Destination -Recurse -Force }
    Move-Item -LiteralPath $source -Destination $Destination
    Write-Host "Staged verified Nemotron CUDA/CPU runtime: $Destination"
} finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
}
