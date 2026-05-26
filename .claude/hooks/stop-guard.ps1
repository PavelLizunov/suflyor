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

# Anti-loop SAFETY RAIL (not bypass).
#
# Old version returned exit 0 the moment stop_hook_active became true,
# which meant the harness was allowed to stop after just ONE block. Live
# regression 2026-05-26 04:41: user reported "автоматический режим снова
# завершился слишком рано" — marathon ran 45 min then silently died. Root
# cause was this bypass.
#
# New version uses a STOP COUNTER:
#   - Each Stop-block increments the counter
#   - If counter exceeds MAX_BLOCKS_PER_HOUR (default 240 = 1 every 15s),
#     we let the next Stop through — interpret as "model is genuinely
#     stuck in a loop, end this poorly". A counter file with a timestamp
#     for rate windowing.
#   - Plan edits reset the counter (productive work continues).
#
# This way one careless "end of turn" message can't disarm the marathon,
# but a runaway loop where Claude immediately re-stops every cycle is
# still bounded.
$stopCountFile = Join-Path $projectRoot ".claude/_stop_count"
$now = Get-Date
$nowUnix = [int]([DateTimeOffset]::Now.ToUnixTimeSeconds())
$windowSecs = 3600
$maxBlocks = 240

$blocksInWindow = 0
$earliestUnix = $nowUnix
if (Test-Path $stopCountFile) {
    # File format: one unix timestamp per line, one per past Stop event.
    $lines = Get-Content $stopCountFile -ErrorAction SilentlyContinue
    if ($lines) {
        $kept = @()
        foreach ($t in $lines) {
            $u = 0
            try { $u = [int]$t } catch { continue }
            if ($u -ge ($nowUnix - $windowSecs)) {
                $kept += $u
            }
        }
        $blocksInWindow = $kept.Count
        if ($kept.Count -gt 0) { $earliestUnix = $kept[0] }
        # Rewrite file with only in-window entries to bound size.
        Set-Content $stopCountFile -Value ($kept -join "`n") -Force
    }
}

# Add this block to the counter file.
Add-Content $stopCountFile -Value "$nowUnix"

if ($blocksInWindow -ge $maxBlocks) {
    Write-Host "Stop guard rate-limit hit: $blocksInWindow blocks in last hour. Letting stop proceed (model likely stuck)."
    exit 0
}

# Read stdin for diagnostics but DO NOT use stop_hook_active as a bypass.
$stdin = ""
try { $stdin = [Console]::In.ReadToEnd() } catch {}

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
