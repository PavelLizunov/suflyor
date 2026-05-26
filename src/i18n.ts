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
