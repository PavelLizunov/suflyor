# Upgrading

This is a personal-use Windows app — no config migration tool, but
your `%APPDATA%\overlay-mvp\config.json` is **forward compatible** via
serde defaults. Old configs gain new fields at their default values
automatically on next launch.

In-app updates: **Settings → 🆙 Обновления → 🔍 Проверить обновления**
shows a release-notes preview + opens the GitHub release page in browser
for download. No auto-install (no code signing — by design).

## Per-version migration notes

### → v0.0.25 (2026-05-26)

Three more UX bugs from live session (continuation of v0.0.24 sweep):

- **Tile double-click no longer maximizes.** Was Tauri 2's default
  behaviour on `data-tauri-drag-region` — double-click toggled
  maximize. User reported double-click «выделяет окно и блокирует
  остальные» — because maximize covered everything else AND grabbed
  focus. Now: `onDoubleClick={e.preventDefault + e.stopPropagation}`
  on both tile-bar AND overlay-bar.
- **Overlay always-on-top is re-asserted every 3s.** Was set at
  creation only; some windows (Zoom call, screen-share toolbars)
  push us behind. Periodic `setAlwaysOnTop(true)` keeps us TOPMOST.
- **Overlay bar auto-resizes width based on content.** Was fixed 520px;
  with «🟢 Listening 🟡 ⏱ rate-limited 💰 over budget» chips + HUD +
  buttons, the ⚙ gear at the right edge got clipped. ResizeObserver
  on bar → calls `setSize` to fit content (cap 1200px to avoid runaway
  growth). Skipped when Settings is open (Settings has its own size).

### → v0.0.24 (2026-05-26)

Bug-report sweep — user reported 6 UX issues during live session.
Addressed 4 directly, 2 deferred pending more info:

- **Bigger close/pin buttons.** 18×18 → 24×24 with visible background.
  Close button gets red-tint on hover (clear destructive cue), pin
  button gets yellow tint when active. Был жалоба «крестика не видно».
- **Bigger tile default size.** 380×280 → 460×360 initial, max 510px
  tall (vs 400). +21% width / +29% height for content. Чтобы не
  растягивать каждое окно руками. Grid math validated for 1920×1080
  (fits 2 column-pairs × 2 rows = 8 slots ≥ MAX_TILES=6).
- **Less transparent tile background.** Glass bg → opaque rgba(20,22,30,0.92).
  Two birds: (a) close × clearly visible against content; (b) edge clicks
  no longer pass through to underlying windows on certain Windows DWM
  modes (could be the "перестают быть кликабельными" report root cause).
- **Ctrl+Alt+W: close all tiles.** New global hotkey + tray menu item
  «Close all tiles». Respects pinned tiles. Helps recover from
  aggressive-mode flood without quitting the session.

Deferred pending repro:
- «Окна слишком далеко». Tiles spawn on secondary monitor by design;
  if user's secondary is physically inconvenient, will add a config
  `tile_spawn_target: primary|secondary|overlay-monitor` in v0.0.25.
- «При выборе окна блокируются все остальные». If the transparency
  fix above doesn't resolve it, need a screencast to understand the
  exact mechanism.

### → v0.0.23 (2026-05-26)

- **🚀 One-click update.** Settings → 🆙 Обновления → новая кнопка
  «🚀 Скачать и установить (one-click)» рядом со старой
  «⬇ Открыть в браузере». Качает NSIS-installer с GitHub Releases в
  `%TEMP%`, спавнит его, программа закрывается через 2 секунды,
  установщик заменяет файлы (с UAC prompt) и поднимает новую версию.
  Без хождения в браузер.
- Старая кнопка «Открыть в браузере» оставлена как fallback на случай
  если one-click не сработает (network issue, GitHub Releases отдал
  пустой asset, и т.п.).
- Защита: проверка размера скачанного файла — если меньше 100 KB
  (значит redirect HTML / corrupted asset / GitHub mid-publish),
  отказывается запускать чтобы пользователь не получил мутную ошибку
  от Windows.
- Backend cmd: `download_and_install_update`. Не использует
  `tauri-plugin-updater` потому что у нас нет signed артефактов;
  свой минимальный flow проще и не требует генерации key pair.

### → v0.0.22 (2026-05-26)

