# PreToolUse hook: BLOCK `git commit` / `git push` if the testing layers
# 1-3 of the methodology fail.
#
# Adapted from the vpnctl `.claude/hooks/git-gate.sh` (bash) to PowerShell
# so it matches the existing overlay-mvp hook style. Same contract:
#
#   stdin: JSON like {"tool_input":{"command":"git commit -m foo"}}
#   exit 0 -> allow the Bash tool to run
#   exit 2 -> block with stderr shown to operator
#   git commit -> cargo fmt --check + cargo clippy --all-targets -D warnings
#   git push   -> above + cargo test --lib + cargo test --test copy_contract + npx tsc --noEmit
#   --no-verify in command -> bypass with WARN
#   Not git commit/push -> instant exit 0 (zero overhead)
#   cargo missing -> exit 0 + WARN (don't brick commits in transient envs)
#
# Pipe-test:
#   '{"tool_input":{"command":"git commit -m x"}}' | powershell -NoProfile -ExecutionPolicy Bypass -File .claude/hooks/git-gate.ps1
#
# CRITICAL: the settings watcher does NOT pick up changes mid-session.
# After editing this file or .claude/settings.json, open the /hooks UI
# or restart Claude Code.

$ErrorActionPreference = "Stop"

# --- Read stdin JSON --------------------------------------------------
$raw = [Console]::In.ReadToEnd()
if (-not $raw) { exit 0 }

try {
    $payload = $raw | ConvertFrom-Json
} catch {
    [Console]::Error.WriteLine("[git-gate] WARN: failed to parse stdin JSON; allowing tool call.")
    exit 0
}

$cmd = ""
if ($payload.tool_input -and $payload.tool_input.command) {
    $cmd = [string]$payload.tool_input.command
}

# --- Fast exit if not git commit/push ---------------------------------
$isCommit = $cmd -match '\bgit\s+commit\b'
$isPush   = $cmd -match '\bgit\s+push\b'
if (-not ($isCommit -or $isPush)) {
    exit 0
}

# --- --no-verify bypass ------------------------------------------------
if ($cmd -match '--no-verify') {
    [Console]::Error.WriteLine("[git-gate] WARN: --no-verify bypasses methodology gates.")
    exit 0
}

# --- Locate project root + cargo --------------------------------------
$projectRoot = $PSScriptRoot
while ($projectRoot -and -not (Test-Path (Join-Path $projectRoot "src-tauri\Cargo.toml"))) {
    $parent = Split-Path -Parent $projectRoot
    if ($parent -eq $projectRoot) { break }
    $projectRoot = $parent
}
if (-not (Test-Path (Join-Path $projectRoot "src-tauri\Cargo.toml"))) {
    exit 0
}

$cargoExe = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargoExe)) {
    [Console]::Error.WriteLine("[git-gate] WARN: cargo not at $cargoExe -- gate disabled.")
    exit 0
}

Set-Location $projectRoot
$manifest = "src-tauri/Cargo.toml"

# --- Run a gate command. Uses Start-Process to capture stderr cleanly
# without the PS 5.1 `2>&1` NativeCommandError trap (see CLAUDE.md
# Operational gotchas). stdout+stderr both go to a tempfile, only the
# native exe's exit code is checked. On failure, tail tempfile to stderr.
function Invoke-Gate($name, $exe, $argsList) {
    [Console]::Error.WriteLine("[git-gate] $name ...")
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    try {
        $p = Start-Process -FilePath $exe -ArgumentList $argsList -NoNewWindow `
            -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr `
            -Wait -PassThru
        if ($p.ExitCode -ne 0) {
            [Console]::Error.WriteLine("")
            [Console]::Error.WriteLine("[git-gate] BLOCK: $name failed (exit $($p.ExitCode)).")
            [Console]::Error.WriteLine("[git-gate] Last 30 lines of output:")
            $combined = @()
            if (Test-Path $tmpOut) { $combined += Get-Content $tmpOut }
            if (Test-Path $tmpErr) { $combined += Get-Content $tmpErr }
            ($combined | Select-Object -Last 30) | ForEach-Object {
                [Console]::Error.WriteLine("  $_")
            }
            [Console]::Error.WriteLine("")
            [Console]::Error.WriteLine("[git-gate] Fix the failure and re-commit. Use --no-verify to bypass (NOT recommended).")
            exit 2
        }
    } finally {
        Remove-Item -Force -ErrorAction SilentlyContinue $tmpOut, $tmpErr
    }
}

# --- Always: cargo fmt --check + clippy (both commit and push) -------
Invoke-Gate "cargo fmt --check" $cargoExe @("fmt", "--manifest-path", $manifest, "--all", "--", "--check")
Invoke-Gate "cargo clippy -D warnings" $cargoExe @("clippy", "--manifest-path", $manifest, "--all-targets", "--", "-D", "warnings")

# --- Slint pilot crate (Phase 0+) — added 2026-05-27 ---------------
# Gate the slint-experiment/ sibling crate if it exists. Wrapping in
# Test-Path so a future cleanup/removal of the pilot doesn't brick
# the gate. Same fmt+clippy on commit, plus test on push.
$slintManifest = "slint-experiment/Cargo.toml"
if (Test-Path (Join-Path $projectRoot $slintManifest)) {
    Invoke-Gate "slint fmt --check"        $cargoExe @("fmt", "--manifest-path", $slintManifest, "--all", "--", "--check")
    Invoke-Gate "slint clippy -D warnings" $cargoExe @("clippy", "--manifest-path", $slintManifest, "--all-targets", "--", "-D", "warnings")
}

# --- Push-only: full test suite (commits skip these for speed) -------
if ($isPush) {
    Invoke-Gate "cargo test --lib"                $cargoExe @("test", "--manifest-path", $manifest, "--lib", "--quiet")
    Invoke-Gate "cargo test --test copy_contract" $cargoExe @("test", "--manifest-path", $manifest, "--test", "copy_contract", "--quiet")

    # npx is a .cmd shim — Start-Process handles it the same way.
    Invoke-Gate "npx tsc --noEmit" "npx.cmd" @("tsc", "--noEmit")

    # Slint pilot crate tests — same Test-Path guard.
    if (Test-Path (Join-Path $projectRoot $slintManifest)) {
        Invoke-Gate "slint cargo test" $cargoExe @("test", "--manifest-path", $slintManifest, "--quiet")
    }
}

[Console]::Error.WriteLine("[git-gate] All gating layers green. Proceeding with: $cmd")
exit 0
