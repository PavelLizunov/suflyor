# PreToolUse on Write|Edit — refuses files that contain "punt" phrases
# while autonomous mode is active.
#
# Hook contract:
#   exit 0 = allow tool call
#   exit 2 = block + stderr to Claude
#
# stdin (JSON): { "tool_name": "Write"|"Edit", "tool_input": {...}, ... }

$ErrorActionPreference = "SilentlyContinue"

$projectRoot = "C:/Users/x3d_mutant/Natively/overlay-mvp"
$marker = Join-Path $projectRoot ".claude/autonomous_active"

if (-not (Test-Path $marker)) { exit 0 }  # not autonomous — no enforcement

$stdin = ""
try { $stdin = [Console]::In.ReadToEnd() } catch {}
if (-not $stdin) { exit 0 }

$payload = $null
try { $payload = $stdin | ConvertFrom-Json } catch { exit 0 }

# Identify file_path + content (Write has .content, Edit has .new_string).
$filePath = ""
if ($payload.tool_input.file_path) { $filePath = $payload.tool_input.file_path }

$content = ""
if ($payload.tool_input.content) { $content = $payload.tool_input.content }
elseif ($payload.tool_input.new_string) { $content = $payload.tool_input.new_string }

if (-not $content) { exit 0 }

# Whitelist meta files — they legitimately MENTION the banned phrases
# (this rules doc, the plan doc, the marker, the hook scripts themselves).
$whitelist = @(
    "AUTONOMOUS_RULES.md",
    "autonomous_active",
    "stop-guard.ps1",
    "banned-phrases.ps1",
    "plan-progress-check.ps1",
    "CLAUDE.md",
    ".claude/commands/",
    "NIGHT_RUN_PLAN.md"  # plan file may genuinely log past punts
)
foreach ($wl in $whitelist) {
    if ($filePath -like "*$wl*") { exit 0 }
}

# Also whitelist hook self-deactivation attempts targeting the marker —
# but ONLY via the /auto-stop slash command path (different mechanism).
# Bash/PowerShell tools aren't covered by this hook (Edit/Write only).

$banned = @(
    "next session",
    "next-session",
    "morning brief",
    "morning summary",
    "defer to later",
    "defer to next",
    "deferred to next",
    "deferred to later",
    "let me know when",
    "let me know if you want",
    "let me know if you'd like",
    "tell me which",
    "what would you like next",
    "i'll stop here",
    "i'll end here",
    "до твоего ответа не делаю",
    "жду твоего ответа",
    "не делаю пока",
    "пока я сплю",       # nostalgic — bans referring to the prior failed run
    "проснёшься",
    "проснешься"
)

foreach ($phrase in $banned) {
    if ($content -match [regex]::Escape($phrase)) {
        $shortPath = if ($filePath) { Split-Path $filePath -Leaf } else { "(unknown)" }
        $msg = @"
=== AUTONOMOUS MODE: WRITE/EDIT BLOCKED ===
File: $shortPath
Banned phrase detected: '$phrase'

R6 (decisions on the spot) and R1 (no exits) require you to NOT punt
work via this phrasing. Two options:

  (a) Rewrite the file content WITHOUT the banned phrase. State the
      action as something you are doing now, not deferring.
  (b) If the work was genuinely the next step, just do that work in
      tool calls instead of writing about it.

Then retry the Write/Edit. This hook will not get out of your way.
"@
        [Console]::Error.WriteLine($msg)
        exit 2
    }
}

exit 0
