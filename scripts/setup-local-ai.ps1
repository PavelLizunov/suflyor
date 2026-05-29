<#
.SYNOPSIS
  One-shot setup of the LOCAL AI stack for suflyor (overlay-mvp): llama.cpp (LLM),
  whisper.cpp (mixed RU+EN STT) and GigaAM-v3 (Russian STT). Downloads the binaries
  + models, launches the two local servers, and prints the exact Settings values to
  enter in the app.

.DESCRIPTION
  After this finishes you point the app (Settings -> AI / STT) at the local servers:
    AI provider  : Local       URL http://127.0.0.1:8080/v1   model gemma-4-E4B-it-Q4_K_M.gguf
    STT engine   : Local Whisper                URL http://127.0.0.1:8081/v1   (mixed RU+EN)
    STT engine   : Local GigaAM-v3 (Russian)    dir <Root>\gigaam-v3           (best Russian)

  Everything runs on THIS PC; nothing is sent to the cloud. GigaAM runs in-process
  inside the app (no server) once you point the dir at it.

.PARAMETER Root
  Install dir for binaries + models. Default: %USERPROFILE%\suflyor-local-ai

.PARAMETER Cpu
  Force the CPU llama.cpp build even if an NVIDIA GPU is present.

.PARAMETER NoLaunch
  Download + install only; do not start the servers.

.PARAMETER SkipLlama / SkipWhisper / SkipGigaam
  Skip that component.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\setup-local-ai.ps1
