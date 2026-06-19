# Регресс-чеклист — overlay-mvp (suflyor)

Дата: 2026-06-04 · покрывает состояние после **v0.9.2** (архив + курируемая память).

**Зачем:** методология «без марафонов» — статические проверки (clippy/test) проходили,
а пользователь ловил регрессы ЖИВЬЮ (layout, focus-расы, мульти-монитор, i18n). Этот
чеклист — что прогнать ПЕРЕД каждым релизом, чтобы регресс не доехал до пользователя.

Слои 1–4 = автоматом (gate их частично форсит). Слой 5 = руками, на живой сборке.
**Что трогает Slint UI / геометрию / прозрачность — требует ВСЕХ пяти.**

---

## Слой 1–3 — статический gate (каждый коммит; git-gate форсит fmt+clippy на commit, тесты на push)

```pwsh
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$be = "overlay-backend\Cargo.toml"; $se = "slint-experiment\Cargo.toml"
& $cargo fmt --manifest-path $be;  & $cargo fmt --manifest-path $se
& $cargo clippy --manifest-path $be --all-targets   # 0 warnings (deny unwrap/expect/panic)
& $cargo clippy --manifest-path $se --bin overlay-host
& $cargo test --manifest-path $be   # ~233 тестов
& $cargo test --manifest-path $se   # ~37 тестов (+ slint UI компилится => .slint валиден)
```
- [ ] clippy = 0 в ОБЕИХ крейтах (включая `--all-targets`).
- [ ] cargo test зелёный в ОБЕИХ.
- [ ] fmt применён (не `--check` — прогнать и закоммитить).

## Слой 4 — review-agent (перед коммитом, не-hotfix)
- [ ] Независимый агент с полным diff + инвариантами (секреты/паники/логика/i18n/borrow).
      Hotfix-short-circuit ТОЛЬКО если: impl ≤5 строк И одна поверхность И нет @tr-строки с ru.po.

## Слой 5a — boot-smoke (дёшево: запустить .exe, прочитать stderr ~5с, убить)
Запуск (release): `Start-Process target\release\overlay-host.exe -RedirectStandardError <log>`.
В логе должно быть, БЕЗ `panic`:
- [ ] `=== suflyor overlay-host vX.Y.Z start ===` (правильная версия).
- [ ] Все хоткеи: `F1 F3 F4 F6 F7 F8 Shift+F8 F9 Shift+F9 hotkey registered` (9 шт).
- [ ] `catalog: indexed N sessions (M skipped, 0 failed)` (SQLite-каталог жив).
- [ ] `bar pinned at (x, y)` — на ПЕРВИЧНОМ мониторе.
- [ ] `capture pre-stealth: stealth_ok=true taskbar_ok=true`.
- [ ] `local STT warmed` (если GigaAM) / стек-строка без секретов (`base_url=set`, не значение).

## Слой 5b — визуал (computer-use ВРЁТ про цвета прозрачного оверлея!)
Истина = `CopyFromScreen` по HWND-rect (EnumWindows+GetWindowRect, фильтр по pid+title
`overlay-mvp (Slint)`), сохранить PNG, прочитать.
- [ ] Бар не обрезан (правый кластер `✏ +тайл 📷 ⚙ 🆘 🔄 X` виден целиком, ширина 1160).
- [ ] Все глифы рендерятся, НЕ «тофу»-квадрат (🎤🔊🎯🔥🗄📷⚙🆘🔄 + 📝🔁➕ в памяти).
- [ ] Цвет/тема корректны (light/dark по `color_scheme`).

---

## Слой 5c — ручной интерактивный smoke (18-pt Win32; по поверхностям)

### Бар
- [ ] Драг за лого / status-pill двигает бар; не «залипает».
- [ ] Чипы кликабельны: 🎤🔊 toggle, 🎤/🔊 record (hold→STT→ask), ▶/⏸ таймер, 🎯 stealth,
      🔥 aggressive, 🗄 архив, ✏ написать, +тайл, 📷 capture, ⚙ настройки, 🆘 help, 🔄 restart, X quit.
- [ ] Live-строка транскрипта обновляется; «AI streaming» пульсирует при ask.

