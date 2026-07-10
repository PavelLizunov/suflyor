# Agent-agnostic git gate — called by .githooks/pre-commit ("commit") and
# .githooks/pre-push ("push"). Unlike .claude/hooks/git-gate.ps1 (a Claude
# Code PreToolUse hook that only fires inside Claude sessions), this runs for
# EVERY committer: Codex, other agents, humans. Enable once per clone:
#   git config core.hooksPath .githooks
#
# commit -> cargo fmt --check (3 crates)            (~seconds)
# push   -> clippy -D warnings + tests (3 crates)   (~minutes)
#
# Exit non-zero blocks the git operation.
param([string]$Stage = "commit")

$ErrorActionPreference = 'Stop'
$env:CARGO_INCREMENTAL = '0'
$root = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path $cargo)) { $cargo = 'cargo' }
$crates = @('overlay-backend', 'slint-experiment', 'suflyor-tts')

function Run($label, $argv) {
    Write-Host "[gate:$Stage] $label"
    & $cargo @argv
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[gate:$Stage] FAIL: $label" -ForegroundColor Red
        exit 1
    }
}

foreach ($c in $crates) {
    $m = Join-Path $root "$c\Cargo.toml"
    if ($Stage -eq 'commit') {
        Run "$c fmt --check" @('fmt', '--manifest-path', $m, '--', '--check')
    }
    else {
        Run "$c clippy" @('clippy', '--manifest-path', $m, '--all-targets', '--', '-D', 'warnings')
        Run "$c test" @('test', '--manifest-path', $m)
    }
}
Write-Host "[gate:$Stage] OK" -ForegroundColor Green
exit 0
