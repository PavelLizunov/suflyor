param(
    [Parameter(Mandatory = $true)]
    [string]$InstallDir,
    [switch]$CheckOnly
)

$ErrorActionPreference = 'Stop'
$targetNames = @('overlay-host', 'suflyor-tts', 'suflyor-teratts')
$targets = @{}
foreach ($name in $targetNames) {
    $targets[[IO.Path]::GetFullPath((Join-Path $InstallDir "$name.exe"))] = $true
}

function Get-InstalledSuflyorProcess {
    $matches = @()
    foreach ($name in $targetNames) {
        foreach ($process in @(Get-Process -Name $name -ErrorAction SilentlyContinue)) {
            try {
                $path = [IO.Path]::GetFullPath($process.Path)
                if ($targets.ContainsKey($path)) {
                    $matches += $process
                }
            } catch {
                # A process whose executable path cannot be proven is never ours.
            }
        }
    }
    return $matches
}

$running = @(Get-InstalledSuflyorProcess)
if ($CheckOnly) {
    if ($running.Count -gt 0) { exit 10 }
    exit 0
}

foreach ($process in $running) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
}

$deadline = [DateTime]::UtcNow.AddSeconds(5)
do {
    if (@(Get-InstalledSuflyorProcess).Count -eq 0) { exit 0 }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)

exit 11
