# Ревью зависимостей — 2026-07-09

Полный аудит трёх крейтов (`overlay-backend`, `slint-experiment`, `suflyor-tts`)
на состояние 2026-07-09 (master @e3ca633 + незакоммиченный `tiny_http` из
hermes-сессии). Прогнано: `cargo deny check advisories bans sources` (свежая
RUSTSEC-база), `cargo deny check licenses` (полный разбор отказов), обратные
деревья дубликатов (`cargo tree -i --target x86_64-pc-windows-msvc`), сверка
всех прямых зависимостей с crates.io, grep-проверка фактического использования
каждой прямой зависимости, триаж открытых Dependabot-PR.

## Вердикт

**Безопасность: зелено.** Ни одного действующего RUSTSEC-advisory, ни одного
yanked-крейта, все источники — crates.io. Но: **3 мёртвые прямые зависимости**
в overlay-backend, **4 Dependabot-PR ждут триажа с 23.06**, и **licenses-гейт
можно наконец включить в CI** — весь список «неожиданных» лицензий разобран,
несовместимого с GPL-3.0 в дереве нет.

---

## 1. Advisories / bans / sources — OK ×3 крейта

- `advisories ok, bans ok, sources ok` на всех трёх (RUSTSEC-база от 2026-07-09).
- crossbeam-epoch 0.9.20 (фикс RUSTSEC-2026-0204 из релиза v0.31.0) на месте.
- **5 ignore-записей в deny.toml НЕ трогать.** Для slint-experiment все пять
  живые — крейты в дереве: generational-arena 0.2.9, paste 1.0.15,
  ttf-parser 0.25.1, quick-xml 0.39.4. Предупреждения `advisory-not-detected`
  сыплются только на прогонах overlay-backend / suflyor-tts (там этих крейтов
  нет), конфиг общий на три крейта → предупреждения косметические, CI не валят.
  Пересмотреть список при миграции на Slint 1.17.

## 2. Мёртвые прямые зависимости (overlay-backend) — удалить 3 строки

Grep по `src/**` + `tests/**` (точные паттерны `<crate>::`, `use <crate>`,
типы):

| Крейт | Использований | Факт |
|---|---|---|
| `bytes` | 0 | Тип `Bytes` нигде не именуется; `resp.bytes()` у reqwest прямой зависимости не требует. Останется транзитивной через reqwest/hyper. |
| `once_cell` | 0 | Вытеснен std (`OnceLock`/`LazyLock`) по ходу рефакторов. |
| `thiserror` | 0 | Ошибки целиком на anyhow; ни одного `#[derive(Error)]`. |

Правка — только манифест (дерево не меняется, всё остаётся транзитивным),
риска ноль, гейт подтвердит:

```diff
--- overlay-backend/Cargo.toml
-reqwest = { ... }
-bytes = "1"
+reqwest = { ... }
 ...
-parking_lot = "0.12"
-once_cell = "1"
+parking_lot = "0.12"
 ...
-log = "0.4"
-thiserror = "2"
-anyhow = "1"
+log = "0.4"
+anyhow = "1"
```

(`futures-util` — 1 вызов, `StreamExt` в ai.rs для SSE-стрима — легитимен.
Остальные прямые зависимости всех трёх крейтов используются; ноль у
`i-slint-backend-selector` ожидаем — это feature-носитель для `mcp`,
задокументировано в манифесте.)

## 3. Licenses — TODO из deny.toml готов к закрытию

`cargo deny check licenses` сейчас падает на всех трёх крейтах, но полный
список отказов конечен и весь разобран:

| Лицензия | Кто | Оценка для GPL-3.0-приложения |
|---|---|---|
| `GPL-3.0-or-later` | наши собственные 3 крейта | это мы сами |
| `GPL-3.0-only OR LicenseRef-Slint-*` | 13 крейтов Slint | потребляем ровно GPL-ветку — совместимо по определению |
| `CDLA-Permissive-2.0` | webpki-roots, webpki-root-certs | пермиссивная data-лицензия (CA-бандл Mozilla), ок |
| `BSL-1.0` | clipboard-win, error-code | Boost — GPL-совместима (FSF) |
| `(MIT OR Apache-2.0) AND NCSA` | libfuzzer-sys | NCSA пермиссивная; сам крейт только в build-tooling (rav1e←ravif←image←i-slint-compiler), в бинарник не попадает |

