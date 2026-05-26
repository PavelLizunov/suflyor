# i18n plan — full RU + EN translations

User: «Также сделай полные переводы как на русский так и на английский»

Currently the UI is a mix of Russian + English (button labels in
English, descriptions in Russian, headings vary). Need both languages
selectable + complete translation of every visible string.

## Scope inventory

**Settings.tsx** — ~250 visible strings across 11 panels.
**Overlay.tsx** — ~50 strings (hotkey legend, indicator legend, toolt­ips).
**TileWindow.tsx** — ~20 strings (close-tip, pin-tip, sources).
**Replay.tsx** — ~40 strings (event kinds, headings, filter chips).

**Total: ~360 strings × 2 languages = 720 translation entries.**

## Architecture decision

Two reasonable approaches:

**A. Build-time strings** (typed t-tagged template)

```ts
// src/i18n.ts
type Lang = "ru" | "en";
const strings = {
  "settings.title":           { ru: "Настройки",      en: "Settings" },
  "settings.exit":            { ru: "Выйти",          en: "Quit" },
  "settings.back":            { ru: "← К overlay",    en: "← Back to overlay" },
  "settings.save":            { ru: "Сохранить",      en: "Save" },
  // ...
} as const;
export type StringKey = keyof typeof strings;
export function t(key: StringKey, lang: Lang): string {
  return strings[key]?.[lang] ?? strings[key]?.ru ?? key;
}
```

Pros: type-safe — TS will catch missing keys. Single file to scan.
Cons: One giant translation file is a maintenance pain.

**B. Per-component locale objects**

```ts
// In Settings.tsx
const ru = { title: "Настройки", exit: "Выйти", ... };
const en = { title: "Settings", exit: "Quit", ... };
const t = lang === "en" ? en : ru;
// JSX:
<h2>{t.title}</h2>
```

Pros: Localized to component, easy to grep.
Cons: Lots of duplicate boilerplate.

**Chosen: A** — central typed `strings` map. Better discoverability for
a 1-developer project; type safety prevents drift.

## Implementation order

1. **v0.0.41**: i18n infrastructure
   - Add `ui_language: "ru" | "en"` to Config (default "ru" — current
     primary language)
   - Create `src/i18n.ts` with `t(key, lang)`
   - Add language switcher in Settings → Interface panel
   - Translate **sidebar nav + footer + header** (visible always — high
     visibility win for small key count)
2. **v0.0.42**: Settings panel translations (Stealth + Coaching +
   Interface + Hotkeys)
3. **v0.0.43**: Settings panel translations (AI + Profile + Audio)
4. **v0.0.44**: Settings panel translations (Tiles + Knowledge +
   Advanced)
5. **v0.0.45**: Overlay + TileWindow + Replay strings

Per-release cycle: 6-gate verification with computer-use smoke test in
both RU + EN modes.

## Language detection / persistence

- Default: `ui_language: "ru"` (current primary)
- Stored in config.json — same file as bridge URL, models, etc.
- Loaded once on Settings/Overlay mount, no runtime switching of strings
  needed (re-render via `useEffect` on config change is enough)
- Tray tooltips + tray menu: separate concern — Tauri tray menu is built
  in Rust, would need to read config language on every menu rebuild.
  Tray remains Russian for now.

## Things NOT translated

- **Model names** (`claude-haiku-4-5`, etc.) — proper nouns
- **Hotkey names** (F3, F4, Ctrl+Alt+W) — universal
- **Code snippets in markdown tile bodies** — AI generates in
  response_language, which is independent
- **AI response language** — already a separate `response_language`
  config field
- **Knowledge base entries** — Russian-only by design (DevOps interview
  vocab)
- **Journal event names** (`transcript_line`, `ai_request`) — internal
- **Console log messages** — dev-facing, not user-facing

## Acceptance criteria

For each release in the i18n series:
1. Switch language to EN in Settings → Interface → Language
2. Save + reopen Settings → ALL visible strings English
3. Switch back to RU → ALL visible strings Russian
4. No fallback to key name visible (the `?? key` branch)
5. 6-gate methodology passes (smoke screenshot in both languages)
