---
description: Start autonomous mode for N hours with hook enforcement (R1-R10)
---

Activate autonomous mode for the duration in $ARGUMENTS.

Accepted: `6h`, `30m`, `90 min`, `4 hours`, `until 08:00`, `until 14:30`.
Default if $ARGUMENTS is empty or unparseable: `6h`.

Steps to perform without acknowledgement:

1. Parse $ARGUMENTS into a future deadline (ISO 8601 local time).
   If "until HH:MM", that's today at HH:MM (or tomorrow if past).

2. Write the ISO deadline string to
   `C:/Users/x3d_mutant/Natively/overlay-mvp/.claude/autonomous_active`
   (single line, overwrite). Example content:
       2026-05-25T14:30:00

3. Reset progress counter:
   write `0` to `C:/Users/x3d_mutant/Natively/overlay-mvp/.claude/_progress_counter`.

4. Read `C:/Users/x3d_mutant/Natively/overlay-mvp/NIGHT_RUN_PLAN.md`.
   - If no `## Backlog` section exists, create one. Populate it with at
     least 5 prioritized concrete tasks pulled from the project's
     pending/incomplete state (look at: undone S1 findings, deferred
     features from prior morning briefs, untested code paths, pending
     UX bugs, in-progress task list).
   - Add a `## In progress` line for whatever you start first.

5. Print exactly this and nothing else:
       AUTONOMOUS MODE ACTIVE until <deadline>.
       Backlog has <N> items. Starting #1: <title>.
       Stop hook armed. Rules R1-R10 in .claude/AUTONOMOUS_RULES.md.

6. Immediately execute tool calls for backlog item #1.
   Do NOT ask "shall I start?" — start.

Hook behaviour from this point:
- `Stop` hook will exit 2 with continuation prompt if you try to end
- `PreToolUse` on Write/Edit refuses banned-phrase content
- `PostToolUse` on Write/Edit forces plan update every 30 ops

To deactivate before deadline: `/auto-stop`
