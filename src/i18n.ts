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
