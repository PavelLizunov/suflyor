# Upgrading

This is a personal-use Windows app — no config migration tool, but
your `%APPDATA%\overlay-mvp\config.json` is **forward compatible** via
serde defaults. Old configs gain new fields at their default values
automatically on next launch.

In-app updates: **Settings → 🆙 Обновления → 🔍 Проверить обновления**
shows a release-notes preview + opens the GitHub release page in browser
for download. No auto-install (no code signing — by design).

## Per-version migration notes

### → v0.0.71 (2026-05-26) — QOL block 5, #19

**Tile collapse button (▾/▴) — minimize to chrome only.**

New ▾ button in tile chrome between 📌 pin and 🔄 reload. Click →
question + answer body hide via CSS display:none, tile window shrinks
to a chrome-only height (~44 px). Click ▴ → restored.

Pure frontend, no backend changes. Pin status preserved so the reaper
still respects collapsed-pinned tiles. The auto-resize useLayoutEffect
now keys on `[answer, collapsed]` so toggling re-measures + setSize's
the window.

Use case: keep 4 tiles visible during a long interview without them
eating screen real estate. Collapse the ones you've already read,
leave the active one expanded. Reload/pin/close all still work on
collapsed tiles.

### → v0.0.70 (2026-05-26) — QOL block 5, #18

**F4 palette searches snippets via `/key` prefix.**

The F4 KB palette is now also a snippet picker. Type `/elevator` to
filter user snippets by key+title; type just `/` to see the top 8.
Enter expands the selected snippet via `expand_snippet` (existing
backend command) → instant tile, $0 (no AI call).

Plain queries (no `/` prefix) still search the embedded knowledge base
exactly as before — backward-compatible. `source` field on each result
distinguishes `SNIPPET` from `GLOSSARY/COMMAND/PATTERN` so the
expandSelected handler routes to the right backend command.

Use case: in the middle of an interview you want to use a pre-written
"elevator pitch" snippet — F4 → `/elev` → Enter → tile. 4 keystrokes,
no Settings dive.

Placeholder text updated to hint at the new prefix.

### → v0.0.69 (2026-05-26) — QOL block 5, #17

**Tile age indicator (⏱) + reload generation badge (🔄×N).**

Two new tiny chips in tile chrome between source label and pin button:

- **⏱ Ns/Nm/1h+** — time since this tile was spawned. Updates every 5s.
  Dim (55% opacity) so it doesn't compete for attention.
- **🔄×N** — only visible when this tile is the result of a reload
  (gen≥1). Subtle amber background. Lets you tell at a glance "I've
  already re-asked this 3 times".

Backend changes:
- `spawn_tile_with_generation` — new entry point taking explicit gen.
  Existing `spawn_tile_with_highlight` delegates with gen=0.
- `tile_reload` now accepts `current_generation: Option<u32>` and
  bumps it via saturating_add (capped at 99 to keep badge layout sane).
- URL gets `&gen=N` only when N>0 (forward-compat: old clients/URLs
  still work, frontend defaults missing param to 0).

Use case: "did this tile come straight from the detector, or is it
already my 4th attempt?" — ⏱45s 🔄×4 tells you instantly.

### → v0.0.68 (2026-05-26) — QOL block 5, #16

**🔄 Reload button on tile chrome — re-ask the same question.**

New 🔄 button between 📌 pin and ✕ close on every tile. Click → backend
re-asks the same question via Haiku → closes the old tile → spawns a
fresh tile with the new answer.

Use case: AI's first answer was off-target or you want a second take
without typing the question again. One click = new attempt. Pin status
is intentionally NOT preserved — re-asking implies you want to consider
the new answer fresh.

Architecture:
- Tile windows can't call `tile_reload` directly (assert_overlay guard)
- Tile emits `tile:reload-request` event with `{ label, question }`
- Overlay window listens → invokes `tile_reload` → backend does the work
- On AI error: tile stays alive, overlay surfaces error toast

Cost: ~$0.001 per click on Haiku. Spinner shows ⏳ during in-flight to
prevent double-spawn from rapid clicks. New tile uses `Manual` kind
(orange chrome) since the action was explicitly user-initiated.

### → v0.0.67 (2026-05-26) — QOL block 5, #15

**STT language quick-switch chip in overlay bar (🎙 ru/en/auto).**

New `🎙 <lang>` chip in the overlay bar (between ⏱ timer and ⌥ hotkey
hint). Shows current `cfg.stt_language` setting: `ru`, `en`, or `авто`/
`auto` for None. Click cycles `ru → en → auto → ru…`. Backend `set_stt_
language` Tauri command validates the value (rejects anything except
`ru`, `en`, or empty), updates `cfg.stt_language` in-memory + persists
via `config::save`.

STT module reads `cfg.stt_language` on every transcription call, so the
switch applies on the very next audio chunk — no session restart needed.

