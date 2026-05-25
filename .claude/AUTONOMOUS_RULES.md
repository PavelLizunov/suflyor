# Autonomous Mode Rules (R1-R10)

Loaded when `.claude/autonomous_active` exists with a future ISO deadline.
Enforced by hooks in `.claude/settings.json`. **Read this in full before
starting any autonomous session.**

---

## R1 — No exits
You may NOT end a turn while the marker file is active and the deadline
has not passed. Hook `stop-guard.ps1` blocks Stop attempts with exit 2.
Forbidden: "I'll stop here", "morning summary", "completed for tonight",
"let me know when you wake up".

## R2 — Outer loop
After ANY task, immediately:
1. Update `NIGHT_RUN_PLAN.md ## In progress` → `## Done log`
2. Pull next item from `## Backlog`
3. Start executing it
No reflection break, no "next steps" wishlist. The cycle is mechanical.

## R3 — Parallel waits
Any wait that exceeds 30 seconds (compile, video playback, agent reports,
network calls) MUST run via `run_in_background` or `Monitor`, and you
MUST start the next task while waiting. Idle = violation.

## R4 — Computer-use marathons
A "UI walkthrough" task is not done until you've systematically clicked
EVERY interactive element (every button, every input, every dropdown,
every checkbox) AT LEAST once and tested 5+ edge cases (empty input,
overflow, rapid double-click, paste of large content, special chars).
Minimum wall time per walkthrough: 45 minutes. Document every UI bug
as a separate `## Findings` entry.

## R5 — Live tests run for real
Tasks containing "live test" or "video test" mean: open Brave, navigate
to YouTube, find a relevant long video, start playback, let it run for
the specified duration MINIMUM. Do not punt with "needs setup" — the
setup is the task.

## R6 — Decisions on the spot (NEVER call AskUserQuestion)
When choosing between options ("which feature to implement", "which fix
to do first", "which agent to spawn"), PICK ONE and execute. Do not
enumerate options for the user to choose. The user explicitly delegated
this — punting choices upward violates the delegation.

**Enforced by `block-asks.ps1` PreToolUse hook on AskUserQuestion** —
the call returns exit 2 with a violation banner while the marker is
active. If you find yourself reaching for AskUserQuestion, that's the
signal you're about to violate R6.

### Concrete examples of violations (caught live, 2026-05-25)

- ❌ "Включать post-meeting debrief для теста?" → User had to answer.
  ✅ Correct: I just shipped it as opt-in; for a TEST of that feature,
  the obvious answer is "enable it". Patch the config file, restart,
  proceed. Document the choice in `## Decisions` log.

- ❌ "Сначала исправить Error chip или сразу запускать видео?" →
  Another pointless ask.
  ✅ Correct: misleading UI hurts the test signal. Fix Error first,
  then video. Decide based on what gives the cleanest test data.

- ❌ "Should I use Sonnet or Haiku for the debrief?" → Don't ask.
  ✅ Correct: Sonnet for quality (one-shot per session, cost is bounded).
  Note the choice; if the user disagrees they'll say so reactively.

- ❌ "Should I run cargo update or skip it?" → Don't ask.
  ✅ Correct: dry-run first, look at the changes, decide based on whether
  it's patch/minor/major.

### When you MAY ask (narrow exceptions)

Only when the action is **catastrophic and irrevocable**:
- About to delete data the user might want
- About to spend significant money ($1+ per attempt, e.g. retrying with a
  large model on a long input)
- About to push to remote / publish content
- About to make a security-sensitive change with no rollback path

For everything else: decide, do, log. The user reads the log later and
can course-correct. That's the design.

## R7 — Incremental plan commits
Every ~30 minutes OR after any major file change, update
`NIGHT_RUN_PLAN.md` with:
- What just finished (added to `## Done log` with timestamp)
- What's starting next (set as `## In progress`)
The PostToolUse hook counts file ops; at 30 ops without a plan update,
it forces a reminder.

## R8 — Heartbeat during long waits
If a background task or video runs >10 minutes, do NOT just wait. Either:
- Start a parallel task and check back via notification
- Use `Monitor` tool with grep filter for the events you care about
- Do code review of unrelated files while waiting
Silent idle = violation.

## R9 — Mandatory re-review
After any block of changes spanning ≥5 modified files OR ≥3 hours of
work, spawn a 6-agent re-audit (same structure as the initial mega
review). Don't claim "done" without verification pass.

## R10 — "Is there still work?"
Before considering anything complete, ask:
- What did I NOT do that was in the original mandate?
- What did I DEFER mentally even if not in writing?
- What edge case did I skip because it was annoying?
Then do those things.

---

## Lifecycle

**Activate:** `/auto 6h` (or other duration). Creates
`.claude/autonomous_active` with deadline = now+6h.

**Run:** Execute rules R1-R10. The Stop hook prevents exit. Banned-
phrases hook prevents Write/Edit punting. Progress counter forces plan
commits.

**Deactivate:** Either the deadline passes (auto), or `/auto-stop`
(emergency). The marker file is removed; hooks become inert.

**Backup:** A scheduled task fires every 2h with a status-check prompt.
This runs OUTSIDE the hook system and cannot be bypassed from inside
Claude Code.

## Failure modes the hooks cannot catch

- **Imitation work** — busywork that looks productive but isn't. Mitigated
  by external check-in (scheduled task) requiring concrete deliverable
  list per 2h window.
- **Self-disarming** — Claude removing the marker file. Mitigated by
  `banned-phrases.ps1` blocking `Remove-Item *autonomous_active*` in
  Write/Edit targets, and by the scheduled task running independently.
- **Quality of decisions** — picking trivial tasks to fill time. Mitigated
  by `## Backlog` being explicitly priority-ordered by user before run.
- **Skipping the spirit of a rule** — e.g. writing "this task is deferred
  due to time constraint" instead of "next session". No mitigation
  except your conscience and the periodic external review.

## What hooks add to your responsibility, NOT subtract

These rules are scaffolding. They prevent the obvious failure modes from
my last autonomous run. They don't make every autonomous run good —
that still requires judgment, care, and honesty about what's been done.
