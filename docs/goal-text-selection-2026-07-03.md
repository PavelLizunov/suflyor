# Goal — выделение текста мышью + связная память (ТЗ 2026-07-03)

Источник: ТЗ владельца от 2026-07-03 (ниже). Feasibility разобрана fable по
исходнику **Slint 1.16.1** из cargo-registry (то, что вкомпилируется в бинарь).
Направление выбрано владельцем 2026-07-03.

## ТЗ (дословно, критерии приёмки)

1. **Выделение текста мышью** в архиве / тайлах / сводках / стенограмме, с
   действиями **«Копировать»** и **«В память ⭐»**.
2. **Связная память** — связанные факты сохраняются ОДНОЙ записью, не дробятся.
   Пример: «Бекап-сервер z14-4443-backup — IP 10.255.28.116, подсеть
   10.255.28.96/27» = ОДИН факт, не три.

Приёмка:
- [ ] В архиве/тайлах/сводках/стенограмме можно выделить текст мышью → скопировать / в память.
- [ ] Память сохраняет связанные факты одной записью (не дробит).
- [ ] Ручное добавление из выделения = один цельный факт (с возможностью подправить).

## Что реально даёт Slint 1.16.1 (проверено fable по исходнику)

- `Text {}` — **НЕ** выделяется (display-only). `builtins.slint:95-121`.
- `TextInput { read-only: true }` — **выделяется полноценно**: drag, double-click=слово,
  triple-click=абзац, Shift+клик, Ctrl+C, Ctrl+A, I-beam-курсор авто. Стрелки в
  read-only отключены (мышь/Ctrl+A only). `items/text.rs:839-935, 1054-1060`.
- Свап `Text → read-only TextInput` **почти drop-in**: `TextInput::layout_info`
  считает высоту из завёрнутого текста, растёт под контент в VerticalLayout. `text.rs:784-825`.
- `selection-foreground/background-color` настраиваются. Весь font/color-стайлинг
  TextInput принимает (в отличие от std-`TextEdit`, где тема не пробрасывается).
- **Выделение переживает popup-активацию** (`text.rs:1214-1221`) → right-click
  ContextMenuArea с «Копировать выделенное»/«⭐ в память» — чисто.
- Байтовые offset'ы выделения: `TextInput.cursor-position-byte-offset` /
  `anchor-position-byte-offset` — out-свойства, читаются из .slint/Rust, но
  помечены «internal, only for tests». На пине `=1.16.1` стабильно → **ДЕ-РИСК СПАЙКОМ**.
  Фоллбэк если недоступны: `ti.copy()` → читаем системный буфер (clipboard-win в дереве).
- ScrollView **не** ворует drag (fluent Flickable `interactive:false`) — скролл цел.
- `TextEdit` (std) read-only — есть, + встроенное right-click меню Copy/SelectAll;
  но тема не стилизуется + не отдаёт offsets. Годится для тумблера «Исходник» (b).
- Кастомный pixel→char хит-тест = 🔴 нет публичного API (sealed RendererSealed) — НЕ делаем.

**Единственное честное ограничение:** выделение — **в пределах одного блока**
(каждый блок — остров). Сквозной drag через блоки в 1.16 невозможен без потери
рендера → закрывается тумблером «Исходник» (b).

## Выбранный объём (владелец, 2026-07-03)

- Выделение = **(a) поблочно + (b) тумблер «Исходник»**.
- Память = **склейка N⭐ в одну запись + опциональная AI-группировка авто-извлечения**.

## Статус (2026-07-03)

- **P0 спайк** — ✅ свёрнут в P1: байтовые offset'ы выделения читаются из read-only
  `TextInput` на пине `=1.16.1` (фоллбэк на буфер не понадобился).
- **P1 тайлы/сводки/архив** — ✅ commit `8096671`. Gate green, adversarial-ревью 0 crit/imp,
  live-smoke: блоки на верной высоте (нет схлопывания), выделение + RU-контекст-меню +
  посев редактора подтверждены на архивном тайле.
- **P2 транскрипт** — ✅ (этот коммит). `SelectableText` вынесен в общий `controls.slint`
  (акцентная подсветка выделения); транскрипт использует его; **seek → таймкод/имя** (выбор
  владельца), клик по тексту выделяет. Gate green, ревью 0 crit/imp (M1-фикс: clamped index).
  Live-smoke: общий компонент подтверждён на summary-тайле (выделение+меню); визуал строки
  транскрипта отложен на live-ретест владельца (окно транскрипта не открылось чисто из-под
  маскировки computer-use).
