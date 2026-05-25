# PostToolUse on Write|Edit — counts file ops since last NIGHT_RUN_PLAN
# update. At 30 ops without a plan touch, forces a reminder via exit 2.
#
# Hook contract:
#   exit 0 = silent ok
#   exit 2 = reminder text added to Claude's context
#
# stdin (JSON): { "tool_name": ..., "tool_input": { "file_path": ... }, ... }

$ErrorActionPreference = "SilentlyContinue"

$projectRoot = "C:/Users/x3d_mutant/Natively/overlay-mvp"
$marker = Join-Path $projectRoot ".claude/autonomous_active"

if (-not (Test-Path $marker)) { exit 0 }  # only enforce while autonomous

$counterFile = Join-Path $projectRoot ".claude/_progress_counter"
$planFile = Join-Path $projectRoot "NIGHT_RUN_PLAN.md"

$stdin = ""
try { $stdin = [Console]::In.ReadToEnd() } catch {}
if (-not $stdin) { exit 0 }

$payload = $null
try { $payload = $stdin | ConvertFrom-Json } catch { exit 0 }

$filePath = ""
if ($payload.tool_input.file_path) { $filePath = $payload.tool_input.file_path }

# If THIS edit was the plan file, reset counter and exit silently.
if ($filePath -like "*NIGHT_RUN_PLAN.md") {
    Set-Content $counterFile "0" -Force
    exit 0
}

# Otherwise increment counter.
$n = 0
if (Test-Path $counterFile) {
    try { $n = [int]((Get-Content $counterFile -ErrorAction SilentlyContinue | Select-Object -First 1)) } catch { $n = 0 }
}
$n++
Set-Content $counterFile $n -Force

if ($n -ge 30) {
    Set-Content $counterFile "0" -Force
    $msg = @"
=== R7 ENFORCEMENT — 30 file ops without plan update ===
You have written/edited 30 files since the last NIGHT_RUN_PLAN.md update.
Rule R7 requires an incremental log entry every ~30 ops.

Before your next other action:
  1. Open NIGHT_RUN_PLAN.md
  2. Add a one-line entry to '## Done log' for what just finished
  3. If your '## In progress' target has shifted, update it
  4. Save the file (this resets the counter automatically)

Then continue with whatever you were about to do.
"@
    [Console]::Error.WriteLine($msg)
    exit 2
}

exit 0
