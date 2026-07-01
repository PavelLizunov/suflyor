# PreToolUse hook: BLOCK `git commit` / `git push` if the testing layers
# 1-3 of the methodology fail.
#
# Adapted from the vpnctl `.claude/hooks/git-gate.sh` (bash) to PowerShell
# so it matches the existing overlay-mvp hook style. Same contract:
#
#   stdin: JSON like {"tool_input":{"command":"git commit -m foo"}}
#   exit 0 -> allow the Bash tool to run
#   exit 2 -> block with stderr shown to operator
#   git commit/push -> for slint-experiment + overlay-backend:
#     cargo fmt --check + cargo clippy --all-targets -D warnings + test --lib
#   (Phase 7 cut: src-tauri/React gates + npx tsc removed with the stack.)
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

# --- Retest-HTML gate (golden rule) -----------------------------------
# BLOCK `gh release create vX.Y.Z` when there is no tester checklist
# docs/retest-*X.Y.Z*.html. Every release ships a FILLABLE self-contained HTML
# checklist (memory: always-html-test-report). Defensive: any error here ALLOWS
# the release (a hook bug must never brick a release).
try {
    if ($cmd -match 'gh\s+release\s+create\s+v?(\d+\.\d+\.\d+)') {
        $ver = $Matches[1]
        $pr = $PSScriptRoot
        while ($pr -and -not (Test-Path (Join-Path $pr "overlay-backend\Cargo.toml"))) {
            $parent = Split-Path -Parent $pr
            if ($parent -eq $pr) { break }
            $pr = $parent
        }
        $docs = Join-Path $pr "docs"
        $hits = @()
        if (Test-Path $docs) {
            $hits = @(Get-ChildItem -Path $docs -Filter "retest-*$ver*.html" -File -ErrorAction SilentlyContinue)
        }
        if ($hits.Count -eq 0) {
            [Console]::Error.WriteLine("")
            [Console]::Error.WriteLine("[git-gate] BLOCK: releasing v$ver but docs/retest-*$ver*.html is missing.")
            [Console]::Error.WriteLine("[git-gate] Golden rule: every release ships a fillable self-contained HTML tester checklist.")
            [Console]::Error.WriteLine("[git-gate] Create docs/retest-v$ver-fixes.html (copy docs/retest-template.html), then re-run.")
            exit 2
        }
        [Console]::Error.WriteLine("[git-gate] retest checklist present for v$ver ($($hits[0].Name)) -- release allowed.")
        exit 0
    }
} catch {
    [Console]::Error.WriteLine("[git-gate] WARN: retest-gate errored ($_); allowing release.")
    exit 0
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
# Phase 7 cut: root marker is now overlay-backend/Cargo.toml (src-tauri
# was the marker before the React/Tauri stack was removed).
$projectRoot = $PSScriptRoot
while ($projectRoot -and -not (Test-Path (Join-Path $projectRoot "overlay-backend\Cargo.toml"))) {
    $parent = Split-Path -Parent $projectRoot
    if ($parent -eq $projectRoot) { break }
    $projectRoot = $parent
}
if (-not (Test-Path (Join-Path $projectRoot "overlay-backend\Cargo.toml"))) {
    exit 0
}

$cargoExe = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargoExe)) {
    [Console]::Error.WriteLine("[git-gate] WARN: cargo not at $cargoExe -- gate disabled.")
    exit 0
}

Set-Location $projectRoot

# Disk hygiene (added 2026-06-19): this gate runs `clippy --all-targets` +
# `cargo test` on BOTH crates on every commit/push. clippy --all-targets
# compiles every bin/example/test target (overlay_host, slint_replay,
# markdown_spike, overlay_spike, ...), each spawning its own
# target/debug/incremental/<crate>-<hash> dir — and a FRESH hash whenever the
# build context shifts. cargo's GC only prunes sessions WITHIN a live dir, never
# the orphaned per-hash dirs, so they snowballed to 281 GB (943 dirs) by
# 2026-06-19. The gate is not an interactive edit loop, so incremental buys it
# nothing — disable it for gate builds. Interactive `cargo run` (no env set)
# still uses incremental for fast rebuilds.
$env:CARGO_INCREMENTAL = "0"

