# Upgrading

This is a personal-use Windows app — no config migration tool, but
your `%APPDATA%\overlay-mvp\config.json` is **forward compatible** via
serde defaults. Old configs gain new fields at their default values
automatically on next launch.

In-app updates: **Settings → 🆙 Обновления → 🔍 Проверить обновления**
shows a release-notes preview + opens the GitHub release page in browser
for download. No auto-install (no code signing — by design).

## Per-version migration notes

### → v0.0.33 (2026-05-26) 🚨 P0 hang fix

Four live-feedback fixes — most critical first.

- **(P0) F4 KB palette no longer hangs the app.** User: «f4 палитра
  ломает приложение зависает». Root cause: ResizeObserver + setSize
  race. When the palette opens or closes, both the palette's own
  setSize useEffect AND the bar's auto-resize ResizeObserver could
  fire on the same DOM-mutation, racing to call `setSize` on the
  Tauri window. The previous guard (`paletteOpenRef.current` set in
  a separate useEffect) was updated AFTER React commit, leaving a
  race window where RO saw palette content with the guard still
  stale → competing setSize calls → potential infinite loop / hang on
  rapid F4 + typing.

  Fix: moved the guard from a ref into the `useEffect` deps array.
  ResizeObserver is now literally not attached while palette is open
  (`if (paletteOpen) return;` at the top of the effect, plus
  `[paletteOpen]` deps so it re-attaches on close). Zero race possible.
- **(UX) Indicator legend.** User: «нужна расшифровка индикаторов».
  The ℹ-popover (click `F3·F4·…·ℹ` strip in the bar) now has a second
  table «Indicators — что значат точки и чипы» listing the 3 HUD dots
  (audio · stt · ai), voice-coach pill (🎙 wpm), screenshot-ready (📸),
  aggressive mode (🔥), rate-limited (⏱), over-budget (💰), session-cost
  ($X.XXX). The Hotkeys table also gained the Ctrl+Alt+W close-all row.
- **(UX) Settings footer no longer wraps Save to a new row.** Was 7
  buttons (Replay · Logs · Export full · Export share · Import · Back ·
  Save) overflowing the 750-px default Settings width. Moved the 5
  «сессии / экспорт» buttons into the Advanced panel (where Обновления
  + Диагностический дамп already live — conceptually they're all about
  session diagnostics & config migration). Footer is now minimal:
  just **← Back to overlay** + **Save**. Fits any window width.
- **(UX) Overlay bar padding +30 → +50.** User: «минимальный размер
  должен быть таким чтоб все индикаторы помещались + запас 50 пикселей».
  The ResizeObserver-derived desired width adds buffer past the
  measured content. Was +30, now +50. Abs floor (520), abs ceiling
  (1200), and 50 %-of-screen cap are unchanged.

### → v0.0.31 (2026-05-26)

Three follow-ups from v0.0.30 live screenshot review:

- **Confirm-modal button label is now contextual.** User reported the
  «Выйти из приложения?» modal had a red «Удалить» button — confusing,
  since the action is «Выйти» (exit), not delete. Root cause: the confirm
  modal hardcoded the OK label + danger class for the original delete-
  snippet use case, and the new exit-app call reused it unchanged.
  Fix: `showConfirm(title, { confirmLabel?, danger? })` — default label
  is «Подтвердить», default style is neutral. Quit-app passes
  `{ confirmLabel: "Выйти", danger: true }`. Profile/snippet delete
  pass `{ confirmLabel: "Удалить", danger: true }`. Future callers get
  a safe default if they forget.
- **Sidebar pins «Приложение» group (Интерфейс/Скрытность/Хоткеи/Обновления)
  to the bottom.** v0.0.30 had all 4 nav groups stacked from the top with
  empty space below — system-level panels read better at the bottom
  (Slack/Discord/Linear pattern). CSS-only fix:
  `.settings-nav .nav-group:nth-last-of-type(1) { margin-top: auto; }`.
  In a flex column, `margin-top: auto` pushes the targeted element + its
  following siblings to the end. Added a soft top border + extra padding
  so it reads as a separator, not a glitch.