Ничего копилефт-несовместимого или экзотического в деревьях нет. Готовая
правка `deny.toml`:

```toml
[licenses]
allow = [
  # ... существующие ...
  "GPL-3.0-only",        # Slint (потребляем GPL-ветку тройной лицензии)
  "GPL-3.0-or-later",    # собственные крейты репо
  "CDLA-Permissive-2.0", # webpki-roots — CA-данные Mozilla
  "BSL-1.0",             # clipboard-win, error-code
  "NCSA",                # libfuzzer-sys (build-tooling slint-compiler)
]
```

и в `.github/workflows/security.yml` строку проверки дополнить:

```yaml
- run: cargo deny --manifest-path ${{ matrix.crate }}/Cargo.toml check --config deny.toml advisories bans sources licenses
```

После этого supply-chain-гейт закрывает и лицензии (сейчас — «warn-only
локально», т.е. фактически не проверяются никем).

## 4. Версии: сверка с crates.io (2026-07-09)

**Актуальны** (locked == latest): serde, serde_json, tokio 1.52.3,
futures-util, parking_lot, once_cell, transcribe-rs 0.3.11, wasapi 0.23.0,
windows 0.62.2, base64, dirs, thiserror, anyhow, hound, tiny_http 0.12.0,
tempfile, scopeguard, winresource, raw-window-handle, pulldown-cmark,
image 0.25.10, clipboard-win 5.4.1.

**Отстают** — и почти всё уже лежит открытыми Dependabot-PR (без триажа
с 23.06–01.07):

| Зависимость | Локально | Latest | PR | Рекомендация |
|---|---|---|---|---|
| log | 0.4.30 | 0.4.33 | #7 | **Мерджить** (патч, lockfile-only). После мерджа сделать то же в overlay-backend (`cargo update -p log`) — PR покрывает только slint-experiment. |
| rusqlite | 0.40.0 | 0.40.1 | — | **Взять**: `cargo update -p rusqlite` — патч тянет свежий bundled SQLite (libsqlite3-sys 0.38; фиксы SQLite = наша БД памяти/сессий). |
| sherpa-onnx | 1.13.3 | 1.13.4 (08.07) | — | Дёшево при следующей работе над TTS-сайдкаром. |
| rfd | 0.15.4 | 0.17.2 | #4 | Не авто-мерджить: два минора, API диалогов мог поехать. Брать при следующем заходе в Settings + живой смоук import/export. |
| sha2 | 0.10.9 | 0.11.0 | #2 | Мажор RustCrypto: digest-трейты поменялись, у нас 3 вызова в update.rs. Делать осознанно, не ботом. Не срочно. |
| reqwest | 0.12.28 | 0.13.4 | #1 | Мажор на критическом пути (ai/stt/update/local_ai). Планировать отдельной задачей с полным гейтом + live smoke. 0.12 пока жив, не горит. |
| global-hotkey | 0.6.4 | 0.8.0 | — | Два минора; хоткеи = ядро UX. Мигрировать отдельно с ретестом всех F-клавиш. |
| rodio | 0.20.1 | 0.22.2 | — | Бонус бампа: уводит cpal с windows 0.54 (минус дубль windows в дереве). Но Source-API менялся → трогать вместе со следующей правкой плеера + LISTEN-ретест. |
| slint (+build/selector/testing) | =1.16.1 | 1.17.1 (07.07) | — | Уже в роадмапе как отдельная миграция. 1.17.1 вышел 2 дня назад — не спешить. Помнить: пин `=1.16.1` на selector и фича `unstable-winit-030` двигаются строго в связке. |
| timestretch | 0.4.0 | 0.5.0 (**сегодня**) | — | **Не бампать вслепую** — см. §6. |

Итог по PR: #7 — мерджить; #4 — отложенно, с ретестом; #1, #2 — закрыть или
держать как маркеры и вести миграции руками по плану. (Dependabot-конфиг
корректный: weekly, группировка minor+patch, все 3 директории покрыты.)

## 5. Дубликаты версий / вес дерева

- Локфайлы: overlay-backend 287 пакетов (15 имён задублировано),
  slint-experiment 782 (65), suflyor-tts 106 (2). Но **Windows-граф реально
  компилируемого**: 167 и 436 пакетов — бо́льшая часть дублей (wayland-, objc2-,
  ndk-, jni-поддеревья) под Windows вообще не собирается. Паниковать не о чем.
