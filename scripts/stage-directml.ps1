# Copies ort's matching DirectML.dll beside a crate's test executables.
# Rust test executables live under target/debug/deps. The statically linked
# DirectML provider imports DMLCreateDevice1 before main; older Windows builds
# have a system DirectML.dll without that export, so the tests exit with
# 0xc0000138 before any test runs unless the redistributable is staged.
# Shared by scripts/ci.ps1 and scripts/git-gate-native.ps1.
param(
    [Parameter(Mandatory = $true)]
    [string]$CrateDir,
    # Fail when the build produced no DirectML.dll (crates that link ort).
    [switch]$Required
)

$ErrorActionPreference = 'Stop'
$source = Join-Path $CrateDir 'target\debug\DirectML.dll'
if (-not (Test-Path -LiteralPath $source)) {
    if ($Required) {
        Write-Host "matching DirectML.dll not found at $source" -ForegroundColor Red
        exit 1
    }
    exit 0
}
$destination = Join-Path $CrateDir 'target\debug\deps\DirectML.dll'
$alreadyMatching = (Test-Path -LiteralPath $destination) -and
    ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -eq
     (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash)
if (-not $alreadyMatching) {
    Copy-Item -LiteralPath $source -Destination $destination -Force
}
exit 0
