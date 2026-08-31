# Agent-agnostic selective git gate used by .githooks/pre-commit and pre-push.
# Normal development and prereleases use docs/targeted checks. Full CI is
# explicit and reserved for publishing an owner-authorized stable release:
#   powershell -File scripts/git-gate-native.ps1 push -Full
param(
    [ValidateSet('commit', 'push', 'manual', 'classify')]
    [string]$Stage = 'commit',
    [string]$Base = 'origin/master',
    [switch]$Full,
    [switch]$ListOnly
)

$ErrorActionPreference = 'Stop'
$env:CARGO_INCREMENTAL = '0'
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '2' }
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = 'cargo' }

$crateOrder = @(
    'overlay-backend',
    'slint-experiment',
    'suflyor-tts',
    'suflyor-teratts',
    'suflyor-wsola'
)

function Invoke-Git([string[]]$Arguments) {
    $result = @(& git.exe @Arguments)
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed"
    }
    return $result
}

$diffArguments = @()
$includeUntracked = $false
if ($Stage -eq 'commit') {
    $diffArguments = @('--cached')
}
elseif ($Stage -eq 'push') {
    & git.exe rev-parse --verify $Base *> $null
    if ($LASTEXITCODE -ne 0) { $Base = 'HEAD~1' }
    $diffArguments = @("$Base...HEAD")
}
else {
    & git.exe rev-parse --verify $Base *> $null
    if ($LASTEXITCODE -ne 0) { $Base = 'HEAD' }
    # Manual/classify includes staged and unstaged working-tree changes.
    $diffArguments = @($Base)
    $includeUntracked = $true
}

$changedInput = @(Invoke-Git (@('diff') + $diffArguments + @('--name-only', '--diff-filter=ACMRD')))
if ($includeUntracked) {
    $changedInput += @(Invoke-Git @('ls-files', '--others', '--exclude-standard'))
}
$changed = @($changedInput |
    ForEach-Object { $_.Replace('\', '/') } |
    Where-Object { $_ } |
    Sort-Object -Unique)

if ($changed.Count -eq 0) {
    Write-Host "[gate:$Stage] no changed files"
    exit 0
}

$affectedCrates = @($crateOrder | Where-Object {
    $prefix = "$_/"
    @($changed | Where-Object { $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) }).Count -gt 0
})

$docsOnly = @($changed | Where-Object {
    $_ -notmatch '(^|/)[^/]+\.(md|html|txt)$'
}).Count -eq 0

$tier = 'targeted'
if ($docsOnly) { $tier = 'docs' }
if ($Full) { $tier = 'full' }

Write-Host "[gate:$Stage] tier=$tier files=$($changed.Count) crates=$($affectedCrates -join ',')"
if ($Full) {
    Write-Host "[gate:$Stage] full reason: explicit stable-release gate"
}
$changed | ForEach-Object { Write-Host "[gate:$Stage]   $_" }
if ($ListOnly -or $Stage -eq 'classify') { exit 0 }

function Run([string]$Label, [scriptblock]$Command) {
    Write-Host "[gate:$Stage] $Label"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[gate:$Stage] FAIL: $Label" -ForegroundColor Red
        exit 1
    }
}

function Check-PowerShellSyntax {
    foreach ($path in @($changed | Where-Object { $_ -like '*.ps1' })) {
        $fullPath = Join-Path $root $path
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) { continue }
        $tokens = $null
        $errors = $null
        [void][System.Management.Automation.Language.Parser]::ParseFile(
            $fullPath,
            [ref]$tokens,
            [ref]$errors
        )
        if ($errors.Count -gt 0) {
            $errors | ForEach-Object { Write-Host "[gate:$Stage] ${path}: $($_.Message)" -ForegroundColor Red }
            exit 1
        }
    }
}

Run 'git diff --check' {
    & git.exe diff @diffArguments --check
}
Check-PowerShellSyntax

if ($Stage -eq 'commit') {
    foreach ($crate in $affectedCrates) {
        $manifest = Join-Path $root "$crate\Cargo.toml"
        Run "$crate fmt --check" {
            & $cargo fmt --manifest-path $manifest --all -- --check
        }
    }
    Write-Host "[gate:$Stage] OK ($tier)" -ForegroundColor Green
    exit 0
}

if ($tier -eq 'docs') {
    Write-Host "[gate:$Stage] OK (docs-only; no Cargo work)" -ForegroundColor Green
    exit 0
}

if ($tier -eq 'full') {
    Run 'full scripts/ci.ps1' {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\ci.ps1')
    }
    Write-Host "[gate:$Stage] OK (full)" -ForegroundColor Green
    exit 0
}

foreach ($crate in $affectedCrates) {
    $manifest = Join-Path $root "$crate\Cargo.toml"
    Run "$crate fmt --check" {
        & $cargo fmt --manifest-path $manifest --all -- --check
    }

    $crateFiles = @($changed | Where-Object { $_.StartsWith("$crate/", [StringComparison]::OrdinalIgnoreCase) })
    $slintUiOnly = $crate -eq 'slint-experiment' -and
        @($crateFiles | Where-Object { $_ -match '\.(rs|toml|lock)$' -or $_ -match '/build\.rs$' }).Count -eq 0

    if ($slintUiOnly) {
        Run 'slint UI compile check' {
            & $cargo check --locked --manifest-path $manifest --bin overlay-host
        }
        $guards = @(
            'codex_copy_guard', 'i18n_guard', 'icon_guard',
            'lock_chip_geometry_guard', 'lock_chip_layout_guard',
            'lock_mode_menu_guard', 'macos_settings_guard',
            'rc3_regression_guard', 'settings_reset_guard',
            'tera_tts_layout_guard', 'tile_player_layout_guard',
            'tray_guard', 'version_guard'
        )
        $guardArgs = @('test', '--locked', '--manifest-path', $manifest)
        foreach ($guard in $guards) { $guardArgs += @('--test', $guard) }
        Run 'slint static guard tests' {
            & $cargo @guardArgs
        }
    }
    else {
        Run "$crate clippy" {
            & $cargo clippy --manifest-path $manifest --all-targets -- -D warnings
        }
        Run "$crate test" {
            & $cargo test --manifest-path $manifest
        }
    }
}

Write-Host "[gate:$Stage] OK (targeted)" -ForegroundColor Green
exit 0
