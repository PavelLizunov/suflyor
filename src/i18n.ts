// src/i18n.ts — v0.0.42 typed translation map.
//
// Design choice per docs/I18N_PLAN.md: a single typed strings map with
// `t(key, lang)` helper. TS catches missing keys at compile time, single
// file to scan. Per-component locale objects were the alternative but
// for a 1-developer project a central registry beats scatter.
//
// SCOPE — v0.0.42 ships only the visible-always strings:
//   - Settings header (Settings, Quit)
//   - Settings footer (Back to overlay, Save, Saved, Session Replay)
//   - Sidebar nav (4 groups + 10 items)
//   - Search placeholder, quit confirm
//
// Per-panel content strings (~360 total) roll out in v0.0.43 → v0.0.45
// per the I18N_PLAN.md release schedule.

export type Lang = "ru" | "en";

/// Resolve a Lang from a raw config string (may be anything; fallback to "ru").
export function resolveLang(raw: string | undefined | null): Lang {
  return raw === "en" ? "en" : "ru";
}

const strings = {
  // ── Header ──────────────────────────────────────────────────────────
  "settings.title":           { ru: "Settings",          en: "Settings" },
  "settings.quit":            { ru: "✕ Выйти",           en: "✕ Quit" },
  "settings.quit.tip":        {
    ru: "Полностью завершить suflyor (с подтверждением)",
    en: "Quit suflyor entirely (with confirmation)",
  },
  "settings.quit.confirm":    {
    ru: "Выйти из приложения? Текущая сессия захвата завершится, journal сохранится.",
    en: "Quit application? The current capture session will end, journal will be saved.",
  },
  "settings.quit.confirm.label": { ru: "Выйти",          en: "Quit" },
  "settings.quit.failed":     { ru: "quit failed",       en: "quit failed" },

  // ── Footer ──────────────────────────────────────────────────────────
  "settings.back":            { ru: "← К overlay",       en: "← Back to overlay" },
  "settings.save":            { ru: "Сохранить",         en: "Save" },
  "settings.saved":           { ru: "✓ Сохранено",       en: "✓ Saved" },
  "settings.replay":          { ru: "📊 Session Replay", en: "📊 Session Replay" },

  // ── Sidebar nav: groups ─────────────────────────────────────────────
  "nav.group.session":        { ru: "СЕССИЯ",            en: "SESSION" },
  "nav.group.ai":             { ru: "AI",                en: "AI" },
  "nav.group.logic":          { ru: "ЛОГИКА",            en: "LOGIC" },
  "nav.group.app":            { ru: "ПРИЛОЖЕНИЕ",        en: "APP" },

  // ── Sidebar nav: items ──────────────────────────────────────────────
  "nav.profile":              { ru: "Профиль и контекст", en: "Profile & context" },
  "nav.audio":                { ru: "Аудио и STT",        en: "Audio & STT" },
  "nav.ai":                   { ru: "AI мост · модели · бюджет", en: "AI bridge · models · budget" },
  "nav.tiles":                { ru: "Авто-тайлы и сниппеты", en: "Auto-tiles & snippets" },
  "nav.knowledge":            { ru: "База знаний",        en: "Knowledge base" },
  "nav.coaching":             { ru: "Коучинг",            en: "Coaching" },
  "nav.interface":            { ru: "Интерфейс",          en: "Interface" },
  "nav.stealth":              { ru: "Скрытность",         en: "Stealth" },
  "nav.hotkeys":              { ru: "Хоткеи",             en: "Hotkeys" },
  "nav.advanced":             { ru: "Обновления · диагностика", en: "Updates · diagnostics" },
  "nav.stats":                { ru: "Статистика",              en: "Stats" },

  // Stats dashboard (v0.0.60)
  "stats.title":              { ru: "📊 Статистика по всем сессиям", en: "📊 Stats across all sessions" },
  "stats.refresh":            { ru: "🔄 Обновить",             en: "🔄 Refresh" },
  "stats.busy":               { ru: "⏳ Читаю…",                en: "⏳ Reading…" },
  "stats.refresh.tip":        {
    ru: "Перечитать все JSONL в %APPDATA%\\overlay-mvp\\sessions",
    en: "Re-read all JSONL in %APPDATA%\\overlay-mvp\\sessions",
  },
  "stats.empty":              { ru: "Сессий нет — стартани одну в overlay.", en: "No sessions yet — start one from the overlay." },
  "stats.summary.title":      { ru: "🧮 Суммарно",             en: "🧮 Summary" },
  "stats.row.sessions":       { ru: "Сессий всего",            en: "Total sessions" },
  "stats.closed":             { ru: "закрытых",                en: "closed" },
  "stats.row.duration":       { ru: "Время записи",            en: "Total runtime" },
  "stats.row.ai":             { ru: "AI запросов",             en: "AI requests" },
  "stats.row.tiles":          { ru: "Тайлов заспавнено",       en: "Tiles spawned" },
  "stats.row.cost":           { ru: "Общая стоимость (USD)",   en: "Total cost (USD)" },
  "stats.daily.title":        { ru: "📅 Сессии за последние 30 дней", en: "📅 Sessions over last 30 days" },
  "stats.daily.hint":         {
    ru: "По колонке за день, слева — старее. Tooltip показывает дату + число.",
    en: "One bar per day, oldest on the left. Hover for date + count.",
  },
  "stats.top.title":          { ru: "🔥 Топ-5 повторяющихся вопросов", en: "🔥 Top-5 recurring questions" },

  // ── Nav misc ────────────────────────────────────────────────────────
  "nav.filter.placeholder":   { ru: "фильтр…",            en: "filter…" },
  "nav.filter.aria":          {
    ru: "Фильтр секций настроек",
    en: "Filter settings sections",
  },
  "nav.aria.settings":        { ru: "Настройки",          en: "Settings sections" },
  "nav.aria.pane":            {
    ru: "Активная секция настроек",
    en: "Active settings panel",
  },

  // ── Interface panel: language switcher (v0.0.42 minimal addition) ──
  "interface.language.title":     { ru: "🌐 Язык интерфейса", en: "🌐 UI language" },
  "interface.language.desc":      {
    ru: "Переключает Settings + overlay UI. AI-ответы и transcript — отдельный язык (см. AI → Язык ответов).",
    en: "Switches Settings + overlay UI. AI responses + transcript are a separate language (see AI → Response language).",
  },
  "interface.language.ru":        { ru: "Русский",          en: "Russian" },
  "interface.language.en":        { ru: "English",          en: "English" },

  // ── Common labels reused (toast types) ─────────────────────────────
  "common.ok":                { ru: "ОК",                en: "OK" },
  "common.cancel":            { ru: "Отмена",            en: "Cancel" },

  // ── Stealth panel (v0.0.43) ────────────────────────────────────────
  "stealth.card.title":       { ru: "🎯 Screen-share поведение", en: "🎯 Screen-share behaviour" },
  "stealth.switch.title":     {
    ru: "Скрыть overlay + tiles от screen share",
    en: "Hide overlay + tiles from screen share",
  },
  "stealth.switch.desc":      {
    ru: "Windows 10 2004+: SetWindowDisplayAffinity (WDA_EXCLUDEFROMCAPTURE). Применяется сразу — restart не нужен. Не работает в OBS с режимом «window capture»; работает в Zoom/Teams/Meet.",
    en: "Windows 10 2004+: SetWindowDisplayAffinity (WDA_EXCLUDEFROMCAPTURE). Applied immediately — no restart needed. Doesn't work in OBS with «window capture» mode; works in Zoom/Teams/Meet.",
  },
  "stealth.switch.aria":      { ru: "Переключить stealth-режим", en: "Toggle stealth mode" },
  "stealth.banner":           {
    ru: "Тест: пошарь экран в Teams/Meet, спроси коллегу видит ли он overlay. Если да — graphics driver не поддерживает WDA_EXCLUDEFROMCAPTURE; используй overlay на втором мониторе.",
    en: "Test: share screen in Teams/Meet, ask a colleague if they see the overlay. If yes — graphics driver doesn't support WDA_EXCLUDEFROMCAPTURE; use overlay on a second monitor.",
  },

  // ── Coaching panel (v0.0.43) ───────────────────────────────────────
  "coaching.card.title":      { ru: "🎓 Post-meeting debrief", en: "🎓 Post-meeting debrief" },
  "coaching.switch.title":    {
    ru: "Coaching tile после Stop (opt-in)",
    en: "Coaching tile after Stop (opt-in)",
  },
  "coaching.switch.desc":     {
    ru: "После Stop session AI шлёт mic-транскрипт в Sonnet и возвращает 3 коротких замечания о вашей речи (темп, паразиты, структура). Срабатывает только если сессия ≥30 сек и было ≥5 mic-реплик. Стоит ~1 Sonnet вызов (≈$0.005). Не забудь Save.",
    en: "After Stop session AI sends the mic transcript to Sonnet and returns 3 short notes about your speech (pace, fillers, structure). Triggers only when session ≥30 sec and ≥5 mic lines. Costs ~1 Sonnet call (≈$0.005). Don't forget to Save.",
  },
  "coaching.switch.aria":     { ru: "Переключить post-meeting debrief", en: "Toggle post-meeting debrief" },

  // ── Interface panel polish (v0.0.43) ───────────────────────────────
  "interface.cost.title":     {
    ru: "🎨 Внешний вид overlay",
    en: "🎨 Overlay appearance",
  },
  "interface.cost.switch.title": {
    ru: "Показывать индикатор стоимости 💰",
    en: "Show cost indicator 💰",
  },
  "interface.cost.switch.desc":  {
    ru: "Шильдик «💰 $X.XXX» в overlay-баре. Скрытие НЕ отключает учёт — деньги всё равно пишутся в журнал и cost:update event летает. Только убирает шильдик из бара.",
    en: "The «💰 $X.XXX» chip in the overlay bar. Hiding it does NOT disable accounting — money still goes to the journal and cost:update events still fire. Only removes the chip from the bar.",
  },
  "interface.cost.switch.aria":  { ru: "Переключить индикатор стоимости", en: "Toggle cost indicator" },

  // v0.0.55: compact overlay + tile font size
  "interface.compact.title":     { ru: "🤏 Компактный режим overlay", en: "🤏 Compact overlay mode" },
  "interface.compact.switch.title": { ru: "Сжать overlay-бар", en: "Compact overlay bar" },
  "interface.compact.switch.desc":  {
    ru: "Прячет чипы 💰 / 🎙 wpm / 📸 ready / ⏱ / 🔥 — оставляет статус + dot + push-to-talk + шестерёнку. Бар становится узким, занимает меньше места над окном собеседования.",
    en: "Hides 💰 / 🎙 wpm / 📸 ready / ⏱ / 🔥 chips — leaves status + dot + push-to-talk + gear. Bar becomes narrow, takes less space above the meeting window.",
  },
  "interface.compact.switch.aria":  { ru: "Переключить компактный режим", en: "Toggle compact mode" },
  "interface.tilefs.title":      { ru: "📐 Шрифт в тайлах", en: "📐 Tile font size" },
  "interface.tilefs.label":      { ru: "Размер шрифта", en: "Font size" },
  "interface.tilefs.hint":       {
    ru: "Размер тела markdown в каждом тайле (px). Применяется к новым тайлам.",
    en: "Tile body markdown font size (px). Applies to newly spawned tiles.",
  },

  // ── Hotkeys panel (v0.0.43) ────────────────────────────────────────
  "hotkeys.card.title":       { ru: "⌨ Глобальные хоткеи", en: "⌨ Global hotkeys" },
  "hotkeys.hint":             {
    ru: "Регистрируются как global hotkeys Windows — работают когда overlay не в фокусе. Формат: «F9» / «Ctrl+Shift+A» (Tauri syntax). После Save потребуется restart сессии чтобы перерегистрировать.",
    en: "Registered as Windows global hotkeys — work when overlay isn't focused. Format: «F9» / «Ctrl+Shift+A» (Tauri syntax). After Save, restart the session to re-register.",
  },
  "hotkeys.ask.label":        { ru: "Ask AI",              en: "Ask AI" },
  "hotkeys.ask.hint":         {
    ru: "Спросить AI сейчас (со screenshot если есть)",
    en: "Ask AI now (with screenshot if present)",
  },
  "hotkeys.screenshot.label": { ru: "Take screenshot",     en: "Take screenshot" },
  "hotkeys.screenshot.hint":  {
    ru: "Захват экрана для следующего F9",
    en: "Screen capture for the next F9",
  },
  "hotkeys.toggle.label":     { ru: "Toggle visibility",   en: "Toggle visibility" },
  "hotkeys.toggle.hint":      {
    ru: "PANIC HIDE — скрыть overlay + все тайлы",
    en: "PANIC HIDE — hide overlay + all tiles",
  },
  "hotkeys.pause.label":      { ru: "Pause audio",         en: "Pause audio" },
  "hotkeys.pause.hint":       {
    ru: "Пауза/возобновить сессию (F8)",
    en: "Pause/resume session (F8)",
  },

  // ── AI panel (v0.0.44) ─────────────────────────────────────────────
  // Bridge card
  "ai.bridge.title":          { ru: "🛰 Bridge endpoint",     en: "🛰 Bridge endpoint" },
  "ai.bridge.url.label":      { ru: "Base URL",               en: "Base URL" },
  "ai.bridge.url.hint":       {
    ru: "OpenAI-compatible Claude proxy. Local bridge или Caddy-fronted Anthropic.",
    en: "OpenAI-compatible Claude proxy. Local bridge or Caddy-fronted Anthropic.",
  },
  "ai.bridge.bearer.label":   { ru: "Bearer secret",          en: "Bearer secret" },
  "ai.bridge.bearer.hint":    {
    ru: "BRIDGE_SECRET — хранится в config.json, не отправляется в журнал.",
    en: "BRIDGE_SECRET — stored in config.json, never sent to journal.",
  },
  "ai.bridge.health.label":   { ru: "Health check",           en: "Health check" },
  "ai.bridge.health.hint":    {
    ru: "1-токен POST на /chat/completions — проверяет URL + bearer + сетевой путь.",
    en: "1-token POST to /chat/completions — checks URL + bearer + network path.",
  },
  "ai.bridge.check.button":   { ru: "🔌 Проверить мост",       en: "🔌 Check bridge" },
  "ai.bridge.check.busy":     { ru: "⏳ Проверяю…",            en: "⏳ Checking…" },
  "ai.bridge.check.tip":      {
    ru: "Минимальный 1-токен POST на /chat/completions",
    en: "Minimal 1-token POST to /chat/completions",
  },
  "ai.bridge.fail.tip":       {
    ru: "💡 Проверь: запущен ли мост на этом IP/порту, открыт ли firewall, не сменился ли BRIDGE_SECRET.",
    en: "💡 Check: is the bridge running on this IP/port, is the firewall open, did BRIDGE_SECRET change.",
  },
  "ai.bridge.warn.http":      {
    ru: "⚠ Plaintext HTTP к non-localhost — bearer token + промпты летят в открытом виде. Используй https:// (Caddy/Nginx впереди) для любого non-localhost deployment.",
    en: "⚠ Plaintext HTTP to non-localhost — bearer token + prompts travel in clear. Use https:// (Caddy/Nginx in front) for any non-localhost deployment.",
  },

  // Models card
  "ai.models.title":          { ru: "🧠 Модели + язык",        en: "🧠 Models + language" },
  "ai.models.live.label":     { ru: "Живые ответы",           en: "Live answers" },
  "ai.models.live.hint":      {
    ru: "Эта модель работает на каждый тайл. Нужна скорость.",
    en: "This model runs on every tile. Needs to be fast.",
  },
  "ai.models.prep.label":     { ru: "Подготовка контекста",   en: "Context prep" },
  "ai.models.prep.hint":      {
    ru: "Структурирование meeting_context, coaching debrief. Нужно качество.",
    en: "Structuring meeting_context, coaching debrief. Needs quality.",
  },
  "ai.models.lang.label":     { ru: "Язык ответов",           en: "Response language" },
  "ai.models.lang.hint":      {
    ru: "Принудительно через system prompt. Whisper может транскрибировать на другом языке.",
    en: "Forced via system prompt. Whisper may transcribe in a different language.",
  },

  // Budget card
  "ai.budget.title":          { ru: "💰 Лимит затрат на сессию", en: "💰 Per-session budget cap" },
  "ai.budget.cap.label":      { ru: "Cap (USD)",              en: "Cap (USD)" },
  "ai.budget.cap.hint":       {
    ru: "0 = выкл (default с v0.0.28). Любое положительное значение — жёлтый 💰 чип в overlay-bar когда сессия превысит.",
    en: "0 = off (default since v0.0.28). Any positive value lights up a yellow 💰 chip in the overlay bar when the session exceeds it.",
  },
  "ai.budget.note":           {
    ru: "Для справки: $1 ≈ 200 Haiku тайлов · $5 ≈ час непрерывной речи в Aggressive mode. Это SOFT warning — AI продолжает отвечать после превышения, чип просто загорается.",
    en: "Reference: $1 ≈ 200 Haiku tiles · $5 ≈ one hour of continuous speech in Aggressive mode. This is a SOFT warning — AI keeps responding after the cap, the chip just lights up.",
  },

  // Detector card
  "ai.det.title":             { ru: "🎯 Триггер на спавн тайла", en: "🎯 Tile-spawn trigger" },
  "ai.det.skip.title":        { ru: "Игнорировать ваш голос (mic)", en: "Ignore your own voice (mic)" },
  "ai.det.skip.desc":         {
    ru: "ON по умолчанию. Только вопросы собеседника триггерят auto-tile. Без этого детектор фаерит на ваших фразах типа «Я работал с Kubernetes…» — лишние тайлы. Выключи только если хочешь подсказки по обеим сторонам.",
    en: "ON by default. Only the other side's questions trigger auto-tiles. Without this the detector fires on your own phrases like «I worked with Kubernetes…» — extra tiles. Disable only if you want hints from both sides.",
  },
  "ai.det.skip.aria":         { ru: "Переключить skip-mic детектора", en: "Toggle detector skip-mic" },
  "ai.det.agg.title":         { ru: "🔥 Aggressive mode",     en: "🔥 Aggressive mode" },
  "ai.det.agg.desc":          {
    ru: "Спавнить тайл на КАЖДУЮ строку транскрипта (v0.0.18+). OFF по умолчанию. Bypass'ит «вопрос/не вопрос» проверку — каждая строка от Whisper (длиннее 5 символов) → тайл. Rate-limit бампается с 15 до 60 тайлов/мин. Overlay-бар покажет 🔥 чип когда включён — будешь видеть статус.",
    en: "Spawn a tile on EVERY transcript line (v0.0.18+). OFF by default. Bypasses the question/non-question check — every Whisper line (longer than 5 chars) → tile. Rate limit bumps from 15 to 60 tiles/min. The overlay bar shows a 🔥 chip when enabled — you'll see the status.",
  },
  "ai.det.agg.aria":          { ru: "Переключить aggressive mode", en: "Toggle aggressive mode" },

  // ── Profile panel (v0.0.45) ────────────────────────────────────────
  "profile.profiles.title":   { ru: "👥 Профили контекста",   en: "👥 Context profiles" },
  "profile.active.label":     { ru: "Активный профиль",       en: "Active profile" },
  "profile.none":             { ru: "— нет —",                en: "— none —" },
  "profile.save.button":      { ru: "+ Сохранить текущий как профиль", en: "+ Save current as profile" },
  "profile.delete.button":    { ru: "× Удалить активный",     en: "× Delete active" },
  "profile.prompt.name":      { ru: "Имя нового профиля",     en: "Name of new profile" },
  "profile.prompt.placeholder": { ru: "K8s interview, Backend SRE, …", en: "K8s interview, Backend SRE, …" },
  "profile.saved.toast":      { ru: "Профиль «{name}» сохранён", en: "Profile «{name}» saved" },
  "profile.deleted.toast":    { ru: "Профиль «{name}» удалён", en: "Profile «{name}» deleted" },
  "profile.delete.confirm":   { ru: "Удалить профиль «{name}»?", en: "Delete profile «{name}»?" },
  "common.delete":            { ru: "Удалить",                en: "Delete" },

  // ── Meeting context (v0.0.45) ──────────────────────────────────────
  "meeting.title":            { ru: "📝 Meeting context",     en: "📝 Meeting context" },
  "meeting.label":            {
    ru: "Контекст которой AI видит при каждом запросе (резюме, описание проекта, термины…)",
    en: "Context the AI sees on every request (resume, project description, terms…)",
  },
  "meeting.placeholder":      {
    ru: "Например: Это собеседование на Senior SRE в Acme. Мой опыт: 7 лет K8s, etcd, networking…",
    en: "Example: Senior SRE interview at Acme. My experience: 7 years K8s, etcd, networking…",
  },
  "meeting.record.button":    { ru: "🎤 Записать голосом ({sec}с)", en: "🎤 Record voice ({sec}s)" },
  "meeting.record.busy":      { ru: "🔴 Идёт запись… {sec}с",  en: "🔴 Recording… {sec}s" },
  "meeting.record.tip":       {
    ru: "Запишет с микрофона 30 секунд и добавит транскрипт в поле выше",
    en: "Records 30 seconds from the microphone and appends transcript to the field above",
  },
  "meeting.structure.button": { ru: "✨ Структурировать ({model})", en: "✨ Structure ({model})" },
  "meeting.structure.busy":   { ru: "✨ Структурирую через Sonnet…", en: "✨ Structuring via Sonnet…" },
  "meeting.structure.tip":    {
    ru: "Отправит текст в {model} с промтом структурирования и заменит на чистый контекст",
    en: "Sends text to {model} with a structuring prompt and replaces with cleaned context",
  },

  // ── Audio panel (v0.0.45) ──────────────────────────────────────────
  "audio.devices.title":      { ru: "🎤 Audio devices",       en: "🎤 Audio devices" },
  "audio.mic.label":          { ru: "Microphone (your voice)", en: "Microphone (your voice)" },
  "audio.mic.default":        { ru: "— default —",            en: "— default —" },
  "audio.sys.label":          {
    ru: "System audio (what they say) — выбери loopback устройство (для Astro A50: \"Line (A50 Stream Out)\")",
    en: "System audio (what they say) — pick a loopback device (for Astro A50: \"Line (A50 Stream Out)\")",
  },
  "audio.sys.default":        {
    ru: "— default render endpoint loopback —",
    en: "— default render endpoint loopback —",
  },
  "audio.loopback.suffix":    { ru: "(loopback)", en: "(loopback)" },
  "audio.stt.title":          { ru: "🎙 STT (Groq Whisper)",  en: "🎙 STT (Groq Whisper)" },
  "audio.stt.key.label":      { ru: "Groq API key (gsk_…)",   en: "Groq API key (gsk_…)" },
  "audio.stt.lang.label":     {
    ru: "Язык (пусто = auto-detect)",
    en: "Language (empty = auto-detect)",
  },
  "audio.stt.lang.placeholder": { ru: "ru, en, …",            en: "ru, en, …" },
  "audio.stt.model.label":    {
    ru: "Whisper model — точность ↔ скорость tradeoff",
    en: "Whisper model — accuracy ↔ speed tradeoff",
  },
  "audio.stt.model.large":    {
    ru: "whisper-large-v3 (default — лучшая точность на терминах)",
    en: "whisper-large-v3 (default — best accuracy on terms)",
  },
  "audio.stt.model.turbo":    {
    ru: "whisper-large-v3-turbo (≈3× быстрее, слегка хуже на редких словах)",
    en: "whisper-large-v3-turbo (≈3× faster, slightly worse on rare words)",
  },
  "audio.stt.note":           {
    ru: "Turbo сокращает latency Whisper-вызова с ~500ms до ~150-200ms на 2-5s клипе. Качество падает на редких технических терминах (kubectl-debug, consistent hashing). Для типовых SRE/DevOps вопросов разница незаметна. Меняй при необходимости low-latency feedback.",
    en: "Turbo cuts Whisper-call latency from ~500ms to ~150-200ms on a 2-5s clip. Quality drops on rare technical terms (kubectl-debug, consistent hashing). For typical SRE/DevOps questions the difference is unnoticeable. Switch when you need low-latency feedback.",
  },

  // ── Auto-tiles panel (v0.0.46) ─────────────────────────────────────
  "tiles.auto.title":         { ru: "🪟 Авто-тайлы",          en: "🪟 Auto-tiles" },
  "tiles.auto.switch.title":  {
    ru: "Включить авто-окошки при вопросах в транскрипте",
    en: "Enable auto-windows on transcript questions",
  },
  "tiles.auto.switch.desc":   {
    ru: "Когда детектор видит вопрос (или любая строка в Aggressive mode) — спавнится тайл рядом с meeting window.",
    en: "When the detector sees a question (or any line in Aggressive mode), a tile spawns next to the meeting window.",
  },
  "tiles.auto.switch.aria":   { ru: "Переключить авто-тайлы", en: "Toggle auto-tiles" },
  "tiles.monitor.label":      { ru: "Монитор для tiles",       en: "Monitor for tiles" },
  "tiles.monitor.hint":       {
    ru: "по умолчанию — первый не-primary; если монитор один — primary",
    en: "default — first non-primary; if only one monitor — primary",
  },
  "tiles.monitor.auto":       {
    ru: "— авто (предпочитать не-primary) —",
    en: "— auto (prefer non-primary) —",
  },
  "tiles.keywords.label":     { ru: "Trigger-keywords",        en: "Trigger keywords" },
  "tiles.keywords.hint":      {
    ru: "через пробел, case-insensitive, whole-word match. Срабатывают как дополнительный триггер на спавн.",
    en: "space-separated, case-insensitive, whole-word match. Act as an extra spawn trigger.",
  },

  // ── Knowledge base panel (v0.0.46) ─────────────────────────────────
  "kb.title":                 { ru: "📚 Knowledge Base",       en: "📚 Knowledge Base" },
  "kb.stats":                 {
    ru: "{total} entries ({glossary} glossary · {commands} commands · {patterns} patterns)",
    en: "{total} entries ({glossary} glossary · {commands} commands · {patterns} patterns)",
  },
  "kb.search.label":          {
    ru: "Поиск по встроенной базе (термины + команды + паттерны). Хит → Open as tile.",
    en: "Search the embedded base (terms + commands + patterns). Hit → Open as tile.",
  },
  "kb.search.placeholder":    {
    ru: "kubernetes / dijkstra / saga / iptables / consistent hashing …",
    en: "kubernetes / dijkstra / saga / iptables / consistent hashing …",
  },
  "kb.searching":             { ru: "ищу…",                    en: "searching…" },
  "kb.no.match":              { ru: "нет совпадений по «{q}»", en: "no matches for «{q}»" },
  "kb.open.button":           { ru: "Открыть →",               en: "Open →" },
  "kb.open.tip":              { ru: "Открыть тайл с записью «{h}»", en: "Open tile with entry «{h}»" },
  "kb.opened.toast":          { ru: "Открыт тайл «{h}»",       en: "Opened tile «{h}»" },
  "kb.spawn.fail.toast":      { ru: "kb_spawn failed",         en: "kb_spawn failed" },
  "kb.note":                  {
    ru: "KB файлы embedded в бинарник (read-only). Источники: src-tauri/knowledge/{glossary,commands,patterns}.md.",
    en: "KB files embedded in the binary (read-only). Sources: src-tauri/knowledge/{glossary,commands,patterns}.md.",
  },
  "kb.source.aria":           { ru: "источник: {s}",           en: "source: {s}" },

  // ── Advanced panel (v0.0.47) ───────────────────────────────────────
  // Section title
  "adv.updates.title":        { ru: "🆙 Обновления",          en: "🆙 Updates" },
  // Check button
  "adv.check.button":         { ru: "🔍 Проверить обновления", en: "🔍 Check for updates" },
  "adv.check.busy":           { ru: "⏳ Проверяю…",            en: "⏳ Checking…" },
  "adv.check.tip":            {
    ru: "Проверить GitHub Releases на новую версию",
    en: "Check GitHub Releases for a new version",
  },
  "adv.check.toast.new":      { ru: "Доступна v{latest} (у вас v{current})", en: "v{latest} available (you have v{current})" },
  "adv.check.toast.same":     { ru: "Актуальная версия (v{current})", en: "Up to date (v{current})" },
  "adv.check.toast.err":      { ru: "Update check: {err}",     en: "Update check: {err}" },
  "adv.check.toast.fail":     { ru: "Update check failed: {err}", en: "Update check failed: {err}" },
  // Current/latest line
  "adv.current.label":        { ru: "Текущая: v{v}",           en: "Current: v{v}" },
  "adv.latest.suffix":        { ru: " · последняя: v{v}",      en: " · latest: v{v}" },
  // Update available banner
  "adv.available.title":      { ru: "✨ Доступна v{latest}",   en: "✨ v{latest} available" },
  "adv.available.notes":      { ru: "Release notes",           en: "Release notes" },
  "adv.available.upToDate":   { ru: "✓ У вас актуальная версия v{current}.", en: "✓ You're on the latest version v{current}." },
  // Download buttons
  "adv.download.button":      { ru: "🚀 Скачать и установить (one-click)", en: "🚀 Download & install (one-click)" },
  "adv.download.busy":        { ru: "⏳ Скачиваю…",            en: "⏳ Downloading…" },
  "adv.download.tip":         {
    ru: "Скачивает NSIS установщик и запускает его. Программа закроется, инсталлер заменит файлы и поднимет новую версию. UAC prompt будет.",
    en: "Downloads the NSIS installer and runs it. The app will close, the installer replaces files and launches the new version. UAC prompt will appear.",
  },
  "adv.download.toast.start": { ru: "⬇ Скачиваю установщик…", en: "⬇ Downloading installer…" },
  "adv.download.toast.ok":    {
    ru: "✓ Установщик запущен ({file}). Программа закроется через 2 сек, дальше следуй за UAC + NSIS подсказками.",
    en: "✓ Installer started ({file}). The app will close in 2 sec, then follow the UAC + NSIS prompts.",
  },
  "adv.download.toast.fail":  { ru: "Ошибка обновления: {err}", en: "Update error: {err}" },
  "adv.download.toast.stuck": {
    ru: "Не удалось выйти — закрой программу вручную, установщик в %TEMP%",
    en: "Couldn't quit — close the app manually, installer is in %TEMP%",
  },
  "adv.browser.button":       { ru: "⬇ Открыть в браузере",   en: "⬇ Open in browser" },
  "adv.browser.tip":          {
    ru: "Альтернативно: откроет страницу релиза в браузере — скачай MSI/EXE и запусти руками",
    en: "Alternative: opens the release page in browser — download MSI/EXE and run manually",
  },
  "adv.browser.toast.fail":   { ru: "Не удалось открыть браузер: {err}", en: "Couldn't open browser: {err}" },
  "adv.smartscreen.note":     {
    ru: "Без code signing — SmartScreen может предупредить «Unknown publisher». Жми More info → Run anyway. Установщик заменит старую версию, config сохранится.",
    en: "No code signing — SmartScreen may warn «Unknown publisher». Click More info → Run anyway. The installer replaces the old version, config is preserved.",
  },
  "adv.update.note":          {
    ru: "Запрос идёт на api.github.com (1 KB JSON, ~200ms). Авто-проверки нет — только когда жмёшь.",
    en: "Hits api.github.com (1 KB JSON, ~200ms). No auto-check — only when you click.",
  },
  // Crash report
  "adv.crash.title":          { ru: "⚠ Найден crash-report",  en: "⚠ Crash report found" },
  "adv.crash.desc":           { ru: "Прошлый запуск упал на startup. Файл: {path}", en: "Previous launch crashed on startup. File: {path}" },
  "adv.crash.button":         { ru: "📨 Открыть в Notepad",   en: "📨 Open in Notepad" },
  "adv.crash.tip":            {
    ru: "Открыть в Блокноте — посмотри что упало",
    en: "Open in Notepad — see what crashed",
  },
  "adv.crash.toast.fail":     { ru: "Не открылось: {err}",     en: "Couldn't open: {err}" },
  // Diagnostic dump
  "adv.dump.button":          { ru: "📊 Диагностический дамп", en: "📊 Diagnostic dump" },
  "adv.dump.tip":             {
    ru: "Сохранить sanitized config + последние 50 событий журнала + crash report (если есть) одним .md файлом на Desktop — приложи к bug report",
    en: "Save sanitized config + last 50 journal events + crash report (if any) as a single .md on Desktop — attach to bug report",
  },
  "adv.dump.toast.ok":        { ru: "Диагностика сохранена: {path}", en: "Diagnostics saved: {path}" },
  "adv.dump.toast.fail":      { ru: "Не получилось: {err}",   en: "Failed: {err}" },
  "adv.dump.note":            {
    ru: "Сохраняет на Desktop. Секреты (groq_api_key, ai_bearer, ai_base_url, meeting_context, profiles) обнулены.",
    en: "Saves to Desktop. Secrets (groq_api_key, ai_bearer, ai_base_url, meeting_context, profiles) blanked.",
  },
  // Sessions / export / import
  "adv.sessions.label":       { ru: "Сессии и экспорт конфига", en: "Sessions and config export" },
  "adv.replay.button":        { ru: "📊 Replay",               en: "📊 Replay" },
  "adv.replay.tip":           {
    ru: "In-app просмотрщик session journals — timeline transcript/AI/detector/tiles",
    en: "In-app viewer for session journals — transcript/AI/detector/tiles timeline",
  },
  "adv.logs.button":          { ru: "📁 Логи сессий",         en: "📁 Session logs" },
  "adv.logs.tip":             {
    ru: "JSONL логи всех transcript/AI/detector событий по сессиям",
    en: "JSONL logs of all transcript/AI/detector events per session",
  },
  "adv.export.full.button":   { ru: "💾 Export (full)",       en: "💾 Export (full)" },
  "adv.export.full.tip":      {
    ru: "ПОЛНЫЙ backup на Desktop: snippets + контекст + ключи + URL моста. Для переезда на другую свою машину. НЕ шарь с другими.",
    en: "FULL backup to Desktop: snippets + context + keys + bridge URL. For migrating to your own other machine. Do NOT share with others.",
  },
  "adv.export.full.toast.ok": { ru: "Конфиг сохранён: {path}", en: "Config saved: {path}" },
  "adv.export.share.button":  { ru: "🔐 Export (share)",      en: "🔐 Export (share)" },
  "adv.export.share.tip":     {
    ru: "Shareable export — без groq_api_key, ai_bearer, ai_base_url, meeting_context, context_profiles. Можно отправить другу. Получатель доставит свои ключи + URL моста сам.",
    en: "Shareable export — without groq_api_key, ai_bearer, ai_base_url, meeting_context, context_profiles. Safe to send to a friend. The recipient will plug in their own keys + bridge URL.",
  },
  "adv.export.share.toast.ok": { ru: "Безопасный конфиг (без ключей): {path}", en: "Safe config (without keys): {path}" },
  "adv.export.fail":          { ru: "Ошибка экспорта: {err}", en: "Export error: {err}" },
  "adv.import.button":        { ru: "📥 Import",              en: "📥 Import" },
  "adv.import.tip":           {
    ru: "Открыть Windows Explorer и выбрать .json файл",
    en: "Open Windows Explorer and pick a .json file",
  },
  "adv.import.dialog.title":  { ru: "Выбери config.json для импорта", en: "Pick a config.json to import" },
  "adv.import.filter.json":   { ru: "JSON config",            en: "JSON config" },
  "adv.import.filter.all":    { ru: "Все файлы",              en: "All files" },
  "adv.import.toast.ok":      {
    ru: "Конфиг загружен. Перезапустите session чтобы применить.",
    en: "Config loaded. Restart the session to apply.",
  },
  "adv.import.toast.fail":    { ru: "Ошибка импорта: {err}",  en: "Import error: {err}" },
  "adv.export.note":          {
    ru: "Full export = все настройки + ключи (для миграции на свою машину). Share export = без секретов, безопасно для GitHub issue.",
    en: "Full export = all settings + keys (for migrating to your own machine). Share export = without secrets, safe for GitHub issue.",
  },

  // ── Overlay bar (v0.0.48) ──────────────────────────────────────────
  // Drag tooltip
  "overlay.drag.tip":         {
    ru: "Перетащи за пустую область бара, чтобы подвинуть overlay",
    en: "Drag an empty area of the bar to move the overlay",
  },
  // Status text
  "overlay.status.stopped":   { ru: "Stopped",                en: "Stopped" },
  "overlay.status.paused":    { ru: "⏸ Pause (F8 чтобы возобновить)", en: "⏸ Paused (F8 to resume)" },
  "overlay.status.listening": { ru: "Слушаю",                 en: "Listening" },
  "overlay.status.thinking":  { ru: "Спрашиваю AI…",          en: "Asking AI…" },
  "overlay.status.answering": { ru: "Отвечаю",                en: "Answering" },
  "overlay.status.error":     { ru: "Ошибка: {msg}",          en: "Error: {msg}" },
  "overlay.health.aria":      { ru: "Статус подсистем",       en: "Subsystem health" },
  // Coach pill
  "overlay.coach.tip":        {
    ru: "Voice coach (вы, последние 60 сек):\n  темп: {wpm} wpm ({pace})\n  паразиты: {fillers} / {words} слов{per100}",
    en: "Voice coach (you, last 60s):\n  pace: {wpm} wpm ({pace})\n  fillers: {fillers} / {words} words{per100}",
  },
  // Chips
  "overlay.screenshot.aria":  { ru: "Screenshot готов",       en: "Screenshot ready" },
  "overlay.screenshot.text":  { ru: "📸 готов",               en: "📸 ready" },
  "overlay.aggressive.aria":  { ru: "Aggressive mode включён — тайл на каждую строку транскрипта", en: "Aggressive mode is enabled — tile spawns on every transcript line" },
  "overlay.aggressive.tip":   {
    ru: "🔥 AGGRESSIVE MODE ON — тайл на КАЖДУЮ строку транскрипта (bypass детектора, до 60 тайлов/мин). Отключить: Settings → 🪟 Auto-tiles → снять галку «спавнить тайл на каждую строку»",
    en: "🔥 AGGRESSIVE MODE ON — tile on EVERY transcript line (detector bypass, up to 60 tiles/min). Disable: Settings → 🪟 Auto-tiles → uncheck «spawn tile on every line»",
  },
  "overlay.ratelimit.aria":   { ru: "Rate-limited",           en: "Rate limited" },
  "overlay.overbudget.aria":  { ru: "Сессия превысила настроенный бюджет", en: "Session cost over configured budget" },
  "overlay.overbudget.tip":   {
    ru: "Сессия превысила Soft budget warning (Settings → AI proxy). AI продолжает работать — это passive notice.",
    en: "Session exceeded Soft budget warning (Settings → AI proxy). AI keeps working — this is a passive notice.",
  },
  "overlay.cost.tip":         {
    ru: "Накопленная стоимость сессии (Claude tokens) — переключить в Settings → UI",
    en: "Accumulated session cost (Claude tokens) — toggle in Settings → UI",
  },
  "overlay.cost.aria":        { ru: "Стоимость сессии {usd} долларов", en: "Session cost {usd} dollars" },
  "overlay.hotkey.warn.tip":  { ru: "Проблемы с хоткеями:\n{warnings}", en: "Hotkey issues:\n{warnings}" },
  "overlay.hotkey.warn.aria": { ru: "{n} проблем(ы) с хоткеями", en: "{n} hotkey warning(s)" },
  // PTT buttons
  "overlay.ptt.system.hold":  {
    ru: "Зажми чтобы записать СОБЕСЕДНИКА, отпусти чтобы спросить AI",
    en: "Hold to record the OTHER SIDE, release to ask AI",
  },
  "overlay.ptt.mic.hold":     {
    ru: "Зажми чтобы записать СЕБЯ, отпусти чтобы спросить AI",
    en: "Hold to record YOURSELF, release to ask AI",
  },
  "overlay.ptt.system.click": {
    ru: "Спросить AI про последние реплики СОБЕСЕДНИКА",
    en: "Ask AI about recent OTHER SIDE lines",
  },
  "overlay.ptt.mic.click":    {
    ru: "Спросить AI про последние реплики СЕБЯ",
    en: "Ask AI about recent MICROPHONE lines",
  },
  "overlay.ptt.system.aria.hold":      { ru: "Push-to-talk собеседник", en: "System push-to-talk" },
  "overlay.ptt.system.aria.hold.rec":  { ru: "Push-to-talk собеседник — запись", en: "System push-to-talk — recording" },
  "overlay.ptt.mic.aria.hold":         { ru: "Push-to-talk микрофон",  en: "Microphone push-to-talk" },
  "overlay.ptt.mic.aria.hold.rec":     { ru: "Push-to-talk микрофон — запись", en: "Microphone push-to-talk — recording" },
  "overlay.ptt.system.aria.click":     { ru: "Спросить AI про последние реплики собеседника", en: "Ask AI about recent system lines" },
  "overlay.ptt.mic.aria.click":        { ru: "Спросить AI про последние реплики микрофона", en: "Ask AI about recent microphone lines" },
  "overlay.ptt.hold":         { ru: "удерж.",                 en: "hold" },
  "overlay.ptt.ask":          { ru: "спр.",                   en: "ask" },
  // Help popover button
  "overlay.help.aria":        { ru: "Расшифровка хоткеев — клик чтобы раскрыть", en: "Hotkey legend — click to expand" },
  "overlay.help.tip":         { ru: "Click для расшифровки всех hotkey'ев", en: "Click to expand all hotkey descriptions" },
  // Settings gear
  "overlay.gear.tip":         { ru: "Настройки",              en: "Settings" },
  "overlay.gear.aria":        { ru: "Открыть настройки",      en: "Open settings" },
  // Help popover content
  "overlay.help.dialog.aria": { ru: "Справка по хоткеям",     en: "Hotkey reference" },
  "overlay.help.hk.title":    {
    ru: "Хоткеи (global) — клик в любом месте чтобы закрыть",
    en: "Hotkeys (global) — click anywhere to close",
  },
  "overlay.help.hk.f3":       { ru: "Reask — повторить последний вопрос со свежим контекстом", en: "Reask — repeat the last question with fresh context" },
  "overlay.help.hk.f4":       { ru: "KB palette — поиск в knowledge base (1643 entries)", en: "KB palette — search the knowledge base (1643 entries)" },
  "overlay.help.hk.f6":       { ru: "Manual tile — спавнить тайл из последней реплики", en: "Manual tile — spawn a tile from the last line" },
  "overlay.help.hk.f8":       { ru: "Pause / Resume — пауза/возобновить сессию", en: "Pause / Resume — pause/resume the session" },
  "overlay.help.hk.f9":       { ru: "Ask AI — спросить AI сейчас (со screenshot если есть)", en: "Ask AI — ask AI now (with screenshot if present)" },
  "overlay.help.hk.f10":      { ru: "Screenshot — захват для следующего F9", en: "Screenshot — capture for the next F9" },
  "overlay.help.hk.f11":      { ru: "PANIC HIDE — скрыть overlay + все тайлы", en: "PANIC HIDE — hide overlay + all tiles" },
  "overlay.help.hk.ctrl_w":   { ru: "Close all tiles (кроме pinned)", en: "Close all tiles (except pinned)" },
  // Indicators legend
  "overlay.help.ind.title":   {
    ru: "Индикаторы — что значат точки и чипы",
    en: "Indicators — what the dots and chips mean",
  },
  "overlay.help.ind.audio":   {
    ru: "🟢 audio — capture работает (зелёный = ok, жёлтый = thinking, серый = idle, красный = error)",
    en: "🟢 audio — capture working (green = ok, yellow = thinking, gray = idle, red = error)",
  },
  "overlay.help.ind.stt":     {
    ru: "🟢 stt — Whisper транскрибирует (loops каждые 2-5 сек)",
    en: "🟢 stt — Whisper transcribing (loops every 2-5 sec)",
  },
  "overlay.help.ind.ai":      {
    ru: "🟢 ai — Claude отвечает на тайлах (purple flash = active request)",
    en: "🟢 ai — Claude responding on tiles (purple flash = active request)",
  },
  "overlay.help.ind.mic":     {
    ru: "🎙 wpm — voice coach: ваш темп речи + filler-words за 60 сек (mic only)",
    en: "🎙 wpm — voice coach: your speech pace + filler-words over 60 sec (mic only)",
  },
  "overlay.help.ind.screenshot": {
    ru: "📸 ready — screenshot захвачен (F10) и прикрепится к следующему F9 ask",
    en: "📸 ready — screenshot captured (F10) and will attach to the next F9 ask",
  },
  "overlay.help.ind.aggressive": {
    ru: "🔥 aggressive — bypass-режим, тайл на каждую строку транскрипта (Settings → Auto-tiles)",
    en: "🔥 aggressive — bypass mode, tile on every transcript line (Settings → Auto-tiles)",
  },
  "overlay.help.ind.ratelimit": {
    ru: "⏱ rate-limited — backend временно throttles (3 сек cooldown), AI запросы пропускаются",
    en: "⏱ rate-limited — backend temporarily throttles (3 sec cooldown), AI requests are skipped",
  },
  "overlay.help.ind.overbudget": {
    ru: "💰 over budget — сессия превысила Soft budget warning (Settings → AI proxy). AI работает дальше",
    en: "💰 over budget — session exceeded Soft budget warning (Settings → AI proxy). AI keeps working",
  },
  "overlay.help.ind.cost":    {
    ru: "💰 $X.XXX — накопленная стоимость сессии (Claude tokens). Переключить в Settings → Interface",
    en: "💰 $X.XXX — accumulated session cost (Claude tokens). Toggle in Settings → Interface",
  },
  // KB palette
  // v0.0.70: hint at the `/` prefix for snippet search.
  "overlay.palette.placeholder": {
    ru: "KB поиск или /сниппет (Esc закрыть · Enter раскрыть)",
    en: "KB search or /snippet (Esc to close · Enter to expand)",
  },
  "overlay.palette.aria":     { ru: "Поиск по knowledge base", en: "Knowledge base search" },

  // ── Tile chrome (v0.0.48) ──────────────────────────────────────────
  "tile.close.tip":           { ru: "Закрыть тайл",           en: "Close tile" },
  "tile.close.aria":          { ru: "Закрыть",                en: "Close" },
  "tile.pin.tip":             { ru: "Запинить (отключить авто-закрытие)", en: "Pin (disable auto-close)" },
  "tile.unpin.tip":           { ru: "Открепить",              en: "Unpin" },
  "tile.pin.aria":            { ru: "Запинить тайл",          en: "Pin tile" },
  "tile.unpin.aria":          { ru: "Открепить тайл",         en: "Unpin tile" },
  "tile.source.auto":         { ru: "AUTO · ДЕТЕКТОР",        en: "AUTO · DETECTOR" },
  "tile.source.mic":          { ru: "MIC",                    en: "MIC" },
  "tile.source.system":       { ru: "SYSTEM",                 en: "SYSTEM" },
  "tile.source.manual":       { ru: "ВРУЧНУЮ",                en: "MANUAL" },
  "tile.source.snippet":      { ru: "СНИППЕТ",                en: "SNIPPET" },
  "tile.source.kb":           { ru: "KB",                     en: "KB" },

  // ── Replay viewer (v0.0.49) ────────────────────────────────────────
  "replay.root.aria":         { ru: "Просмотр журнала сессии", en: "Session journal replay viewer" },
  "replay.title":             { ru: "📊 Session Replay",      en: "📊 Session Replay" },
  "replay.session.placeholder": { ru: "— выбери сессию —",    en: "— pick a session —" },
  "replay.session.aria":      { ru: "Выбрать сессию для просмотра", en: "Choose a session to replay" },
  "replay.back.button":       { ru: "← К overlay",            en: "← Back to overlay" },
  "replay.back.aria":         { ru: "Вернуться к overlay",    en: "Return to overlay" },
  "replay.loading":           { ru: "Загрузка…",              en: "Loading…" },
  "replay.empty":             { ru: "Пустая сессия (нет событий).", en: "Empty session (no events)." },
  "replay.no.sessions":       {
    ru: "Сессий пока нет. Стартани сессию из overlay чтобы заполнить этот список.",
    en: "No sessions yet. Start a session from the overlay to populate this list.",
  },
  "replay.filter.label":      { ru: "Фильтр:",                en: "Filter:" },
  "replay.filter.show.tip":   { ru: "Включить {kind}",        en: "Show {kind}" },
  "replay.filter.hide.tip":   { ru: "Скрыть {kind} ({count} событий)", en: "Hide {kind} ({count} events)" },
  "replay.filter.reset":      { ru: "↺ сбросить",             en: "↺ reset" },
  "replay.filter.reset.tip":  { ru: "Показать все события",   en: "Show all events" },
  "replay.footer.events":     { ru: "{n} событий · {ai} AI ответов", en: "{n} events · {ai} AI responses" },
  "replay.footer.cost":       { ru: "Общая стоимость: ${n}",  en: "Total cost: ${n}" },
  "replay.footer.cost.none":  {
    ru: "Общая стоимость: — (ещё не пишется в журнал)",
    en: "Total cost: — (not tracked in journal yet)",
  },
  // Row labels (uppercase by CSS) — Russian variants are short
  "replay.label.start":       { ru: "SESSION START",          en: "SESSION START" },
  "replay.label.stop":        { ru: "SESSION STOP",           en: "SESSION STOP" },
  "replay.label.summary":     { ru: "SUMMARY",                en: "SUMMARY" },
  "replay.label.detect.on":   { ru: "DETECT ✓",               en: "DETECT ✓" },
  "replay.label.detect.off":  { ru: "detect",                 en: "detect" },
  "replay.label.tile":        { ru: "TILE",                   en: "TILE" },
  "replay.label.unknown":     { ru: "unknown",                en: "unknown" },
  // Detector reason text
  "replay.detect.no.trigger": { ru: "нет триггера",           en: "no trigger" },
  // Row body labels
  "replay.row.dur":           { ru: "мин",                    en: "min" },
  "replay.row.lines":         { ru: "{n} строк ({mic}🎤 · {sys}🗣)", en: "{n} lines ({mic}🎤 · {sys}🗣)" },
  "replay.row.detector":      { ru: "детектор: {t} / {total}", en: "detector: {t} / {total}" },
  "replay.row.aitiles":       { ru: "{ai} AI · {tiles} тайлов", en: "{ai} AI · {tiles} tiles" },
  "replay.row.rl":            { ru: " · {n} rate-limited",    en: " · {n} rate-limited" },
  "replay.row.errors":        { ru: " · {n} ошибок",          en: " · {n} errors" },
  "replay.row.screenshot":    { ru: "📎 screenshot",          en: "📎 screenshot" },
  "replay.row.in_tok":        { ru: "~{n} in-tok",            en: "~{n} in-tok" },

  // ── Polish: drag/import/snippet header (v0.0.50) ───────────────────
  "settings.drag.tip":        {
    ru: "Перетащи за этот заголовок чтобы подвинуть окно",
    en: "Drag this header to move the window",
  },
  "settings.dnd.import.bad":  { ru: "Перетащи именно .json (получено: {ext})", en: "Drop a .json file (got: {ext})" },
  "settings.dnd.import.ok":   { ru: "Конфиг загружен через drag-and-drop.", en: "Config loaded via drag-and-drop." },
  "settings.import.error":    { ru: "Ошибка импорта: {err}",  en: "Import error: {err}" },
  "meeting.error.empty":      {
    ru: "Сначала запишите или впишите текст",
    en: "Record voice or type text first",
  },

  // Snippets section header (CRUD modal still deferred)
  "snippets.title":           { ru: "📋 Снипеты ({n}) — готовые ответы (zero cost)", en: "📋 Snippets ({n}) — pre-written answers (zero cost)" },
  "snippets.expand.tip":      { ru: "Развернуть все снипеты", en: "Expand all snippets" },
  "snippets.collapse.tip":    { ru: "Свернуть",               en: "Collapse" },
  "snippets.expand.button":   { ru: "▼ показать",             en: "▼ show" },
  "snippets.collapse.button": { ru: "▲ свернуть",             en: "▲ hide" },
  "snippets.new.button":      { ru: "+ Новый",                en: "+ New" },
  "snippets.new.tip":         {
    ru: "Создать новый snippet (откроется 3-field форма)",
    en: "Create a new snippet (opens a 3-field form)",
  },
  "snippets.new.title":       { ru: "+ Новый snippet",         en: "+ New snippet" },
  "snippets.create.toast.ok": { ru: "/{key} создан · {n} snippets", en: "/{key} created · {n} snippets" },
  "snippets.create.toast.fail": { ru: "Не сохранилось: {err}", en: "Failed to save: {err}" },
  "snippets.desc":            {
    ru: "Шаблонные ответы, разворачиваются мгновенно (без AI-вызова, $0). Нажми «Expand →» — карточка появится на tile-мониторе (см. секцию Auto-tiles).",
    en: "Template answers, expand instantly (no AI call, $0). Click «Expand →» — the card appears on the tile monitor (see Auto-tiles section).",
  },
  "snippets.filter.placeholder": { ru: "Фильтр ({n} всего)…", en: "Filter ({n} total)…" },
  "snippets.collapsed.hint":  {
    ru: "Свёрнуто чтобы Settings не превращался в портянку. Жми «показать» сверху · или используй F4 (KB palette) во время сессии — там же доступны.",
    en: "Collapsed so Settings doesn't become a long scroll. Click «show» at the top · or use F4 (KB palette) during a session — they're available there too.",
  },
  "snippets.json.hint.before": {
    ru: "Редактирование снипетов через JSON: ",
    en: "Edit snippets via JSON: ",
  },
  "snippets.json.hint.middle": { ru: " → массив ", en: " → array " },
  "snippets.json.hint.after": {
    ru: ". В будущей версии — palette через F4 и UI редактор прямо здесь.",
    en: ". A future version will add a palette via F4 and an inline UI editor here.",
  },

  // ── Toast + modal generic strings (v0.0.51 — caught by agent review) ─
  "toast.close":              { ru: "Закрыть",                en: "Close" },
  "modal.confirm.default":    { ru: "Подтвердить",            en: "Confirm" },
  "common.save":              { ru: "Сохранить",              en: "Save" },

  // ── Snippets list + CRUD modal (v0.0.52) ───────────────────────────
  // Per-snippet row buttons
  "snip.expand.button":       { ru: "Развернуть →",          en: "Expand →" },
  "snip.expand.tip":          {
    ru: "Открыть тайл со снипетом /{key}",
    en: "Open a tile with snippet /{key}",
  },
  "snip.expand.toast.ok":     { ru: "/{key} развёрнут как тайл", en: "/{key} expanded as tile" },
  "snip.expand.toast.fail":   { ru: "Expand failed: {err}",   en: "Expand failed: {err}" },
  "snip.edit.button.tip":     {
    ru: "Редактировать /{key} (title + body)",
    en: "Edit /{key} (title + body)",
  },
  "snip.edit.modal.title":    { ru: "✎ Редактировать /{key}", en: "✎ Edit /{key}" },
  "snip.edit.toast.ok":       { ru: "/{key} обновлён",        en: "/{key} updated" },
  "snip.edit.toast.fail":     { ru: "Не сохранилось: {err}",  en: "Failed to save: {err}" },
  "snip.delete.button.tip":   { ru: "Удалить snippet /{key} (с подтверждением)", en: "Delete snippet /{key} (with confirmation)" },
  "snip.delete.confirm":      {
    ru: "Удалить snippet /{key}?\n\nТекст: «{title}»\n\nВосстановить можно только через Import конфига или дефолты (пустой массив snippets в config.json → авто-заполнится из defaults).",
    en: "Delete snippet /{key}?\n\nText: «{title}»\n\nRestoration is only possible via Import config or defaults (empty snippets array in config.json → auto-fills from defaults).",
  },
  "snip.delete.toast.ok":     { ru: "/{key} удалён · {n} snippets осталось", en: "/{key} deleted · {n} snippets remaining" },
  "snip.delete.toast.fail":   { ru: "Удаление не сохранилось: {err}", en: "Delete didn't save: {err}" },
  // Modal form labels
  "snip.modal.key.label":     {
    ru: "Key (короткий идентификатор, используется как /{key})",
    en: "Key (short identifier, used as /{key})",
  },
  "snip.modal.key.placeholder": { ru: "k8s-ops",              en: "k8s-ops" },
  "snip.modal.key.locked.hint": {
    ru: "Key неизменяем при редактировании (snippet идентифицируется по key). Чтобы переименовать — удали и создай новый.",
    en: "Key is immutable when editing (snippet is identified by key). To rename — delete and create a new one.",
  },
  "snip.modal.title.label":   {
    ru: "Title (отображается в Snippets списке + в заголовке тайла)",
    en: "Title (shown in the Snippets list + tile header)",
  },
  "snip.modal.title.placeholder": {
    ru: "Kubernetes troubleshoot — 5-step framework",
    en: "Kubernetes troubleshoot — 5-step framework",
  },
  "snip.modal.body.label":    {
    ru: "Body (markdown, рендерится в тайле — поддерживает заголовки, списки, code blocks)",
    en: "Body (markdown, rendered in the tile — supports headings, lists, code blocks)",
  },
  "snip.modal.body.placeholder": {
    ru: "1. Check pod status: `kubectl get pods`\n2. Logs: `kubectl logs <pod>`\n3. ...",
    en: "1. Check pod status: `kubectl get pods`\n2. Logs: `kubectl logs <pod>`\n3. ...",
  },
  // Validation errors
  "snip.error.key.required":  { ru: "Key обязателен",         en: "Key is required" },
  "snip.error.key.format":    {
    ru: "Key: только латиница, цифры, '-', '_'. Первый символ — буква/цифра.",
    en: "Key: only latin letters, digits, '-', '_'. First char must be letter/digit.",
  },
  "snip.error.title.required": { ru: "Title обязателен",      en: "Title is required" },
  "snip.error.body.required": { ru: "Body не может быть пустым", en: "Body can't be empty" },
  "snip.error.key.dup":       { ru: "Snippet с key /{key} уже существует. Выбери другой key.", en: "Snippet with key /{key} already exists. Pick a different key." },
} as const;

export type StringKey = keyof typeof strings;

/// Look up a translation by key. Falls back to RU if EN missing, then
/// to the key itself if neither is present. The `?? key` branch should
/// never fire in production — it's a development-time safety net.
export function t(key: StringKey, lang: Lang): string {
  const entry = strings[key];
  if (!entry) return key;
  return entry[lang] ?? entry.ru ?? key;
}
