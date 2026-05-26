# Upgrading

This is a personal-use Windows app — no config migration tool, but
your `%APPDATA%\overlay-mvp\config.json` is **forward compatible** via
serde defaults. Old configs gain new fields at their default values
automatically on next launch.

In-app updates: **Settings → 🆙 Обновления → 🔍 Проверить обновления**
shows a release-notes preview + opens the GitHub release page in browser
for download. No auto-install (no code signing — by design).

## Per-version migration notes

### → v0.0.12 (2026-05-26)

- **No config schema change.**
- New "💰 over budget" chip in overlay-bar when session cost crosses
  `max_session_cost_usd`. Soft warning — AI keeps working. Previously
  conflated with "⏱ rate-limited" chip (different semantics).

### → v0.0.11

- **No config change.**
- Replay viewer has filter chips above the timeline (click to hide event
  kinds). Tile windows now close on Esc.

### → v0.0.10

- **No config change.**
- Overlay bar is now draggable (was broken since v0.0.2). Drag from any
  empty area between status badges + hold/ask buttons.
- Snippet add+edit modal in Settings → 📋 Snippets (Delete shipped in
  v0.0.9). Key format: `[a-z0-9][a-z0-9-_]*`. Key locked when editing.

### → v0.0.9

- **No config change.**
- Snippet delete button (🗑) per row in Settings → 📋 Snippets.

### → v0.0.8

- **No change.** Defensive `dotClass` refactor + README version fix.

### → v0.0.7

- **No config change.**
- Snippet filter now searches body text in addition to key + title.
- Bridge probe got 9 new unit tests for model-not-found matcher.

### → v0.0.6

- **No config change** — defaults added in v0.0.5 still apply.
- Whisper turbo toggle in Settings → 🎙 STT (`whisper-large-v3-turbo`
  option, ~3× faster, slightly lower accuracy on rare technical terms).
- Health HUD dots transition to idle gray after stop_session (were stuck
  on last green/yellow forever).
- Bridge check uses your configured ai_model first, falls back to
  universal `claude-3-5-sonnet-latest` if 400 model-not-found.
- Crash report button in Settings → 🆙 Обновления if
  `%APPDATA%\overlay-mvp\crash-report.txt` exists from prior startup
  panic. Opens in Notepad.

### → v0.0.5 ⚠️ behavior change

- **Cost cap pivoted from HARD BLOCK to SOFT WARNING.** Previously,
  crossing `max_session_cost_usd` blocked all new AI calls until session
  restart. Now AI keeps working and a yellow "💰 over budget" chip
  appears in the overlay. Rationale: blocking AI in the middle of an
  interview was bad UX.
- **Tile slot collision fix (CRITICAL).** Closing a non-last tile via ×
  could cause the next spawn to land on a still-occupied slot. Fix:
  per-tile `slot` field + first-free pick via HashSet diff. Eviction
  now reuses the slot. Unit-tested + live-verified.

### → v0.0.4

- **No config change.**
- Settings → footer split into "💾 Export (full)" and "🔐 Export (share)".
  Share-export blanks 6 sensitive fields (groq_api_key, ai_bearer,
  ai_base_url, meeting_context, context_profiles, active_profile).

### → v0.0.3

- **No change.** Bug-hunt patches: bridge probe uses cfg.ai_model,
  cost cap journals consistently, parseFloat NaN guard, GitHub
  empty-tag handling.

### → v0.0.2 ⚠️ multiple new defaults

- **New config fields (auto-defaulted via serde):**
  - `max_session_cost_usd` = 1.00 USD (HARD block in v0.0.2; SOFT
    warning since v0.0.5).
  - `detector_skip_mic` = true (auto-tile detector ignores mic source
    by default — only triggers on interviewer's voice. Fixes live
    regression where candidate's own speech triggered explanation
    tiles).
  - `post_meeting_debrief_enabled` = false (opt-in).
- **New Settings UI:** 🔌 Проверить мост button, 🆙 Обновления section,
  Max cost per session input, Детектор игнорирует mic toggle.
- **Quit / ✕ Выйти now stop_session first** so JSONL journal closes
  with SessionStop + SessionSummary (was orphaned mid-session).
- **AI calls retry on 5xx/timeout/429** (3 attempts, 1s/2s/4s backoff).
- **Crash report file** created on startup panic at
  `%APPDATA%\overlay-mvp\crash-report.txt`. v0.0.6 surfaces a button.
- **Journal size cap 500 MB** in addition to 100-file count cap.

### → v0.0.1 (initial public release)

Pet project initial drop.

## Rollback

The MSI installer replaces the previous version atomically — there's
no built-in "rollback to vX.Y.Z" button. To downgrade:

1. Download the older MSI from
   [Releases](https://github.com/PavelLizunov/suflyor/releases)
2. Uninstall current via Settings → Apps → suflyor
3. Run the older MSI

Your `config.json` stays untouched (the data dir isn't owned by the
installer). New fields added in versions newer than your downgrade
target will be ignored as unknown JSON properties — no harm done.

## Backup before upgrade

If you're nervous: Settings → 💾 Export (full) before clicking the
update. The backup file lands on Desktop with timestamp. If anything
breaks, Import it back.

## Reporting issues

If a version breaks something for you:
1. Check `%APPDATA%\overlay-mvp\crash-report.txt` (if it exists)
2. Check the latest `%APPDATA%\overlay-mvp\sessions\*.jsonl` for errors
3. Open an issue at
   https://github.com/PavelLizunov/suflyor/issues with both files
   attached (redact `groq_api_key` and `ai_bearer` if present in
   crash report).