- **Overlay bar max width = 50 % of screen** (with abs floor 520, abs
  ceiling 1200). v0.0.30 had a hardcoded 1200-px ceiling that on a
  1920+ monitor let the bar grow past half the screen — too dominant
  for a peripheral HUD. Now:
  - 1280×720  → max 640 px (50 % of screen)
  - 1920×1080 → max 960 px
  - 2560×1440 → max 1200 px (hits absolute ceiling)
  Implementation: `Math.min(Math.floor(window.screen.availWidth * 0.5),
  1200)` computed inside the ResizeObserver callback.

No config schema change. CSS-only + 1 JS line — no rebuild needed for
existing users beyond the standard one-click update.

### → v0.0.30 (2026-05-26) ✨ Settings sidebar redesign

**Settings UI reorganized from one long scroll into a sidebar + content
pane** per Claude Design handoff (`api.anthropic.com/v1/design/h/...`).

User asked: «можем как-то организовать [Settings]» — the original was
13 stacked `<h3>` sections with ~2000 px total height. Now: 200-px
sidebar nav on the left with 4 groups + 11 sections, content pane on
the right showing only the active section.

- **Sidebar groups + sections** (4 / 11):
  - **Сессия**: 👤 Профиль и контекст · 🎚 Аудио и STT
  - **AI**: 🛰 AI мост · модели · бюджет (⚠ HTTP badge when bridge is
    plain http to non-localhost)
  - **Логика**: 🪟 Авто-тайлы и сниппеты (badge: snippet count) ·
    📚 База знаний (badge: KB entry count, e.g. `1.6k`) · 🎓 Коучинг
  - **Приложение**: 🎨 Интерфейс · 🫥 Скрытность · ⌨ Хоткеи ·
    🔧 Обновления · диагностика
- **Search filter** in sidebar (`фильтр…`) — narrows the nav list
  client-side by label substring.
- **No content moved** — each existing settings-section was wrapped in
  a `{activeSection === "X" && (<div...>...</div>)}` conditional, so
  every field binding, save handler, modal trigger, and event listener
  keeps working unchanged.
- **All design CSS appended to `src/styles.css`** — new selectors:
  `.settings-shell`, `.settings-nav`, `.settings-pane`, `.card`,
  `.card-title`, `.card-row`, `.row-label`, `.row-hint`, `.row-control`,
  `.switch`, `.switch-row`, `.switch-meta`, `.switch-title`,
  `.switch-desc`, `.banner.warn|info`, `.chip-cloud`, `.chip`,
  `.hotkey-row`, `.hk-name`, `.hk-keys`, `.nav-search`, `.nav-group`,
  `.nav-item.active|has-warn`, `.nav-icon`, `.nav-badge`. Tokens
  (`--c-*`, `--fs-*`, `--s-*`, `--r-*`) already existed from prior
  design round — re-used as-is.
- **Audio panel** is the only one that shows two existing sections
  (Audio devices at top + STT below) since both belong logically
  together — both render when `activeSection === "audio"`.
- **Profile panel** similarly combines Профили + Meeting context.
- **Tiles panel** combines Auto-tiles + Snippets.
- **AI panel** is a single large card (the existing AI proxy block
  includes bridge URL, bearer, models, language, cost cap, bridge
  check). Future versions may split into 3 separate panels per the
  original design (bridge / models / budget).

No config schema change. JSX class names preserved — `.settings-root`
still wraps everything; `.settings-section`, `.field`, `.btn`,
`.btn.secondary`, `.btn-row` are still used inside the conditionally-
rendered sections.

255/255 lib tests still pass · vite build clean · tsc clean.

### → v0.0.29 (2026-05-26)

**Tile size is now percentage of monitor with absolute floor.** User
said v0.0.24's fixed `460×360` (with auto-grow cap `510`) was «слишком
большое» on his real display — wants it to scale.