- **REAL F8 crash fix.** v0.0.21 added a JS-side re-entry guard which
  prevented one class of race, but the actual panic was in Rust — the
  v0.0.21 runtime-panics.log finally surfaced it:
  ```
  src/runtime.rs:1437 — "there is no reactor running, must be called
                         from the context of a Tokio 1.x runtime"
  ```
  Root cause: `stop_session` is a **sync** Tauri command. Tauri 2 runs
  sync commands on a thread that has NO tokio reactor in TLS. Inside
  `stop_session`, the post-meeting debrief was fired via raw
  `tokio::spawn(...)` which panics in that thread. Same root cause as
  task #93 in 2026-05-25 (also fixed by switching to
  `tauri::async_runtime::spawn`).
  
  Fixed both sites:
  - `runtime.rs:1437` (debrief fire-and-forget)
  - `tile.rs:365` (TTL task — also called from sync kb_spawn /
    expand_snippet commands)

  `tauri::async_runtime::spawn` is a drop-in for `tokio::spawn` but uses
  Tauri's own tokio runtime which is always available.

### → v0.0.21 (2026-05-26)

- **F8 crash fix.** Rapid F8 double-press during an active session
  could panic on WASAPI device race (second start_session firing while
  first stop_session was still awaiting). Now serialised via a
  `pauseInFlightRef` — subsequent F8 presses ignored until the previous
  pause/resume cycle completes.
- **Visible hotkey legend.** Hotkey strip in overlay (`F3·F4·F6·...·ℹ`)
  is now clickable. Opens a popover with full descriptions: F3 reask,
  F4 KB palette, F6 manual tile, F8 pause/resume, F9 ask AI, F10 screenshot,
  F11 PANIC HIDE. Click anywhere to close.
- **Runtime panic log.** New `%APPDATA%\overlay-mvp\runtime-panics.log`
  captures worker-thread panics (separate from startup crash-report.txt).
  Auto-included in diagnostic dump (tail 100 lines, sanitized for
  secret patterns). Каждый panic = timestamp + location + payload,
  append-only, rotates after 1MB.

### → v0.0.20 (2026-05-26)

- **No config schema change.**
- **Keyword highlighting** в тайлах: ключевые слова из `trigger_keywords`
  config'а подсвечиваются жёлтым в question + answer body.
  Сервер передаёт через `?hl=k1,k2,...` (cap 8 keywords / 150 chars URL).
- **Question max-height 78px** (~4 строки) + scroll. Долгий вопрос больше
  не давит ответ — hover на вопрос разворачивает до 200px. Ответу всегда
  гарантировано большую часть высоты тайла.
- **Bottom-scroll fix**: tile-body bumped padding-bottom + added
  `overscroll-behavior: contain` так что wheel-events не уезжают в host
  window. Раньше последние строки длинного ответа не доскролливались.
- Backend: new helper `spawn_tile_with_highlight(...)` параллельно с
  existing `spawn_tile_with_stealth(...)` — старые call sites не меняются.

### → v0.0.19 (2026-05-26)

- **No config schema change.**
- Каждый тайл теперь показывает в заголовке `#N` — sequence number в
  пределах сессии. Без этого с aggressive mode (v0.0.18+) при 30-60
  тайлов в минуту невозможно понять какой новее — слоты переиспользуются
  при evict и новый тайл может оказаться не в правом нижнем углу.
- Backend: новая static `TILE_SEQ_COUNTER: AtomicU64` в `tile.rs`,
  fetch_add при каждом спавне, передаётся через URL param `?seq=N`.
  `start_session` ресетит счётчик в 0.
- Old MSI без seq param → бейдж не рендерится (graceful).

### → v0.0.18 (2026-05-26)

- **New config field** (auto-defaulted via serde):
  - `auto_tile_every_line: bool` = `false`
- **New: AGGRESSIVE MODE.** Settings → 🪟 Auto-tiles →
  **🔥 «спавнить тайл на каждую строку транскрипта»** checkbox. When ON:
  - `maybe_spawn_tile` skips `detect_trigger` entirely. Every transcript
    line (≥5 chars) → tile, regardless of whether it «sounds like a
    question».
  - Internal `MAX_TILES_PER_MIN` bumps from 15 → 60 so the rate-limiter
    doesn't strangle aggressive throughput.
  - Use case: video / interview where Whisper isn't producing `?` and
    the candidate's own monologue is what you want suggestions on. Or
    just to confirm the AI pipeline is healthy without waiting for a
    «question» to surface.
  - Cost: ~30-50 tiles/min of continuous speech, each one Haiku call.
    Soft cost-cap chip still fires but does not block. Plan accordingly.
