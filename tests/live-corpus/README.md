# Live regression corpus

Структурированные тесты overlay-mvp против реальных interview-видео.

## Workflow

```
1. Вы выбираете YouTube-видео собеседования
        ↓
2. Шлёте URL → я генерирую cases/<id>/ground-truth.json
        ↓
3. Вы запускаете app, проигрываете видео (system audio → A50 Stream Out)
        ↓
4. Stop → копируете последний journal из %APPDATA%\overlay-mvp\sessions\
        ↓
5. cargo run --bin journal-eval -- <journal.jsonl> tests/live-corpus/cases/<id>/ground-truth.json
        ↓
6. Markdown-отчёт в stdout: что Whisper не распознал, какие вопросы detector пропустил
```

## Структура

```
tests/live-corpus/
├── README.md
└── cases/
    ├── example/                      ← reference schema
    │   └── ground-truth.json
    └── 001-<short-slug>/
        ├── ground-truth.json         ← что ожидаем от транскрипта/detector
        ├── source.md                 ← URL, описание, почему этот ролик
        └── runs/
            ├── 2026-05-24/
            │   ├── journal.jsonl     ← копия из %APPDATA%
            │   └── report.md         ← результат eval
            └── 2026-05-25/...
```

## Schema: ground-truth.json

См. `cases/example/ground-truth.json`. Поля:

| Поле | Тип | Описание |
|---|---|---|
| `case_id` | string | "001-k8s-basics" |
| `title` | string | "K8s основы — типовое интервью" |
| `source` | string? | URL источника (YouTube/файл) |
| `duration_sec` | u64? | Длина видео в секундах |
| `domain` | string[] | теги: ["kubernetes", "devops"] |
| `expected_terms` | object[] | Whisper-accuracy check |
| `expected_triggers` | object[] | Detector-recall check |
| `expected_quiet` | string[] | Detector-precision check (не должны триггерить) |
| `answer_quality_notes` | string? | Markdown-чеклист для ручной проверки качества ответов |

### `expected_terms[]`

```json
{
  "canonical": "kubernetes",
  "aliases": ["k8s", "кубер"]
}
```

Считается ✅ если `canonical` ИЛИ любой `alias` встретился (case-insensitive substring) в любой транскрибированной строке.

Если совпал alias — в отчёте будет ⚠️ "matched only as alias" — Whisper фонетизировал термин, можно добавить в `trigger_keywords` config или в Whisper prompt.

### `expected_triggers[]`

```json
{
  "text_match": "что такое pod",
  "must_trigger": true
}
```

Eval ищет detector_decision event где `text` содержит `text_match` (case-insensitive). Если такой есть И `triggered=true` — ✅. Если есть, но `triggered=false` — detector пропустил. Если такой строки в транскрипте вообще нет — это либо Whisper зажевал, либо VAD не активировался.

### `expected_quiet[]`

```json
["угу", "ну вот так", "конечно"]
```

False-positive guard. Если detector триггерил на строке содержащей одну из этих фраз — был зря потраченный AI запрос. В реальном собеседовании такие фразы регулярны и не должны разогревать Haiku.

## Запуск eval

```bash
cd src-tauri
cargo run --bin journal-eval -- \
  "%APPDATA%\overlay-mvp\sessions\2026-05-24_15-30-12_abc123.jsonl" \
  ..\tests\live-corpus\cases\001-k8s-basics\ground-truth.json \
  > ..\tests\live-corpus\cases\001-k8s-basics\runs\2026-05-24\report.md
```

В Windows PowerShell:

```powershell
$session = "$env:APPDATA\overlay-mvp\sessions\<имя файла>.jsonl"
$gt = "..\tests\live-corpus\cases\001-k8s-basics\ground-truth.json"
cargo run --bin journal-eval -- $session $gt | Out-File -Encoding utf8 report.md
```

## Что в отчёте

```markdown
# Eval · `001-k8s-basics` — K8s основы

## Whisper accuracy
5/8 terms found (62%)
- ✅ `kubernetes` (exact)
- ⚠️ `kubectl` matched only as alias `кубктл`
- ❌ `statefulset` MISSING

## Detector recall
7/9 expected triggers fired (78%)
- ✅ "что такое pod" → triggered
- ❌ "deployment отличается" → MISSED (transcript: "А как деплоймент...")

## Detector precision
1 false-positive triggers (wasted AI calls)
- ⚠ quiet utterance `угу` triggered (text: "Угу, понял.")

## Session aggregates
- Duration: 10.0 min
- Transcript: 87 lines
- Detector: 8 triggered / 24 skipped
- AI: 8 requests · 8 responses · 0 errors
- Tiles spawned: 8
- Cost: $0.0214
```

## Когда добавлять новый кейс

Любой раз когда нашли неприятный edge case:
- Конкретный термин который Whisper переиначивает → добавьте в `expected_terms`
- Конкретный тип вопроса который detector пропускает → добавьте в `expected_triggers`
- Регулярная "тихая" реплика которая случайно триггерит → в `expected_quiet`

Корпус — это **спецификация что должен уметь app**. Каждый зелёный run = регрессий не было.

## Подготовка YouTube транскрипта

YouTube → видео → кнопка "..." под видео → "Показать транскрипт" → копируйте весь текст с тайм-кодами → пришлите мне.

Альтернатива: `yt-dlp --write-auto-subs --skip-download --sub-format vtt <URL>` → шлёте .vtt файл.