- New constants in `src-tauri/src/tile.rs`:
  - `TILE_W_PERCENT = 0.20` — 20% of picked-monitor width
  - `TILE_H_PERCENT = 0.26` — 26% of picked-monitor height (initial)
  - `TILE_H_MAX_PERCENT = 0.36` — auto-grow cap after markdown
  - `TILE_W_MIN = 340.0` — absolute floor (keeps markdown legible)
  - `TILE_H_MIN = 240.0` · `TILE_H_MAX_MIN = 320.0`
- Computed per-spawn via `tile_dims_for(monitor)` and passed to:
  - `grid_position(monitor, dims, index)` — was using globals before
  - `WebviewWindowBuilder::inner_size(dims.w, dims.h)`
  - URL params `&mh=N&mw=N` so `TileWindow.tsx` ResizeObserver caps
    growth to the right per-monitor value
- Sample sizes:
  - 1280× 720 → 340×240 (both clamped to mins)
  - 1920×1080 → 384×281 (h_max 389)
  - 2560×1440 → 512×374 (h_max 518)
  - 3840×2160 → 768×561 (h_max 778)
- New unit test `tile_dims_scale_with_monitor_and_respect_floors` locks
  in the math at 1920/1280/3840 widths.
- 5 existing grid tests refactored to call `tile_dims_for` then pass
  `dims` to `grid_position`. Test fixture for the «short monitor»
  regression bumped 1100 → 1080 since dims now scale down (h_max=388
  on 1080p fits 2 rows easily).

No config field for the percentages yet — defaults are baked. Easy to
add later if you want per-monitor tuning. Old `TILE_W`/`TILE_H`/
`TILE_H_MAX` consts removed entirely.

### → v0.0.28 (2026-05-26) ⚠️ default change

**Cost-cap default flipped 1.00 → 0 (chip OFF) per user request.**

User has unlimited AI budget («по костам не важно, безлимитные деньги»),
so the 💰 «over budget» chip + scary copy in Settings has been replaced
with neutral status indicators. AI behavior unchanged — was always
SOFT-warning since v0.0.5, never blocked.

- **(Default change)** `max_session_cost_usd` default 1.00 → **0** (chip
  disabled). Old installs keep their existing config value (per-field
  serde default applies only when the key is missing). To re-enable: set
  any positive value in Settings → AI proxy section.
- **(UI)** Settings copy for max_session_cost_usd reworded — no more
  «$1.00 ≈ 200 Haiku тайлов» guilt; just a factual «0 = выкл (default)».
- **(UI)** 🔥 aggressive chip tooltip no longer mentions «~$4-5/час».
  Chip stays as state indicator only.
- **(UI)** Settings copy for aggressive mode no longer says
  «<strong>Стоит ≈$5/час непрерывной речи</strong>». Removed.

**4 review-agent findings from v0.0.20→v0.0.27 wider-scope pass:**

- **(P1) `close_all_tiles` Tauri command now `assert_overlay`-guarded.**
  The Ctrl+Alt+W hotkey and tray menu path call the underlying
  `tile::close_all_unpinned` directly, but the registered Tauri command
  itself was unguarded — a compromised tile-* window or DevTools could
  invoke it to nuke pinned tiles. Added `assert_overlay(&window)?` +
  changed return type to `Result<usize, String>`. No JS callers existed,
  so no frontend changes.
- **(P1) Pin button no longer shares destructive-red hover with close.**
  Both `📌` and `×` buttons used `className="tile-close"` → hovering
  the pin button gave the red destructive cue. New `.tile-pin` class
  with neutral-yellow hover; close keeps the red. New v0.0.28 CSS rule
  also updates the `data-pinned` glow selector to the new class.
- **(P1) Grid layout no longer renders tiles off-screen on small
  monitors.** On 1280×720 (and below), the math for `pair >= 2` could
  return `start_x ≈ −1564 px` → tiles 4-5 fully invisible. Added
  `max_pairs` clamp + final `start_x.max(monitor.x + PAD)` safety. +2
  regression tests (1280×720 single-monitor + secondary monitor at
  non-zero x origin).
- **(P2) `runtime-panics.log` falls back to `%TEMP%` if `config_dir()`
  returns None.** Previously dropped silently — now lands at
  `%TEMP%\overlay-mvp-panic-fallback\runtime-panics.log`.