- **P3 тумблер «Исходник» · P4 склейка N⭐ · P5 AI-группировка** — ⏳ дальше.

## План (каждая фаза: ci.ps1 0/0 ×3 крейта + независимое adversarial-ревью + atomic-коммит; UI → live-smoke; НЕ релизить без «релизь»)

- **P0 — Спайк (де-риск).** Тестовое окно: read-only TextInput ×3 →
  (1) выделение мышью работает; (2) `cursor/anchor-position-byte-offset` читаются
  из .slint на нашем пине; (3) выделение живо в activated ContextMenuArea;
  (4) кириллический Ctrl+с → `copy()` (шим из `text_ask.slint:80-85`). Если (2) нет →
  ⭐-из-выделения через `copy()`+буфер. Спайк-код НЕ коммитим (throwaway).
- **P1 — (a) тайлы/сводки/архив.** `ui/tile.slint`: текстонесущие блоки
  (параграф :579, буллет :601, код :665, таблица :690; H1-H3 опц.) `Text →
  read-only TextInput`; каждый в `ContextMenuArea`: «Копировать выделенное» →
  `ti.copy()`; «⭐ Выделенное в память» → новый колбэк. Rust `tile_copy.rs`:
  слайс `block.text` по offset'ам (char-boundary-safe) ИЛИ буфер → посев в
  существующий `capture-text`-редактор → `insert_approved_note`. Покрывает
  тайлы+сводки+**архив** (`spawn_content_tile` тот же tile.slint).
- **P2 — (a) транскрипт.** `ui/transcript.slint` строки `Text → TextInput` +
  ContextMenuArea; ⭐-из-выделения сеет `capture-text`/`capturing-line-index`.
  ⚠️ click-to-seek (сейчас клик по строке = play-line, :301-305) — TextInput съест
  клики по тексту → seek на таймкоде/спикере ИЛИ маленькая ▶. Принять явно.
- **P3 — (b) тумблер «Исходник».** `TileWindow`: `select-mode:bool` + `raw-text`;
  в «⋯»-меню тумблер; в режиме — read-only `TextEdit` со всем сырым markdown
  (свободное сквозное выделение). `set_raw_text` рядом с ~10 `set_blocks`-колсайтами.
  Char-cap как у транскрипта (i16-класс на гигантском ответе).
- **P4 — Память: склейка N⭐ (S).** `tile_copy.rs on_save_marked` при N>1 →
  склеить отмеченные блоки (перенос/`;`) в ОДИН `insert_approved_note`; редактор
  правки показывать и при N>1 (сейчас `if marked-count==1`, tile.slint:709).
  Save-from-selection (P1) уже = одна запись. Диф ~15 строк + тест.
- **P5 — Память: AI-группировка авто-извлечения (M).** Opt-in AI-проход
  «сгруппируй в атомарные факты по сущности/теме» (JSON → `insert_candidate`).
  Учесть egress/локальную модель/недетерминизм. Дешёвое приближение без AI:
  кластеризация соседних строк по общему name-like термину (`memory/summary_ref.rs:43`
  `key_terms` уже есть) — если хватит, AI не нужен.

## Файлы-якоря

- `slint-experiment/ui/tile.slint` — `MarkdownBlock` struct :14; `for block[i]` :550;
  StarMark :26; copy-block `:642`; `changed blocks` :160.
- `slint-experiment/ui/transcript.slint` — `for line[i]` :284; click-to-seek :301.
- `slint-experiment/src/bin/overlay_host/tile_copy.rs` — `wire_block_capture`,
  `wire_code_copy`, `insert_approved_note` :316, `on_save_marked` :398.
- `slint-experiment/src/markdown.rs` — markdown→blocks парсер.
- `overlay-backend/src/memory.rs` — `insert_memory_item`/`insert_candidate`/`update_memory_item_text`.
- `overlay-backend/src/memory/candidates.rs:39` — эвристика «Извлечь» (Q&A, не построчно).
- Кириллический Ctrl+C шим: `ui/text_ask.slint:80-85`, `ui/palette.slint:87-95`.

## Инварианты (для review-агента)

- Slint `Text` не выделяется — только `TextInput`/`TextEdit`. Пин `=1.16.1`.
- НЕ ломать: драг-тайла за титлбар, скролл, ⭐-гаттер/копи-кнопки v0.26.0, i16-cap.
- i18n: каждая новая строка `@tr("EN")` + msgid/msgstr в `ru.po`; i18n_guard.
- Секьюрити: ⭐/копия только текст блока/выделения; ничего из config к тайлу не течёт.
- Без марафонов; без релиза без «релизь».
