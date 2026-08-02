param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v\d+\.\d+\.\d+\z')]
    [string]$Tag,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^https://github\.com/PavelLizunov/suflyor/releases/tag/v\d+\.\d+\.\d+\z')]
    [string]$ReleaseUrl
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$readmePath = Join-Path $repoRoot 'README.md'
$readme = [IO.File]::ReadAllText($readmePath)
$pattern = '(?s)<!-- latest-release:start -->.*?<!-- latest-release:end -->'
$replacement = @(
    '<!-- latest-release:start -->'
    "Latest published build: [$Tag]($ReleaseUrl)."
    '<!-- latest-release:end -->'
) -join "`n"

if ([regex]::Matches($readme, $pattern).Count -ne 1) {
    throw 'README.md must contain exactly one latest-release marker block.'
}

$updated = [regex]::Replace($readme, $pattern, $replacement)
if ($updated -cne $readme) {
    [IO.File]::WriteAllText($readmePath, $updated, [Text.UTF8Encoding]::new($false))
    Write-Output "Updated README.md to $Tag."
} else {
    Write-Output "README.md already references $Tag."
}