- **(P2) `clear_update_in_flight` Tauri command unstucks the backend
  lock if BOTH `quit_app` AND `window.close()` fail.** v0.0.27's
  `mem::forget` design leaks the lock by design (expecting the process
  to die seconds later); if both shutdown paths fail, the toast-fallback
  path now also calls this command to clear the lock so a retry isn't
  rejected with «Update already in progress».

253 lib tests pass (251 baseline + 2 new grid tests). Clippy clean.

### → v0.0.27 (2026-05-26)

3 review-agent findings from the v0.0.25→v0.0.26 diff pass:

- **(P1) `runtime-panics.log` rotation now UTF-8 safe.** v0.0.26's
  keep-last-500KB rotation byte-sliced a `String` at offset 500_000
  without checking for a char boundary. This app's own panic messages
  are routinely Cyrillic (Russian comments + user-content embedded in
  anyhow! macros = 2 bytes per char), so the slice had ~50% odds of
  landing mid-char and panicking inside the panic handler. The double
  panic would have aborted startup the next time the log was rotated.
  Now: walk forward from `start` to the next valid `char_boundary`
  before slicing, then snap to the entry separator.
- **(P2) `download_and_install_update` guard uses `std::mem::forget`.**
  v0.0.26 used a `guard.reset = false` flag-mutation trick to skip the
  lock-release Drop on the success path. Functionally correct but the
  intent was fragile — a future edit slipping any fallible call between
  `spawn()` and the flag flip would silently leak the lock. Now: the
  guard is a unit struct whose Drop unconditionally clears the flag,
  and the success path explicitly `std::mem::forget`s it. Reads as
  "deliberately do NOT run the destructor" instead of mutating state.
- **(Polish) Aggressive-chip focus-listener comment clarified.** The
  v0.0.26 commit message implied the chip syncs on Settings→overlay
  return via `focus`, but Settings is inline (same window under
  `?settings=1`) so the overlay actually unmounts/remounts and the
  mount-time effect handles that path. The focus listener is a safety
  net for the alt-tab-away-and-back case (e.g. user hand-edited
  config.json in Notepad). Comment now states the real mechanism.

### → v0.0.26 (2026-05-26)

5 fixes from a code-review agent pass on v0.0.20-v0.0.25 diff:

- **(P1) Overlay auto-resize no longer clips transcript-tail / answer-bubble.**
  v0.0.25 hard-coded `setSize(width, 96)` whenever the bar's width
  changed → killed the user's manual vertical drag AND clipped the
  growing children below the bar. Now ResizeObserver watches the whole
  `.overlay-root` (not just `.overlay-bar`) and sets both width AND
  measured height.
- **(P1) runtime-panics.log keep-last-500KB instead of full delete.**
  v0.0.21's rotation removed the file at 1 MB — wiped history right
  when the user might need it most. Now seeks to a clean entry
  boundary and rewrites the latter half.
- **(P1) `download_and_install_update` backend re-entry guard.**
  Static `AtomicBool` flips on entry; second concurrent call (e.g. from
  devtools) returns «Update already in progress» instead of racing for
  the same `%TEMP%/suflyor-update-<ver>.exe` and hitting a Windows
  sharing-violation. Lock stays set on successful spawn (intentional
  — the installer has the file mmap'd until app quits).
- **(P1) `oneClickBusy` Settings button no longer stuck on quit_app
  double-failure.** Edge case: both `quit_app` AND `window.close()` fail
  → flag was never reset → button stuck at «⏳ Скачиваю…» forever.
  Now resets + shows a toast pointing to %TEMP%.
- **(New) 🔥 aggressive chip in overlay-bar** when `auto_tile_every_line`
  is on. User easily forgets between sessions; without it cost can
  unexpectedly creep to ~$5/hour. Reads config on mount and on
  window-focus (so toggling in Settings updates it on return).
- (Polish) Settings copy for aggressive mode now states the concrete
  «≈$5/час» estimate instead of vague «Расход AI взлетит».

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
