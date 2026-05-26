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
