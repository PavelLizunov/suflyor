# ort-sys 2.0.0-rc.12 stages DLLs into these directories without creating them.
# Prepare each consumer before Cargo runs; preserve all existing cached files.
[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
)

$ErrorActionPreference = 'Stop'
foreach ($crate in @('overlay-backend', 'slint-experiment')) {
    $crateRoot = Join-Path $RepositoryRoot $crate
    if (-not (Test-Path -LiteralPath $crateRoot -PathType Container)) {
        throw "Missing crate directory: $crate"
    }
    $debugRoot = Join-Path $crateRoot 'target/debug'
    foreach ($directory in @($debugRoot, (Join-Path $debugRoot 'examples'), (Join-Path $debugRoot 'deps'))) {
        [void][System.IO.Directory]::CreateDirectory($directory)
    }
}
