# Stop guard for autonomous mode.
#
# Fires on every Stop event (Claude about to end a turn).
# If .claude/autonomous_active contains a future ISO deadline,
# exit 2 -> block + stderr feeds back as forced continuation.
#
# Hook contract:
#   exit 0 = let Stop proceed (normal end of turn)
#   exit 2 = block Stop; stderr text is appended to Claude's context
#
# stdin payload (JSON): { "stop_hook_active": bool, ... }
#   stop_hook_active=true means we're already in a continuation triggered
#   by our own block -- let that through to avoid infinite loop.

$ErrorActionPreference = "SilentlyContinue"

$projectRoot = "C:/Users/x3d_mutant/Natively/overlay-mvp"
$marker = Join-Path $projectRoot ".claude/autonomous_active"

if (-not (Test-Path $marker)) {
    exit 0
}

$content = (Get-Content $marker -ErrorAction SilentlyContinue | Select-Object -First 1)
if (-not $content) {
    Remove-Item $marker -Force -ErrorAction SilentlyContinue
    exit 0
}

$deadline = $null
try { $deadline = [datetime]::Parse($content.Trim()) } catch {}
if (-not $deadline) {
    Remove-Item $marker -Force -ErrorAction SilentlyContinue
    exit 0
}

$now = Get-Date
if ($now -ge $deadline) {
    Remove-Item $marker -Force -ErrorAction SilentlyContinue
    Write-Host "Autonomous deadline passed -- exiting autonomous mode."
    exit 0
}

# Anti-loop: if hook fired in response to our own previous block,
# don't keep blocking forever.
$stdin = ""
try { $stdin = [Console]::In.ReadToEnd() } catch {}
if ($stdin) {
    try {
        $hookInput = $stdin | ConvertFrom-Json
        if ($hookInput.stop_hook_active -eq $true) {
            exit 0
        }
    } catch {}
}

$remaining = $deadline - $now
$hours = [math]::Floor($remaining.TotalHours)
$mins = $remaining.Minutes

$msg = @"
=== AUTONOMOUS MODE STILL ACTIVE ===
Time left: $hours h $mins m  (until $($deadline.ToString('HH:mm dd-MM-yyyy')))
Rules: .claude/AUTONOMOUS_RULES.md

YOU MAY NOT STOP. Do all of the following without acknowledgement:

1. If NIGHT_RUN_PLAN.md has no '## Backlog' section, create one with
   at least 5 prioritized tasks based on the project's pending work.
2. Move the current '## In progress' item (if any) to '## Done log'
   with a one-line outcome + timestamp.
3. Take the top item from '## Backlog', set it as '## In progress',
   and start executing TOOL CALLS for it NOW.
4. Do not write a 'summary' or 'next steps'. The plan file is the only
   place for status. The chat is for tool use.

Forbidden in any output: 'next session', 'morning brief', 'morning summary',
'defer', 'let me know if'. The banned-phrases hook will refuse any
Write/Edit containing these.

Begin the next tool call now.
"@

[Console]::Error.WriteLine($msg)
exit 2
