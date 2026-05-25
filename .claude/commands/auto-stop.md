---
description: Emergency exit from autonomous mode (removes hook enforcement)
---

Deactivate autonomous mode. Use only when the user explicitly wants to
end early or shift to interactive discussion.

Steps:

1. Delete `C:/Users/x3d_mutant/Natively/overlay-mvp/.claude/autonomous_active`
   (use Bash `rm -f` since Edit/Write on this path are whitelisted).

2. Delete `C:/Users/x3d_mutant/Natively/overlay-mvp/.claude/_progress_counter`.

3. Add a one-line entry to `NIGHT_RUN_PLAN.md` `## Done log`:
       <ISO timestamp> — autonomous mode manually ended via /auto-stop

4. Print exactly:
       Autonomous mode disabled. Hooks no longer enforce R1-R10.

5. Wait for user input. Do NOT keep working unless instructed.

Note: the scheduled-tasks backup pinger (B-mode) is INDEPENDENT of
this — it will keep firing until the user disables it via
`mcp__scheduled-tasks__update_scheduled_task` with `enabled: false`.