#>
[CmdletBinding()]
param(
    [string]$Root = "$env:USERPROFILE\suflyor-local-ai",
    [switch]$Cpu,
    [switch]$NoLaunch,
    [switch]$SkipLlama,
    [switch]$SkipWhisper,
    [switch]$SkipGigaam
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# --- pinned model coordinates (HuggingFace) + exact byte sizes for integrity ---
$GEMMA_URL  = 'https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf'
$GEMMA_FILE = 'gemma-4-E4B-it-Q4_K_M.gguf'
$GEMMA_SIZE = 4977169568

$WHISPER_URL  = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin'
$WHISPER_FILE = 'ggml-large-v3-turbo-q8_0.bin'
$WHISPER_SIZE = 874188075

$GIGAAM_MODEL_URL = 'https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc.int8.onnx'
$GIGAAM_MODEL_SIZE = 224893347
$GIGAAM_VOCAB_URL = 'https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc_vocab.txt'

function Write-Step($msg) { Write-Host "`n=== $msg ===" -ForegroundColor Cyan }

# Resilient HuggingFace download. The Xet CDN resets open-ended GETs; curl -C -
# resumes with a closed range on retry, so we loop until the file reaches the
# known size. GitHub release zips use a normal CDN and need no looping.
function Save-Model([string]$Url, [string]$Out, [long]$ExpectedSize) {
    for ($i = 0; $i -lt 60; $i++) {
        $cur = if (Test-Path $Out) { (Get-Item $Out).Length } else { 0 }
        if ($cur -ge $ExpectedSize) { break }
        Write-Host ("  {0:N0} / {1:N0} bytes ({2:N1}%) - fetching..." -f $cur, $ExpectedSize, ($cur / $ExpectedSize * 100))
        & curl.exe -L --retry 10 --retry-all-errors --retry-delay 2 -C - -o $Out $Url
    }
    $cur = if (Test-Path $Out) { (Get-Item $Out).Length } else { 0 }
    if ($cur -lt $ExpectedSize) { throw "download incomplete: $Out ($cur / $ExpectedSize)" }
    Write-Host ("  OK {0:N0} bytes" -f $cur) -ForegroundColor Green
}

# Download a small file (vocab/json) with retries.
function Save-Small([string]$Url, [string]$Out) {
    & curl.exe -sL --retry 8 --retry-all-errors --retry-delay 2 -o $Out $Url
    if (-not (Test-Path $Out) -or (Get-Item $Out).Length -eq 0) { throw "download failed: $Out" }
}

# Pick a release asset URL by regex from a GitHub repo's latest release.
function Get-ReleaseAsset([string]$Repo, [string]$Pattern) {
    $json = & curl.exe -sL --retry 6 --retry-all-errors --max-time 40 "https://api.github.com/repos/$Repo/releases/latest" | ConvertFrom-Json
    $asset = $json.assets | Where-Object { $_.name -match $Pattern } | Select-Object -First 1
    if (-not $asset) { throw "no asset matching /$Pattern/ in latest $Repo release" }
    return $asset
}

function Get-ZipAsset([string]$Repo, [string]$Pattern, [string]$DestDir) {
    $asset = Get-ReleaseAsset $Repo $Pattern
    $zip = Join-Path $DestDir $asset.name
    Write-Host "  $($asset.name)"
    & curl.exe -L --retry 8 --retry-all-errors -o $zip $asset.browser_download_url
    Expand-Archive -Path $zip -DestinationPath $DestDir -Force
    Remove-Item $zip -Force
}

New-Item -ItemType Directory -Force $Root | Out-Null
$llamaDir   = Join-Path $Root 'llama.cpp'
$whisperDir = Join-Path $Root 'whisper.cpp'
$gigaamDir  = Join-Path $Root 'gigaam-v3'

$hasNvidia = [bool](Get-Command nvidia-smi -ErrorAction SilentlyContinue) -and -not $Cpu
Write-Host "Install root : $Root"
Write-Host ("GPU build    : {0}" -f $(if ($hasNvidia) { 'CUDA (NVIDIA detected)' } else { 'CPU' }))

# ============================== llama.cpp (LLM) ==============================
if (-not $SkipLlama) {
    Write-Step 'llama.cpp + Gemma-4-E4B'
    New-Item -ItemType Directory -Force $llamaDir | Out-Null
    if (-not (Get-ChildItem $llamaDir -Recurse -Filter 'llama-server.exe' -ErrorAction SilentlyContinue)) {
        if ($hasNvidia) {
            # Pin CUDA 12.4 (the release also ships 13.3): the bundled cudart
            # provides the CUDA runtime DLLs, but the NVIDIA *driver* must support
            # the CUDA version. 12.4 has the widest driver support (>= ~550.x), so
            # it is the safe default for a tester box. The matching cudart-*-12.4
            # zip extracts cudart64_12.dll etc. alongside llama-server.exe.
            Get-ZipAsset 'ggml-org/llama.cpp' '^llama-.*-bin-win-cuda-12\.4-x64\.zip$' $llamaDir
            Get-ZipAsset 'ggml-org/llama.cpp' '^cudart-.*-cuda-12\.4-x64\.zip$'       $llamaDir
        } else {
            Get-ZipAsset 'ggml-org/llama.cpp' '^llama-.*-bin-win-cpu-x64\.zip$'       $llamaDir
        }
    } else { Write-Host '  llama-server.exe already present - skipping binary' }
    $gemmaPath = Join-Path $llamaDir $GEMMA_FILE
    Save-Model $GEMMA_URL $gemmaPath $GEMMA_SIZE
}

# ============================== whisper.cpp (STT, mixed RU+EN) ==============================
if (-not $SkipWhisper) {
    Write-Step 'whisper.cpp + Whisper large-v3-turbo'
    New-Item -ItemType Directory -Force $whisperDir | Out-Null
    if (-not (Get-ChildItem $whisperDir -Recurse -Filter '*server.exe' -ErrorAction SilentlyContinue)) {
        # Plain CPU build (the release also ships blas + cublas variants). Whisper
        # large-v3-turbo q8 is small + fast on CPU, so we skip the GPU matrix here.
        Get-ZipAsset 'ggml-org/whisper.cpp' '^whisper-bin-x64\.zip$' $whisperDir
    } else { Write-Host '  whisper-server.exe already present - skipping binary' }
    Save-Model $WHISPER_URL (Join-Path $whisperDir $WHISPER_FILE) $WHISPER_SIZE
}

# ============================== GigaAM-v3 (STT, Russian, in-process) ==============================
if (-not $SkipGigaam) {
    Write-Step 'GigaAM-v3 (Russian STT, runs in-process in the app)'
    New-Item -ItemType Directory -Force $gigaamDir | Out-Null
    Save-Model $GIGAAM_MODEL_URL (Join-Path $gigaamDir 'model.int8.onnx') $GIGAAM_MODEL_SIZE
    Save-Small $GIGAAM_VOCAB_URL (Join-Path $gigaamDir 'vocab.txt')
    Write-Host '  GigaAM model.int8.onnx + vocab.txt ready' -ForegroundColor Green
}

# ============================== launch servers ==============================
if (-not $NoLaunch) {
    if (-not $SkipLlama) {
        Write-Step 'Starting llama-server on :8080'
        $srv = Get-ChildItem $llamaDir -Recurse -Filter 'llama-server.exe' | Select-Object -First 1
        $ngl = if ($hasNvidia) { '99' } else { '0' }
        Start-Process -FilePath $srv.FullName -WindowStyle Hidden -ArgumentList @(
            '-m', (Join-Path $llamaDir $GEMMA_FILE), '--host', '127.0.0.1', '--port', '8080',
            '-ngl', $ngl, '-c', '8192', '--jinja')
        Write-Host '  llama-server launching (model load takes a few seconds)'
    }
    if (-not $SkipWhisper) {
        Write-Step 'Starting whisper-server on :8081'
        $wsrv = Get-ChildItem $whisperDir -Recurse -Filter 'whisper-server.exe' | Select-Object -First 1
        Start-Process -FilePath $wsrv.FullName -WindowStyle Hidden -ArgumentList @(
            '-m', (Join-Path $whisperDir $WHISPER_FILE), '--host', '127.0.0.1', '--port', '8081',
            '--inference-path', '/v1/audio/transcriptions')
        Write-Host '  whisper-server launching'
    }
}

# ============================== done - Settings values ==============================
Write-Step 'DONE - enter these in the app (gear icon)'
Write-Host @"
  AI tab    -> Provider: Local        URL: http://127.0.0.1:8080/v1   model: $GEMMA_FILE
  STT tab   -> Local Whisper          URL: http://127.0.0.1:8081/v1            (mixed RU+EN)
  STT tab   -> Local GigaAM-v3        dir: $gigaamDir   (best Russian, in-process)

  Tip: the bar's active-stack readout shows 'local: ... - ...' when nothing leaves your PC.
  Re-run this script anytime to (re)start the servers; downloads resume / skip if present.
"@ -ForegroundColor Green