- Default OFF — existing users see no behaviour change unless they
  explicitly opt in.

### → v0.0.17 (2026-05-26)

- **No config schema change.**
- **Bug fix:** import config flow no longer asks you to type the full
  path manually. Settings → 🔽 Import → native Windows Explorer file
  picker. Also accepts **drag-and-drop** — drop a `.json` file onto the
  Settings window and it imports.
- **Bug fix:** path-allowlist that refused any path not under Desktop
  or Documents removed. Was breaking imports from OneDrive (Russian
  Windows uses localised "Рабочий стол" folder name), Downloads, network
  shares, anywhere else. The `assert_overlay` guard already prevents
  poisoned tile windows from reaching `import_config`, so the allowlist
  was paranoid layering with no unique defense — at the cost of breaking
  real flows.
- New dep: `tauri-plugin-dialog` (Rust + JS) for the native file picker.

### → v0.0.16 (2026-05-26)

- **No config schema change.**
- **Security:** diagnostic dump (v0.0.15 feature) now runs the journal
  tail + crash report through `sanitize_diagnostic_text`, which redacts
  `gsk_*`, `Bearer *`, `sk-*` token patterns. Belt-and-suspenders even
  though the sanitized config can't leak these — covers the edge case
  where a future panic message captures an HTTP error with the bearer
  in its Debug repr. +5 unit tests (244 total).
- Dump output now also flags that `ai_request` journal events embed the
  user's `meeting_context` in their prompts — user reviews before sharing.
- docs/architecture.md: assert_overlay count updated 25 → 31 (39 total
  Tauri commands; 8 deliberately unprotected per the doc).

### → v0.0.15 (2026-05-26)

- **No config schema change.**
- **New: Settings → Обновления → 📊 Диагностический дамп.** One click
  writes a sanitized markdown report to Desktop (config without secrets,
  last 50 journal events, crash report if present). Attach to a bug
  report instead of fishing through AppData manually.
- HTTP plaintext warning in Settings now suppressed for loopback URLs
  (127.0.0.1 / localhost / [::1]) — the warning was firing on perfectly
  safe local-host bridge setups.
- CLAUDE.md test invocation corrected (was `cargo test --lib --bin
  overlay-mvp` which runs 0 tests because the binary has none — should
  be `cargo test --lib`).

### → v0.0.14 (2026-05-26)

- **No config schema change.**
- Fix: closing Settings now restores the overlay to its pre-Settings
  position (was snapping back to the default 200,40). If you dragged
  overlay to second monitor and opened Settings, closing Settings
  used to throw the overlay back to primary monitor.
- A11y sweep: tile windows, Replay viewer, and KB palette got proper
  ARIA roles + aria-label/aria-pressed/aria-selected. Replay filter
  chips are now color-coded by event kind (matches timeline borders).
- 2 new edge-case tests for is_strictly_newer (semver compare): test
  count 237 → 239.

### → v0.0.13 (2026-05-26)

- **No config schema change.**
- Three follow-ups from post-v0.0.12 review:
  1. `start_session` now emits `cost:update {session_usd: 0}` so a stale
     "💰 over budget" chip from a prior session clears immediately on
     restart (previously had to wait for its 60s timer).
  2. Over-budget timer is now tracked via `overBudgetTimerRef` and routed
     through the existing `flashFlag` helper — a fresh cap-hit properly
     re-extends the 60s window instead of an earlier timer clearing the
     chip mid-burst.
  3. Collapsed the two `cost:update` listeners into one (smaller cleanup
     surface).

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
  restart. Now AI keeps working. v0.0.5-v0.0.11 reused the yellow
  "⏱ rate-limited" chip to signal the overage (one chip, two meanings);
  v0.0.12 split it into a dedicated "💰 over budget" chip. Rationale for
  the pivot: blocking AI in the middle of an interview was bad UX.
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