# Memory hygiene (2026-06-23): also cap parallel rustc jobs. A COLD gate build
# (toolchain bump / cargo clean / target-dir GC empties the cache) codegens the
# 4 heavy Slint bins at once and OOMs at the default job count (rustc-LLVM out
# of memory) — which would BLOCK the commit. -j2 fits. NB: like every change to
# this file, it only takes effect after a Claude Code restart (the settings
# watcher snapshots hooks at session start). Warm cache = no compile = no cost.
$env:CARGO_BUILD_JOBS = "2"

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

# --- Always (on commit + push): fmt + clippy + fast tests ------------
# (Phase 7 cut: the src-tauri/React legacy gates were removed with the
# stack. The Slint binary + overlay-backend are now the whole product.)

# Slint pilot crate (Phase 0+) — added 2026-05-27.
# Phase E6 v18 update (2026-05-27 evening): move tests from push-only
# to commit too. The 12-test slint-experiment lib suite runs in <1s
# so the cost of gating every commit is negligible vs catching a
# Slint regression early. User feedback: "Slint баги проходят без
# проблем" — that's because the hook only checked fmt+clippy on
# commit, full suite on push, and many commits never get pushed.
$slintManifest = "slint-experiment/Cargo.toml"
if (Test-Path (Join-Path $projectRoot $slintManifest)) {
    Invoke-Gate "slint fmt --check"        $cargoExe @("fmt", "--manifest-path", $slintManifest, "--all", "--", "--check")
    Invoke-Gate "slint clippy -D warnings" $cargoExe @("clippy", "--manifest-path", $slintManifest, "--all-targets", "--", "-D", "warnings")
    # NOT --lib: --lib silently SKIPS everything under tests/, so the i18n_guard
    # (and any future guard test) never ran in the gate despite the docs claiming
    # it did. Full `cargo test` runs lib + bins + integration tests/. (audit G2)
    Invoke-Gate "slint cargo test"         $cargoExe @("test", "--manifest-path", $slintManifest, "--quiet")
}

# overlay-backend (Phase B1) — extracted shared business logic crate.
# Was previously NOT gated at all (added 2026-05-27 evening). 136-test
# lib suite runs in ~3s. Critical because most of the AI / detector /
# config / runtime logic lives here; both src-tauri and slint-experiment
# binaries depend on it.
$backendManifest = "overlay-backend/Cargo.toml"
if (Test-Path (Join-Path $projectRoot $backendManifest)) {
    Invoke-Gate "backend fmt --check"        $cargoExe @("fmt", "--manifest-path", $backendManifest, "--all", "--", "--check")
    Invoke-Gate "backend clippy -D warnings" $cargoExe @("clippy", "--manifest-path", $backendManifest, "--all-targets", "--", "-D", "warnings")
    # NOT --lib — run the integration tests (tests/archive_cycle.rs) too. (audit G2)
    Invoke-Gate "backend cargo test"         $cargoExe @("test", "--manifest-path", $backendManifest, "--quiet")
}

# suflyor-tts (read-aloud sidecar — SHIPPED in the installer; declares deny-lints
# that were never enforced before this). Build into the SHARED slint target dir
# so the cached sherpa-onnx native lib is reused: a cold suflyor-tts/target build
# re-DOWNLOADS sherpa from GitHub, and a flaky network would then BLOCK the
# commit. (audit, v0.22.x — takes effect next Claude session: settings watcher
# doesn't reload hooks mid-session.)
$ttsManifest = "suflyor-tts/Cargo.toml"
if (Test-Path (Join-Path $projectRoot $ttsManifest)) {
    $env:CARGO_TARGET_DIR = Join-Path $projectRoot "slint-experiment\target"
    Invoke-Gate "tts fmt --check"        $cargoExe @("fmt", "--manifest-path", $ttsManifest, "--all", "--", "--check")
    Invoke-Gate "tts clippy -D warnings" $cargoExe @("clippy", "--manifest-path", $ttsManifest, "--all-targets", "--", "-D", "warnings")
    Invoke-Gate "tts cargo test"         $cargoExe @("test", "--manifest-path", $ttsManifest, "--quiet")
    Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
}

# (Phase 7 cut: the push-only block ran src-tauri integration tests +
# `npx tsc --noEmit` for the React stack — both removed. The slint +
# overlay-backend suites above run on every commit AND push.)

[Console]::Error.WriteLine("[git-gate] All gating layers green. Proceeding with: $cmd")
exit 0