### Тайлы (F9 / авто / F6)
- [ ] F9 ask стримит ответ; markdown ок (таблицы, code no-wrap+scroll).
- [ ] 📋 copy, ✏ follow-up, 🔄/🧠 regen/escalate, close/pin/maximize, драг.
- [ ] Размещение на правильном мониторе (primary, либо landscape-вторичный ≥ширины).
- [ ] AI/STT ошибка → GENERIC текст (НЕ error chain, НЕ base_url/IP).

### Хоткеи
- [ ] F3 reask · F4 палитра (toggle+Esc+Enter спавнит) · F6 manual · **F7 архив (toggle)** ·
      F8 vision · Shift+F8 translate · F9 ask · Shift+F9 cloud-escalate · F1 help.

### Настройки (14 вкладок) — ⚙ чип загорается, X закрывает
- [ ] Каждая вкладка грузится, Save персистится в config.json, рестарт подхватывает.
- [ ] Смена схемы пропагируется на ВСЕ окна; смена языка ru/en переводит UI.

### STT / аудио / сессии
- [ ] Mic + sys capture, транскрипт в баре, GigaAM локально / Whisper.
- [ ] Автостарт сессии; stop → debrief-тайл; crash-recovery offer при незакрытой сессии.

---

## 🆕 Регресс-фокус v0.9.2 — архив + память

### 🗄 Архив (F7 / 🗄 чип)
- [ ] Открывается; список сессий новые-сверху (✓/⚠/● + 💬/🤖/модель/cost).
- [ ] Поиск-по-вводу фильтрует (RU и EN; prefix-матч).
- [ ] Клик по строке → read-only тайл с транскриптом (🎤/🗣) + Q&A (❓/💡).
- [ ] Esc / X / повторный F7 закрывают; при 🎯 stealth — НЕ виден в захвате экрана.

### 💭 Память (⚙ → 💭 Память)
- [ ] «Извлечь из последних сессий» наполняет кандидатов (📝 ответы / 🔁 темы).
- [ ] Принять → пункт уходит в «Одобренная память», из кандидатов исчезает.
- [ ] Отклонить → исчезает, пункт НЕ создаётся.
- [ ] Удалить (одобренный) → исчезает.
- [ ] Повторное «Извлечь» НЕ плодит дубли; отклонённые/одобренные НЕ возвращаются.

### 🧠 Память влияет на ответы (3b.4 — ГЛАВНОЕ)
- [ ] С ≥1 одобренным пунктом: F9/авто-ask → в журнале `system_prompt` есть блок
      «Сохранённая память пользователя».
- [ ] С НУЛЁМ одобренных: ask байт-в-байт как раньше (блока нет, лишней стоимости нет).
- [ ] Сломанный/занятый `catalog.sqlite` → ask ВСЁ РАВНО работает (graceful, без падения/виса).
      Журнал: `%APPDATA%\overlay-mvp\sessions\*.jsonl` (читать через IO.File Open+ReadWrite,
      PS Get-Content портит кириллицу cp1251).

---

## Релиз — финальные проверки перед `gh release create`
- [ ] Версия поднята в ОБОИХ: `slint-experiment/Cargo.toml` + `scripts/slint-installer.nsi`.
- [ ] Инсталлятор собран: `scripts/build-slint-release.ps1 -Installer` →
      `target/release/bundle/suflyor-slint-setup.exe`.
- [ ] Бинарь репортит НОВУЮ версию в boot-логе (`vX.Y.Z start`).
- [ ] **Ассет назван ТОЧНО `suflyor-slint-setup.exe`** — иначе встроенный авто-апдейтер
      (`update.rs::INSTALLER_ASSET`, exact-match, без `*.exe`-fallback) НЕ найдёт обновление.
- [ ] `gh release view vX.Y.Z`: `draft:false`, `prerelease:false`, ассет `state:uploaded`;
      `gh release list` показывает его как **Latest** (апдейтер берёт latest + SHA-256 от GitHub).

## Откат, если что-то сломалось
1. НЕ чинить «волнами» поверх сломанного. Откатиться к последнему known-good (`git revert` / тег).
2. Потом фикс с полным слой-кейком (1–5).