- Реальные win-дубли: экосистемная рябь windows-*/hashbrown/getrandom
  (warn-уровень, не гоняться) и один осмысленный: **windows ×3**
  (0.54 ← cpal 0.15.3 ← rodio 0.20.1; 0.61.3 ← accesskit ← slint; 0.62.2 —
  наш). Наш рычаг — бамп rodio (см. таблицу), 0.61.3 уйдёт со Slint 1.17.
- suflyor-tts: webpki-roots 0.26+1.0 — шим-реэкспорт одной логической копии,
  и только в build-графе. Не проблема.
- rav1e (AV1-энкодер!) и libfuzzer-sys в дереве slint-experiment — это
  **build-tooling** i-slint-compiler (через image с default-features у них),
  в shipped-бинарник не попадает; resolver=2 не даёт фичам протечь в наш
  runtime-`image` (jpeg-only). Цена — только время сборки; апстримить нечего.

## 6. Supply-chain: кому мы доверяем

- **timestretch 0.4.0 — самый молодой и наименее обкатанный крейт в
  бинарнике.** Крейту 4,5 месяца (создан 26.02.2026), один владелец
  (robmorgan), ~2.7k суммарных скачиваний. Смягчения уже есть: фактический пин
  (0.4.0 — последний в 0.4.x, авто-скачок на 0.5 невозможен) + его исходники
  независимо ревьюились при интеграции v0.30. **0.5.0 опубликован сегодня без
  GitHub-релиза/тега/ноутов** → не бампать, пока не появится changelog; при
  следующей работе над плеером посмотреть diff 0.4→0.5 — v0.4.0 по релиз-ноутам
  как раз добавлял «streaming WSOLA overlay + crossfading», так что 0.5 может
  быть релевантен известному ⚠ boundary-seam из бэклога. Транзитивные деки
  чистые: arc-swap, rustfft, serde, serde_json.
- **tiny_http 0.12.0 (новый, hermes-мост) — выбор одобряю.** 52M скачиваний,
  advisories нет (смаглинг-эра закрыта ещё в ≤0.8), крошечный API, sync-модель
  идеальна для loopback-моста в отдельном потоке; axum/hyper были бы
  оверкиллом. Каветы: последний релиз 2022 (крейт «тихий», но стабильный);
  HTTP-парсер видит запросы ЛЮБОГО локального процесса ещё до 401 — для
  loopback-порога приемлемо. Просьба к hermes-сессии: добавить к строке в
  манифесте традиционный «why»-комментарий (единственная строка без него).
- **Build-time скачивания бинарей** (осознавать, что они есть): ort-sys
  2.0.0-rc.12 (onnxruntime) и sherpa-onnx-sys 1.13.3 качают нативные
  библиотеки в build.rs (отсюда ureq/native-tls/webpki в их деревьях). Пин
  версий -sys-крейтов это фиксирует; принято. ort — release candidate:
  следить, когда transcribe-rs переедет на финальный ort 2.0.
- Остальное — mainstream топ-100 crates.io (serde/tokio/reqwest/…) либо
  vendor-official: windows = Microsoft, slint = SixtyFPS GmbH,
  global-hotkey = tauri-apps, rfd = PolyMeilex, rusqlite = стандарт де-факто.

## 7. Мелкая гигиена (по одной строке)

- `suflyor-tts/Cargo.toml`: добавить `publish = false` (у двух других крейтов
  есть — страховка от случайной публикации GPL-сайдкара).
- bridge.rs (не зависимость, попутное): сравнение Bearer-токена —
  не constant-time (`==` на строках). На loopback, где любой локальный процесс
  и так может прочитать config.json того же юзера, практический риск ~0;
  если захочется belt-and-braces — сравнить SHA-256 обеих сторон имеющимся
  sha2, без новых зависимостей.

## Повторить аудит

```powershell
# из корня репо, на крейт:
cargo deny --manifest-path <crate>/Cargo.toml check --config deny.toml advisories bans sources licenses
cargo tree --manifest-path <crate>/Cargo.toml --target x86_64-pc-windows-msvc -d   # win-дубли
gh pr list --state open   # триаж Dependabot
```
