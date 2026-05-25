# PreToolUse hook for AskUserQuestion: when an autonomous run is active,
# BLOCK the call and force Claude to decide on its own. Aligns with R6
# ("Decisions on the spot — never ask the user a technical question during
# the autonomous run").
#
# Stop hook continues to enforce R1; this hook closes the loophole where
# Claude tries to ask the user instead of just stopping.
#
# Returns exit 2 with a stderr message that Claude is required to read.

$ErrorActionPreference = 'Stop'
$marker = "C:/Users/x3d_mutant/Natively/overlay-mvp/.claude/autonomous_active"

if (-not (Test-Path $marker)) {
  exit 0  # no active run, ask freely
}

try {
  $deadlineStr = (Get-Content $marker -Raw).Trim()
  $deadline = [DateTime]::Parse($deadlineStr)
  $now = Get-Date
  if ($now -ge $deadline) {
    exit 0  # marker is stale
  }
} catch {
  exit 0  # malformed marker, fail-open
}

# Active autonomous run + AskUserQuestion attempt = R6 violation.
$msg = @"
[R6 VIOLATION] AskUserQuestion blocked. Autonomous run is active until $deadlineStr.

During autonomous mode you MUST decide on the spot. The user explicitly does
not want to be asked technical implementation questions. Possible scenarios
you may have been about to ask:

  - "Should I enable X?"     -> decide YES/NO based on what gives the best
                                 test/result and document the choice in
                                 Decisions log of NIGHT_RUN_PLAN.md.
  - "Do A first or B first?" -> pick the one that unblocks the bigger goal.
  - "What value for X?"      -> pick a sensible default and note it as a
                                 reversible decision.
  - "Is this OK?"            -> ship + verify yourself, then summarize.

If genuinely catastrophic risk (data loss, $$$ spend, irrevocable) THEN ask.
Otherwise: decide, log, continue. Read .claude/AUTONOMOUS_RULES.md R6.
"@

[Console]::Error.WriteLine($msg)
exit 2