Use case: interview mid-flow switches to English ("can you walk me
through this in English?") — instead of opening Settings → STT →
language dropdown → save (5 clicks, blocks UI), one chip click flips
the lang in <100 ms.

The chip respects the existing Settings STT panel: edits there still
work, and the chip auto-reloads `stt_language` on overlay focus event
(so changing it in Settings updates the chip without restart).

### → v0.0.66 (2026-05-26) — QOL block 5, #14

**Detector trigger tester in Settings → 🪟 Auto-tiles.**

New `🧪 Detector tester` row in the Auto-tiles card. Type sample text
+ click "Test" → backend runs real `detect_trigger` with current
`cfg.trigger_keywords` + returns:
- ✓ "Would trigger: matched as question" — for question-pattern hits
- ✓ "Would trigger: matched keyword «kubernetes»" — for keyword hits
- ✗ "Would skip: no '?', no interrogative, no keyword match" — for
  no-trigger cases

`detect_trigger` + `Trigger` enum made `pub` (was crate-private).
New `test_detector` Tauri command + `DetectorTestResult` struct.

Use case: tune trigger_keywords without spinning up a live session.
Type "What about etcd?" → see if it would fire. Add a new keyword,
re-test, iterate.

### → v0.0.65 (2026-05-26) — QOL block 5, #13

**Pre-meeting cheatsheet generator (💎 button).**

Settings → 👥 Profile → 📝 Meeting context → new "💎 Cheatsheet
(claude-sonnet-4-6)" button next to ✨ Structure. Click → backend
calls Sonnet with meeting_context + system prompt "generate 8 likely
questions and answer outlines for an upcoming interview based on this
context. Cover both behavioural + technical deep-dives." Returns full
markdown (capped at 2048 tokens, ~$0.02), saves to Desktop with
`suflyor-cheatsheet-YYYY-MM-DD.md` filename.

Use case: 15 minutes before the call, click 💎 → open file → review
likely questions + your prepared answer outlines. Walk into the
interview ready instead of cold.

Errors gracefully: if meeting_context is empty or AI bridge not
configured, returns descriptive error in the toast.

### → v0.0.64 (2026-05-26) — QOL block 5, #12

**Auto-tile dedup — skip duplicate spawns within 60 sec.**

Stops the "interviewer says 'what about kubernetes' 3 times in 30 sec
→ 3 identical tiles" spam pattern. Now: each maybe_spawn_tile
normalizes the trigger text (lowercase, whitespace-collapsed, first
60 chars), checks if same prefix has been used to spawn a tile in
last 60 sec. If yes, skips silently.

New `RuntimeState.recent_question_prefixes: Vec<(String, Instant)>`.
Cleared on session_start (per-session dedup window).

Saves: AI call cost + tile grid clutter. Doesn't reduce accuracy —
the user's first tile for that topic is already on screen, second/
third would just be the same answer.

### → v0.0.63 (2026-05-26) — QOL block 5, #11

**Bookmark last AI answer — ⭐ button + `bookmarks.md` file.**

Two new Tauri commands:
- `bookmark_last_answer` — reads `last_question` + `last_answer` from
  RuntimeState, appends to `%APPDATA%\overlay-mvp\bookmarks.md` with
  H2 question, body markdown, and `YYYY-MM-DD` timestamp footer.
  Each entry separated by `---`.
- `open_bookmarks` — opens the file in the default markdown app
  (Windows: ShellExecute via `cmd /C start`). Creates the file with a
  friendly placeholder if missing so Notepad shows something useful.

UI:
- **Overlay**: ⭐ button between PTT and 💡, click → bookmark current
  Q+A
- **Settings → 🔧 Advanced**: 📚 Bookmarks button under Diagnostic
  dump, opens the file

Use case: AI nails a great explanation of consistent hashing → click
⭐ → it's now in your personal study file. Open after the interview to
review what worked.

### → v0.0.62 (2026-05-26) — QOL block 5, #10

**Session elapsed-time chip in overlay (⏱ mm:ss).**

Visible while a session is running (between first transcript line and
explicit stop). Updates every 1s via interval. Colors:
- 0–44 min: muted text
- 45–59 min: yellow (typical interview "wrapping up" signal)
- 60+ min: red (you're now over the standard hour)

Implementation: pure frontend. `sessionStartMs` set on first
`transcript:line` event; cleared when status transitions to `stopped`
or when meeting-ending chip is clicked. No backend changes.

Use case: interviewer says "we have about an hour" → you see ⏱ 47:12
in yellow → realize you're almost out of time → wrap up gracefully.

Bilingual hover tooltip.

### → v0.0.61 (2026-05-26) — QOL block 5, #9

**AI follow-up suggestions button (💡 in overlay).**

New 💡 button in the overlay bar next to ⚙ Settings gear. Click flow:
1. Fetch `(last_question, last_answer)` snapshot from RuntimeState via
   new `get_last_qa` Tauri command (read-only, gated by assert_overlay)
2. Call new `tile_followups(question, answer)` Tauri command → Haiku
   call (capped at 256 in + 256 out tokens, <$0.001) → returns 3
   follow-up question strings
3. Format as bulleted markdown
4. `spawn_tile("💡 Follow-up questions", "- Q1\n- Q2\n- Q3")` → new
   tile appears next to existing tiles

System prompt enforces strict format: "EXACTLY 3 questions, one per
line, no numbering, no quotes, no markdown, no preamble. Each
terminated by ?". Output parser strips any rogue digits/dots/parens
just in case.

Bilingual button labels + tile title.

Use case: AI just answered "explain consistent hashing". You see the
answer, want to anticipate what the interviewer might dig into next —
click 💡, get a tile with: "How do you handle adding nodes to the
ring?" · "What's the impact of virtual nodes?" · "How does it compare
to range-based sharding?"

### → v0.0.60 (2026-05-26) — QOL block 5, #8

**Session stats dashboard panel.**

New Settings sidebar item `📊 Stats` (after `🔧 Updates · diagnostics`).
Lazy-loads on first open via new Tauri command
`read_all_session_stats` — walks every `.jsonl` in `sessions/`, reads
each line once, aggregates:

- Total sessions count (+ closed count if app crashed mid-session)
- Total runtime (hours + minutes)
- Total AI requests + tiles spawned
- Total cost in USD
- 30-day bar chart of session count per day (oldest left → newest
  right, tooltip on hover)
- Top-5 most-frequent tile question prefixes (normalized to lowercase,
  whitespace collapsed, first 60 chars)

Backend includes a public-domain civil-from-days date conversion
(`ymd_from_unix_ms`) so we don't pull in chrono just for the "YYYY-MM-
DD" buckets. Files >50 MB are skipped to keep the pass fast (typical
1h-session journal ≈ 200 KB).

Refresh button re-reads the directory. All bilingual.

Use case: "how much have I spent on Claude this month?" — one click,
exact number. Or "what question keeps coming up across interviews?" —
top-5 list shows recurring topics for prep focus.

### → v0.0.59 (2026-05-26) — QOL block 5, #7

**Meeting-ending auto-detector — 🏁 chip in overlay.**

Backend: new `meeting_ending_phrase_match(text)` in runtime.rs scans
transcript lines (system-audio source only — your own "thanks" doesn't
trigger) for ~25 case-insensitive goodbye patterns in EN + RU:
"thanks for your time" · "we'll be in touch" · "any final questions"
· "let's wrap up" · "спасибо за уделённое время" · "будем на связи" ·
"приятно было пообщаться" · etc. Pattern list intentionally requires
multi-word matches so a mid-interview "thanks" doesn't false-fire.

When matched: `RuntimeState.meeting_ending_emitted` flips to true
(once-per-session flag, reset on `start_session`) + Tauri event
`meeting:ending` fires.

Overlay: orange `🏁 ending?` button in the bar (between hotkey-
warnings and PTT buttons). Click → `stop_session` (closes journal
cleanly, fires SessionSummary). Hover tip explains in current lang.

5 new unit tests cover EN + RU patterns + mid-interview false-positive
guards + case-insensitivity + empty/short edge cases.

Use case: interview wraps, you forget to hit F8 → backend notices
"thanks for your time", chip appears → one click, session closed
cleanly + journal saved + cost final. No stale-session ghost cost.

### → v0.0.58 (2026-05-26) — QOL block 5, #6

**Markdown export of session from Replay viewer.**

Replay viewer header gets a new `📥 .md` button next to Back. Click →
backend `export_session_markdown` command reads the selected JSONL,
walks events, generates a human-readable .md and saves to Desktop with
the session's stem name + `.md` ext.

Rendered sections (chronological):
- H1 with session filename
- Header line: model / prep model / response_language + start time
- For each ai_request/ai_response pair: H2 with "#N · purpose · time",
  Prompt code-block, Answer markdown, latency + cost footer
- Summary block at the end: duration, transcript counts, AI request +
  tile counts, total cost

Intentionally skips: raw transcript lines, detector decisions, rate-
limit events. They're noise for a human reading a recap. The Replay
viewer is the right tool when you want the raw timeline.

Path validation: same as `load_session` — only reads from the sessions
dir (canonicalize + starts_with check), 10 MB max file size.

Use case: post-meeting, you want to send your interview partner the
key answers you gave. Click 📥 .md → share the file.

### → v0.0.57 (2026-05-26) — QOL block 5, #5

**Snippet usage frecency sort + count badge.**

(Versioned 0.0.57 instead of 0.0.58 — I'm shipping the frecency task
sooner than test-mic which needs backend audio work.)

Snippets list in Settings → 🪟 Auto-tiles & snippets now sorts by
usage count descending (most-used first), alphabetical for ties.
Each snippet row shows a small `N×` badge in accent color when count
> 0. The `Expand →` button increments the counter on success.

Backed by `localStorage["snippet.uses"]` = `{ [key]: count }` (per-key
cap at 999 to prevent badge overflow). State + bumper live in
Settings.tsx; the list `.sort()` runs after `.filter()` on render
(snapshot via `.slice()` so cfg.snippets stays immutable).

Use case: your most-used snippet (`/k8s-debug` say) bubbles to top
each session — no scrolling through the 57-snippet default list.

### → v0.0.56 (2026-05-26) — QOL block 5, #4

**Recent KB searches as quick-pick chips in F4 palette.**

When F4 palette opens with empty input, show last 10 KB queries as a
horizontal chip strip above the (empty) results list. Click a chip →
prefill the query → KB-search useEffect re-runs. Successful
`expandSelected` writes the current query to history.

Backed by `localStorage["kb.history"]` (JSON array, capped at 10
entries, dedupe by case-insensitive match). State is in Overlay.tsx
(`kbHistory` + `pushKbHistory`). New CSS block in styles.css for the
`.kb-palette-history` chip strip.

Use case: you searched "consistent hashing" earlier — F4 next time,
empty input, see the chip, click — instantly back to the same hits.

### → v0.0.55 (2026-05-26) — QOL block 5, #3

**Tile font size + compact overlay mode.**

Two unrelated Settings → 🎨 Interface additions:

1. **`📐 Tile font size`** — slider 11-18 px, default 12. Backed by new
   `cfg.tile_font_size: u32` (forward-compat via `#[serde(default)]`).
   `spawn_tile_with_highlight` reads value from `SharedConfig`, clamps
   to [11, 18], and bakes `&fs=14` into the tile URL. TileWindow.tsx
   parses the param + sets `--tile-font-size` CSS var on `.tile-root`.
   `.tile-body { font-size: var(--tile-font-size, var(--fs-13)) }`.
   Applies to **newly spawned tiles** — existing tiles keep their
   size until next session.

2. **`🤏 Compact overlay mode`** — toggle. Stored in
   `localStorage["overlay.compact"]`. Overlay reads on mount + on
   `storage` event (same pattern as `overlay.showCost` from v0.0.13).
   Adds `.overlay-root.compact` class. CSS hides via aria-label
   attribute selectors: 📸 ready · 🔥 aggressive · ⏱ rate-limited ·
   💰 over budget · 💰 $cost · ⚠ hotkey-warnings · 🎙 voice-coach pill.
   Keeps: status text + dot · health HUD · PTT buttons · ℹ legend · ⚙.

### → v0.0.54 (2026-05-26) — QOL block 5, #2

**Copy code block button on tile pre blocks.**

Each `<pre>` in a rendered tile body now wraps in a relative-positioned
div with an absolutely-positioned "📋 Copy" button in the top-right
corner. Button is `opacity: 0` until the wrapper is hovered. Click →
`navigator.clipboard.writeText(pre.innerText)` (works in Tauri WebView
since v0.0.27's CSP allows clipboard) + 1.2 sec "✓ Copied" feedback.
Bilingual labels via existing `lang` state.

Use case: interviewer asks for a sample command, you don't want to
retype `kubectl get pods -n my-namespace` — hover, click, paste.

### → v0.0.53 (2026-05-26) — QOL block 5

User: «продолжай оверлей и подумай что еще можно сделать, мне нужно
6 часов бесперывной работы проверок и твоих фонтазий QOL доработак».
Starting marathon block 5 — quality-of-life sprint.

**v0.0.53: syntax highlighting in tile code blocks.**

Tiles render lots of code (kubectl, sql, bash one-liners) — previously
just plain monospace. Now uses `rehype-highlight` + `highlight.js`
github-dark theme. Auto-detects language from fence info-string or
content. Composes with existing `.tile-body.markdown pre` tinting
(background override `.hljs { background: transparent !important }`
so our charcoal bg wins, syntax colors layer on top).

Added deps: `rehype-highlight ^7.0.2`, `highlight.js ^11.11.1` (+ 4
transitive). Total bundle size delta: ~95 KB gzip (full highlight.js
common-languages pack — covers bash/js/ts/sql/yaml/json/python/rust/go).

### → v0.0.52 (2026-05-26)

i18n marathon close-out — translates the previously-deferred **Snippets
CRUD modal** + per-snippet row action buttons. After this release,
literally every visible UI string in the app is RU+EN.

Coverage:
- **Per-snippet row buttons**: Expand → · ✎ Edit · 🗑 Delete with
  tooltips templated as `{key}` (the snippet's `/key` identifier),
  Expand/Edit/Delete success + fail toast templates with `{key}` +
  `{err}` + `{n}` (remaining count) interpolation
- **Delete confirmation modal**: full body text with `{key}` and
  `{title}` interp (restoration-via-Import-config hint included)
- **Edit modal title**: ✎ Edit /{key} interp
- **CRUD modal form** (shown when adding/editing a snippet):
  - Key field: label with `/{key}` preview interp, placeholder, the
    "locked when editing" explanatory hint
  - Title field: label + placeholder
  - Body field: label + multi-line placeholder
  - **5 inline validation errors**: key required, key format (latin +
    digits + `-`/`_`), title required, body required, key-duplicate
    (with `{key}` interp)
  - Save button → uses new `common.save` key

Cleanup:
- **Fixed variable shadowing of `t`** at Settings.tsx ~1929 (agent
  v0.0.51 review flagged this as a footgun). Renamed local
  `const t = snipTitle.trim()` to `const title = ...` so the imported
  `t()` translation helper remains accessible in that handler scope.

Added 18 new i18n keys. Translation map: 217 → 235 keys × 2 langs =
470 entries.

**EVERY visible UI string is now translated**, including the previously
"deferred indefinitely" snippets CRUD path. The only Russian text left
in the binary is in source-code comments + tray menu (Rust-side
rebuild only at startup — would need separate work to translate).

No backend changes.

### → v0.0.51 (2026-05-26)

Agent-review hotfix on top of v0.0.50. Independent review agent
caught 3 legitimate i18n misses OUTSIDE the deferred snippets-CRUD
scope:

1. **Snippets "Edit via JSON" footer note** (Settings.tsx ~1384) —
   inline hint with `<code>` tags split into 3 parts (before/middle/
   after) wrapping the `%APPDATA%\overlay-mvp\config.json` and
   `snippets` literals. Each part now via t().
2. **Toast close button** (Settings.tsx ~1803) — aria-label + title
   were always hardcoded "Закрыть" regardless of UI language. Now
   `t("toast.close", lang)`.
3. **Default modal confirm label** (Settings.tsx ~1868) — when a
   showConfirm caller doesn't pass `confirmLabel`, the fallback was
   hardcoded "Подтвердить" (Russian). Now `t("modal.confirm.default",
   lang)`. Caller-supplied labels (which already use `t()` at the
   call site, e.g. `t("settings.quit.confirm.label", lang)`) keep
   working as before — this fallback only fires when the caller is
   lazy.

Plus i18n entries cleaned up — added 5 new keys:
`snippets.json.hint.{before,middle,after}`, `toast.close`,
`modal.confirm.default`.

Still NOT translated (deferred indefinitely):
- showSnippetEdit CRUD modal (3-field form + validation messages +
  per-snippet expand/delete row buttons) — agent's P1 finding
  matched my v0.0.50 acknowledgement; user almost never opens this
- TileWindow sourceLabel inline ternaries (agent P2) — not via t(),
  but encoded in the same i18n.ts file so easy to extend later
- 1-frame stale "ru" flash on first overlay mount before
  `get_config` resolves (agent P2) — cosmetic, can be fixed by
  pre-fetching at index.html level if it ever becomes an issue

No backend changes.

### → v0.0.50 (2026-05-26)

Ninth and (likely) final i18n release. Mops up the remaining
user-visible strings I deferred in earlier releases.

Strings covered:
- **Settings header drag tooltip**: «Перетащи за этот заголовок чтобы
  подвинуть окно» / «Drag this header to move the window»
- **Drag-and-drop import** of config.json: bad-extension toast,
  success toast, error toast — all with {ext}/{err} interpolation
- **Meeting context "record empty" error**: shown when user clicks
  Structure without recording or typing anything first
- **Snippets section header** (the parts visible without expanding):
  section title with {n} count interpolation, expand/collapse
  toggle button + tip, + New button + tip, create-success +
  create-fail toast templates with {key}/{n}/{err} interp,
  description label, filter input placeholder with {n} interp,
  collapsed-hint footer, empty-state message

NOT translated (deferred indefinitely — extreme edge):
- showSnippetEdit modal internals (3-field form labels, validation
  messages) — only seen when adding/editing a snippet, user rarely
  does this once the defaults are configured
- Per-snippet expand/delete row buttons inside the list — these only
  appear when the user explicitly expands the snippet list, and the
  labels are mostly icons + the snippet's own user-data text

After this release, EVERY visible UI string the typical interview-day
user encounters is fully translated to RU + EN. Switching via Settings
→ 🎨 Interface → 🇷🇺/🇬🇧 takes effect immediately on the overlay
(via window-focus re-read) and on next Settings mount.

No backend changes.

### → v0.0.49 (2026-05-26)

Eighth i18n release. Translates the **Replay viewer** (Settings → 📊
Session Replay). Last remaining major UI surface.

Strings covered:
- Header: dialog aria, "📊 Session Replay" title (universal — was
  already English), session-picker dropdown aria + placeholder
  ("— pick a session —"), Back-to-overlay button + aria
- Status messages: "Loading…", "Empty session (no events).", "No
  sessions yet. Start a session from the overlay to populate this
  list."
- Filter chips: "Filter:" prefix label, show/hide tooltips with
  {kind}/{count} interpolation, "↺ reset" button + tooltip
- Footer: "{n} events · {ai} AI responses" + cost line with both
  variants (total cost / not tracked)

Row-level inline labels (model=, prep=, lines, min, ms, finish=,
in-tok, screenshot, etc.) and event-kind row labels (SESSION START,
SUMMARY, TILE, AI REQ, etc.) intentionally kept as technical English
— they're field names from the JSONL journal and changing them would
desync from the on-disk format. Indicator emoji (🎤 🗣 ✓ 📎) stay
universal.

Replay loads ui_language via get_config on mount — works because
Replay renders inside the overlay window (?replay=1), passing the
assert_overlay caller guard.

Still NOT translated:
- Snippets section in Settings (~40 strings + dynamic + CRUD modal) —
  deferred indefinitely; user rarely touches once configured

No backend changes.

### → v0.0.48 (2026-05-26)

Seventh i18n release. Translates the **Overlay bar** (the most visible
surface — user sees it every second) and **Tile chrome** (close + pin
+ source label).

Strings covered in Overlay.tsx:
- Drag tooltip
- Status text (Stopped · Paused · Listening · Asking AI… · Answering ·
  Error with {msg} interpolation)
- Health-HUD aria-label
- PTT button labels (hold/ask), tooltips for system/mic with hold-vs-
  click variants, aria-labels with recording-state variants
- Chips: 📸 ready (aria), 🔥 aggressive (aria + tip), ⏱ rate-limited,
  💰 over budget (aria + tip), 💰 $X.XXX (cost tip + aria), ⚠
  hotkey-warnings (tip + aria)
- Help popover button: aria + tip
- Settings gear: tip + aria
- Help popover content: dialog aria, Hotkeys title, all 8 hotkey
  descriptions (F3/F4/F6/F8/F9/F10/F11/Ctrl+Alt+W), Indicators title,
  all 9 indicator descriptions
- KB palette: dialog aria + search input placeholder

Strings covered in TileWindow.tsx:
- Source label (system/mic/manual/auto·detector)
- Tile #N tooltip
- Pin/Unpin tooltips + aria-labels
- Close button tooltip + aria-label

To support tile chrome i18n, `tile.rs` now embeds `&lang=ru|en` in
the tile URL when spawning. Tiles can't call `get_config` (gated by
`assert_overlay` caller guard), so the language is baked into the URL
at spawn time. Pulled from `SharedConfig.ui_language` via
`app.try_state::<SharedConfig>()`.

Still NOT translated (v0.0.49+):
- Snippets section in Settings (~40 strings + dynamic + CRUD modal)
- Replay viewer

No new config fields. The `ui_language` field from v0.0.42 powers
both the overlay AND the tiles via the new URL-param plumbing.

### → v0.0.47 (2026-05-26)

Sixth i18n release. Translates the **Advanced (Updates · Diagnostics)**
panel. ~25 strings + many tooltips + toast templates.

Strings covered:
- **🆙 Updates section**: section title, Check button + Checking…
  busy state + tip, all 4 check-result toasts (new version available
  / up-to-date / API error / network failure) with {latest} {current}
  {err} interpolation, current+latest version label
- **✨ Update available banner**: title with {latest}, Release notes
  summary, "Up to date" footer with {current}
- **🚀 Download & install button**: button + busy state + tip,
  start/ok/fail/stuck toast templates with {file}/{err} interp
- **⬇ Open in browser button**: button + tip + failure toast
- **SmartScreen + GitHub API notes**: the "Unknown publisher" warning
  + the api.github.com endpoint note
- **⚠ Crash report banner**: title, description with {path} interp
  (file path stays inside `<code>`), Open in Notepad button + tip +
  failure toast
- **📊 Diagnostic dump button**: button + tip, success+failure toasts
  with {path}/{err} interp, "Saves to Desktop. Secrets blanked" note
- **Sessions and config export section**: label, 5 buttons (📊 Replay,
  📁 Session logs, 💾 Export full, 🔐 Export share, 📥 Import) with
  their tooltips + success toasts, Import file-dialog title + filter
  names (JSON config / All files), import-success + export-fail toasts

Still NOT translated (v0.0.48+):
- Snippets section (~40 strings + dynamic + CRUD modal)
- Overlay bar tooltips + hotkey-help popover + indicator legend
- Tile chrome (close, pin, source labels)
- Replay viewer

No backend changes.

### → v0.0.46 (2026-05-26)

Fifth i18n release. Translates the **Auto-tiles** card and the
**Knowledge base** panel including its dynamic stats line and search
results.

Strings covered:
- **🪟 Auto-tiles**: card title, switch title/desc/aria for "Enable
  auto-windows on transcript questions", Monitor for tiles label/hint +
  the auto-select default option, Trigger-keywords label/hint
- **📚 Knowledge base**: title, stats line with {total} / {glossary} /
  {commands} / {patterns} interpolation, Search label + placeholder,
  searching / no-match states with {q} interpolation, Open button +
  tip + opened-tile toast / kb_spawn fail toast with {h} interpolation,
  source-aria tooltip with {s} interpolation, footer note about
  embedded KB files

Still NOT translated (v0.0.47+):
- Snippets section (~40 strings + dynamic snippet entries +
  showSnippetEdit modal labels) — deferred because of the CRUD modal
  complexity
- Advanced (Updates · Diagnostics) (~25 strings)
- Overlay bar + tile chrome + Replay viewer

No backend changes.

### → v0.0.45 (2026-05-26)

Fourth i18n release. Translates the **Profile + Meeting + Audio** panels.

Strings covered:
- **👥 Context profiles**: dropdown label, "— none —" empty option,
  Save / Delete buttons, the showPrompt "Name of new profile" + its
  placeholder, the save+delete toast messages, the delete-confirm
  body with the {name} placeholder
- **📝 Meeting context**: section title, textarea label + placeholder,
  Record voice button (with {sec} countdown), Structure button (with
  {model} from cfg.prep_model), both tooltips
- **🎤 Audio devices**: Mic + System audio labels with the
  Astro-A50-specific tip, default-option text, (loopback) suffix
- **🎙 STT (Groq Whisper)**: API key + Language + Whisper model
  labels, both model option descriptions (large vs turbo), the
  turbo-vs-large-v3 latency explanation note

First use of `{placeholder}` interpolation pattern — just `.replace()`
on the returned string, no helper. Good enough for v0.0.42's design
goal of "small typed map, no runtime dependencies."

Still NOT translated (v0.0.46+):
- Auto-tiles + Snippets (~40 strings + dynamic snippet entries)
- Knowledge base (~15 strings + search results)
- Advanced (Updates · Diagnostics) (~25 strings)
- Overlay bar + tile chrome + Replay viewer

No backend changes.

### → v0.0.44 (2026-05-26)

Third i18n release. Translates the **AI panel** — all 4 sub-cards.

Panels covered:
- **🛰 Bridge endpoint** — Base URL + Bearer secret + Health check
  labels/hints, "Check bridge" / "Checking…" button states, fail-tip
  troubleshooting message, HTTP-warn banner
- **🧠 Models + language** — Live answers + Context prep + Response
  language labels/hints, model option descriptions ("fast, default" /
  "smarter, slower" / "max quality" etc.)
- **💰 Per-session budget cap** — Cap (USD) label/hint, reference note
- **🎯 Tile-spawn trigger** — Ignore your own voice (mic) + Aggressive
  mode switch titles + descriptions + aria-labels

Still NOT translated (v0.0.45+):
- Profile + Meeting context (~25 strings)
- Audio + STT (~10 strings)
- Auto-tiles + Snippets (~40 strings + dynamic entries)
- Knowledge base (~15 strings + search results)
- Advanced (Updates · Diagnostics) (~25 strings)
- Overlay bar + tile chrome + Replay viewer

No backend changes.

### → v0.0.43 (2026-05-26)

Second i18n release. Translates the **simple panels** — those with
single boolean toggles + minimal hierarchical content — using the
infrastructure from v0.0.42.

Panels covered:
- **Stealth** (🎯 Screen-share behaviour): card title, switch
  title/desc/aria, info banner
- **Coaching** (🎓 Post-meeting debrief): card title, switch
  title/desc/aria
- **Interface** (🎨 Overlay appearance): card title for cost-indicator
  toggle + switch title/desc/aria (the language switcher card was
  already translated in v0.0.42)
- **Hotkeys** (⌨ Global hotkeys): card title, instructional hint, all
  4 row labels (Ask AI · Take screenshot · Toggle visibility · Pause
  audio) + their hints

Still NOT translated (deferred to v0.0.44 → v0.0.45):
- AI panel (~30 strings — Bridge, Models, Budget, Detector cards)
- Profile + Meeting context panel (~25 strings)
- Audio + STT panel (~10 strings)
- Auto-tiles + Snippets panel (~40 strings + dynamic snippet entries)
- Knowledge base panel (~15 strings + search results)
- Advanced (Updates · Diagnostics) panel (~25 strings)
- Overlay bar + tile chrome + Replay viewer

No backend changes — same `ui_language` field from v0.0.42.

### → v0.0.42 (2026-05-26)

User: «Также сделай полные переводы как на русский так и на английский».

First i18n release — adds the **infrastructure** + translates the
**visible-always strings** (sidebar nav + header + footer + search
placeholder + quit-confirm). Per-panel content strings (~360 total)
roll out in v0.0.43 → v0.0.45 per `docs/I18N_PLAN.md`.

- **New config field** `ui_language: "ru" | "en"` (default `"ru"`,
  forward-compat via `#[serde(default = "default_ui_language")]`).
  Stored in `%APPDATA%\overlay-mvp\config.json` like every other
  preference. Anything other than `"en"` falls back to RU at the
  `resolveLang()` boundary.
- **`src/i18n.ts`** — typed `t(key: StringKey, lang: Lang)` helper
  backed by a const object literal. TS catches missing keys at compile
  time, fallback chain is `lang → ru → key-itself`.
- **Settings UI** — header (Settings · ✕ Quit), footer (← Back to
  overlay · Save · ✓ Saved), 4 sidebar groups, 10 sidebar items,
  search placeholder + aria-labels, quit-confirm text + button label
  all use `t(...)` now.
- **Language switcher** — new card at the top of Settings → 🎨
  Интерфейс. Two pill buttons (🇷🇺 Русский / 🇬🇧 English). Click
  flips `cfg.ui_language` immediately (live re-render), Save persists.

NOT translated yet (deferred to v0.0.43+):
- Per-panel content (~300 strings across Profile, Audio, AI, Tiles,
  KB, Coaching, Stealth, Hotkeys, Advanced)
- Overlay bar tooltips + hotkey-help popover + indicator legend
- Tile chrome (close, pin, source label)
- Replay viewer event labels + filter chips
- Tray menu (Rust-side — separate concern, low priority)

Anything not yet covered renders in Russian regardless of language
setting — fallback to source language is intentional, no "[missing]"
placeholder.

### → v0.0.41 (2026-05-26)

User: «хедер где settings и Выйти и футтер где back to overlay и save
были всегда зафиксированы снизу и сверху окна». Also bundles in
v0.0.39 Hotkeys + Auto-tiles polish and v0.0.40 AI panel split (all
unreleased separately — folded into v0.0.41 since they touch the same
Settings UI).

- **Sticky header + footer** — FOUR CSS + Rust fixes (the fourth was
  the actual P0 root cause, caught by computer-use smoke test in Gate
  5 of the release verification):
  1. **(P0 — root cause)** `.settings-root` had TWO conflicting rules:
     the design-handoff rule at line 533 (`position: fixed; inset: 0;
     display: flex; flex-direction: column`) followed by a legacy
     override at line 1275 (`position: static; inset: auto; overflow-y:
     auto; padding: var(--s-5); gap: 0`). The second wins in the
     cascade, clobbering the entire flex-pin layout — the root scrolls
     as one unit, dragging both header and footer out of the viewport
     when content gets tall. Smoke test on AI panel exposed it: footer
     visible on small Профиль pane, GONE on tall AI pane. Fix: reduce
     the legacy override to just the cosmetic resets (no border, no
     border-radius) so the design's flex+fixed pin survives.
  2. `.settings-header` had a legacy `margin: -16px -16px 12px -16px;`
     from when `.settings-root` had 16px padding. The negative margin
     pushed the header outside flex flow, breaking the column layout.
     Removed margins, added explicit `flex: 0 0 auto`.
  3. `.settings-pane` was missing `min-height: 0` — required so the
     `overflow-y: auto` actually scrolls inside the grid track.
     Without it, the pane grows to its content height and pushes the
     footer off-screen.
  4. `open_settings` was hardcoded to resize the overlay window to
     760×900 px. On screens shorter than 900 (laptops with 1366×768
     or scaled 1080p), the bottom of the Settings with the footer
     was off-screen. Now: cap height to `monitor_h.clamp(480, 900)`
     (replaced `.min().max()` chained calls per clippy::manual_clamp).
- **AI panel polished** (v0.0.40 work folded in) — split the wall of
  9 `.field` blocks into 4 `.card` sub-sections:
  - 🛰 Bridge endpoint (URL + Bearer + Check button)
  - 🧠 Models (ai_model + prep_model + response_language)
  - 💰 Budget (cost cap)
  - 🎯 Detector (skip_mic + aggressive toggles)
- **Hotkeys panel polished** (v0.0.39 work folded in) — wrapped in
  `.card` with `.card-row`s
- **Auto-tiles panel polished** (v0.0.39 work folded in) — `.card`
  with `.switch-row` for the boolean + `.card-row`s for monitor
  select + trigger-keywords textarea

i18n (full RU + EN translations) is planned for v0.0.42+ per
`docs/I18N_PLAN.md`.

### → v0.0.38 (2026-05-26)

Second Settings polish micro-release. Converts **Coaching** + **Interface**
panels (both have single boolean toggles — same template as Stealth in
v0.0.37). Two panels folded together because they're trivial conversions
with identical risk profile.

- **Coaching**: `.card` with title «🎓 Post-meeting debrief», `.switch-row`
  with title + desc (session ≥30s + ≥5 mic-lines + ~$0.005 Sonnet vote),
  pill toggle on `post_meeting_debrief_enabled`.
- **Interface**: `.card` with title «🎨 Внешний вид overlay», `.switch-row`
  with title + desc (cost hide doesn't disable accounting, just hides chip),
  pill toggle on the localStorage-backed `showCost`.

Behavior unchanged on both — same state writes, same effects. Pure visual
conversion.

### → v0.0.37 (2026-05-26)

First of the planned Settings polish micro-releases (see
`docs/SETTINGS_POLISH_PLAN.md`). Converts the **Stealth** panel from
the legacy `.field + <input type="checkbox">` to the design's
`.card + .switch-row + .switch + .banner.info`.

- Same backend behavior — `cfg.stealth_enabled` toggle + `set_stealth`
  invoke unchanged
- Visual: pill-shaped toggle (yellow-on when active) replaces the
  bare checkbox; `.card-title` "🎯 Screen-share поведение"; an
  info banner with the OBS / Teams test instruction
- Click on the toggle (or the surrounding row in future panels) flips
  the value via `onClick` instead of `onChange`

Template for the rest of the panels (coaching → interface → hotkeys
→ detector → budget → audio → tiles → knowledge → ai → profile),
ordered by risk. Each future panel = one micro-release through the
full 6-gate RELEASE_CHECKLIST.md cycle.

### → v0.0.36 (2026-05-26)

Agent-review findings on the v0.0.30→v0.0.35 block. **P0 hotfix**
inside, plus 2 P1 + 1 P2.

- **(P0) ℹ hotkey-help popover was clipped.** v0.0.33 added the
  indicator legend, doubling the popover height to ~400 px. The
  popover is `position: absolute` inside `.overlay-root { overflow:
  hidden }` → doesn't contribute to contentRect → ResizeObserver
  never grew the OS window → bottom half invisibly clipped. Users
  who clicked ℹ saw only the Hotkeys table, no Indicator legend.
  Fix: explicit setSize-to-500 on toggle (mirrors palette pattern).
  RO also gated by `hotkeyHelpOpen` so it doesn't fight the manual
  resize.
- **(P1) `download_and_install_update` setTimeout cleanup.** The
  2-second `setTimeout(quit_app, 2000)` after spawn was not stored
  in a ref, so if user clicked Back to overlay during the window
  Settings unmounted but the timer still fired → app quit while
  user was back on the bar. Fix: store timer ID in
  `quitAfterDownloadTimerRef`, clear in unmount cleanup, plus
  `mountedRef.current` guard inside the callback.
- **(P1) Sidebar bottom-pin CSS selector hardened.** Was
  `.settings-nav .nav-group:nth-last-of-type(1)` — matches by tag,
  not class. Any future `<div>` added inside `.settings-nav` after
  the «Приложение» group would silently break the bottom-pinned
  layout. Now: explicit `.nav-group-pinned` class applied by JSX
  based on a `lastGroupIdx` computation. Type-safe, intent-revealing.
- **(P2) Overlay bar width cap uses CURRENT monitor.** Was
  `window.screen.availWidth - 20` (primary monitor only). User who
  drags overlay to a wider secondary monitor was stuck with the
  primary cap. Now: Tauri's `currentMonitor()` cached in
  `currentMonitorWRef`, refreshed on `onMoved` events.

255/255 lib tests still pass; tsc clean; vite build clean. **Passed
all 6 release-checklist gates** before push.

### → v0.0.35 (2026-05-26) 🚨 P0 hotfix for v0.0.34

**v0.0.34 shipped a P0 infinite-grow bug.** User reported: «в 0.34 при
запуске окно уехало в бесконечность». Caught immediately on the first
launch. Caused by:

```js
// v0.0.34 buggy logic:
const intrinsic = bar.scrollWidth;  // == offsetWidth when content fits
const needed = intrinsic + 50;
if (needed > current + 4) setSize(needed, ...);
// After grow: scrollWidth = newWidth, needed = newWidth + 50,
// still > current + 4 → setSize again → ∞
```

v0.0.35 fixes this with:

- **Real intrinsic measurement:** sum of children `offsetWidth` + gaps
  + bar's horizontal padding. With `.overlay-bar > * { flex-shrink: 0 }`,
  each child's `offsetWidth` IS its natural width regardless of the
  parent's actual size. Sum is stable across window resizes.
- **Hard screen-width safety cap:** `Math.min(needed, screen.availWidth - 20)`.
  Even if a future bug recreates an infinite-grow, the window can never
  escape the visible monitor.
- **One-shot initial fit:** the FIRST ResizeObserver fire of a session
  is allowed to SHRINK too. Subsequent fires are grow-only. This auto-
  corrects users who upgraded from v0.0.34 with a persisted oversized
  window state (no manual reset needed).

Also: established a **strict release-verification methodology** in
`RELEASE_CHECKLIST.md`. v0.0.34 passed every static check (255 tests,
clippy clean, tsc clean, build clean) — but no one actually launched
the binary. Going forward, every release must pass 6 gates including
"smoke test via computer-use screenshot, verify the window dimensions
stay stable over 5 seconds" BEFORE git push + GitHub release.

### → v0.0.34 (2026-05-26)

Three live-feedback fixes:

- **Settings footer visually pinned.** User: «футер не зафиксирован».
  The footer (Back + Save) was positionally fixed via flex-column +
  shell `flex:1 1 auto` but had NO visual differentiation — same
  background as the pane, no separator — so it read as a floating
  control row. Added a `.settings-footer` modifier class with
  `border-top: 1px solid var(--c-border)` + `background: var(--c-bg-2)`
  + tighter padding. Layout-wise unchanged; just visually fixed.
- **Overlay bar: no more 50%-screen cap.** User: «Основной экран
  должен быть не на 50%, а чтоб все индикаторы умещались +50 пикселей».
  v0.0.31 capped bar width at `min(screen × 0.5, 1200)`. With many
  active chips (🔥 + ⏱ + 💰 + cost + screenshot etc.) the bar wanted
  ≈1000 px but on 1920p screens got capped at 960, hiding the last
  chip. The cap is gone — bar grows freely to content + 50 px.
- **Overlay bar is now manually resizable.** User: «я его не растянуть
  не сузить не могу». The previous ResizeObserver re-asserted width
  on every fire — because it measured `entry.contentRect.width` of
  the `.overlay-root` (which stretches to fill the window), the
  computed desired width equaled current window width + 50, defeating
  every user drag attempt. Switched to a **grow-only** policy:
  - Width is derived from `overlay-bar.scrollWidth` (intrinsic content
    extent, not container width) + 50 px buffer.
  - Width only `setSize` when `intrinsic > current + 4` (i.e. when
    chips overflow the current bar). Never shrinks.
  - User can drag wider freely. User can't drag narrower than
    intrinsic-content (auto-grows back), which is the correct lower
    bound — chips would overflow otherwise.
  - CSS: added `.overlay-bar > * { flex-shrink: 0; }` so bar children
    keep their natural size (without this, flex's default shrink would
    let scrollWidth equal offsetWidth always, defeating the intrinsic
    measurement).
  - Height continues to auto-grow for transcript-tail / answer-bubble.

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
