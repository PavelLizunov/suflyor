# План обновления зависимостей (чартер рабочей сессии) — 2026-07-09

Исходник: [dependency-review-2026-07-09.md](dependency-review-2026-07-09.md)
(полный аудит: advisories БЫЛИ зелёные на момент ревью, 3 мёртвые зависимости,
4 Dependabot-PR без триажа, licenses-гейт готов к включению). Этот док —
исполняемый план: каждая фаза самодостаточна, разведка по changelog'ам УЖЕ
сделана (2026-07-09), факты проверены по манифестам crates.io и коду репо, не
по памяти.

> **⚠ Обновление 09.07 вечер (Hermes-сессия, `cargo audit` ×3):** появился
> СВЕЖИЙ **RUSTSEC-2026-0185** — `quinn-proto 0.11.14` (remote memory-exhaustion,
> HTTP/3 reassembly). Он **НЕ в `deny.toml ignore`** → следующий CI `security`
> упадёт (как недавно crossbeam-epoch на v0.31.0). Реальный риск ~нулевой
> (`reqwest` без фичи `http3` → quinn не в активном дереве, фантом в lockfile),
> но advisory реальный. **Фикс сделан В ЭТОЙ Hermes-сборке** (см. «Фаза 0-bis»),
> чтобы CI остался зелёным. Плюс `quick-xml` (RUSTSEC-2026-0194/0195) — уже в
> `deny.toml ignore` (Linux-only via wayland/atspi, парсим только свой bundled-XML
> → недостижимо), `cargo update` его не двигает (`wayland-scanner ^0.39`) → уйдёт
> при апгрейде Slint (Фаза 1). Останется зелёным.

## Фаза 0-bis — исполнено в Hermes-сборке (2026-07-09, lockfile-only)

Минимальный безопасный скоуп ЗАОДНО с Hermes-фичей (не трогает Cargo.toml/крупные
фазы ниже — те остаются координированными сессиями):
- **`cargo update -p quinn-proto`** → 0.11.14 → **0.11.16** (backend + slint;
  закрывает RUSTSEC-2026-0185; каскад quinn/quinn-udp/rand — транзитив).
- **`cargo update -p log`** → 0.4.30 → 0.4.33 (все 3 крейта) — Фаза 0 п.1.
- **`cargo update -p rusqlite`** → 0.40.0 → 0.40.1 (backend; свежий bundled SQLite) — Фаза 0 п.2.
- Верификация: `cargo deny check advisories` ×3 зелёный + полный гейт 0/0 +
  живой ретест Hermes-сборки (заодно ловит runtime сетевого/БД стека).
- ⚠ НЕ сделано здесь (остаётся Фазе 0 полной): удаление мёртвых депов из
  Cargo.toml, licenses-гейт, Dependabot-триаж, PR-merge — это правки манифестов/CI,
  оставлены координированной deps-сессии.

**Правила выполнения (методология репо):**
- Каждая фаза = отдельный коммит(ы) + полный гейт (`ci.ps1` 0/0 ×3 крейта) +
  live smoke своей поверхности + self-review диффа. Никаких «всё одним махом».
- Релизы копятся; `gh release` только по явному «релизь» владельца.
- Фазы 0–3 — мелкие, можно вклинивать между задачами. Фазы 4–5 — отдельные
  сессии. Порядок 1→2→3 не жёсткий, но Slint — приоритет владельца.
- ⚠ Координация: overlay-backend/Cargo.toml прямо сейчас правит параллельная
  hermes-сессия (tiny_http). Фазу 0 выполнять ПОСЛЕ её коммита — не раньше.

---

## Фаза 0 — quick wins (~30 мин, без изменения кода)

1. Смерджить [PR #7](https://github.com/PavelLizunov/suflyor/pull/7) (log
   0.4.30→0.4.33, slint-experiment, lockfile-only) — дождаться зелёного CI,
   merge. Затем то же для двух других крейтов локально:
   `cargo update -p log` в overlay-backend и suflyor-tts.
2. `cargo update -p rusqlite` (0.40.0→0.40.1, overlay-backend) — патч тянет
   свежий bundled SQLite (наша БД памяти/сессий/FTS5).
3. Удалить 3 мёртвые прямые зависимости из overlay-backend/Cargo.toml:
   `bytes`, `once_cell`, `thiserror` (ноль использований, проверено grep'ом
   по src+tests; остаются транзитивными — дерево не меняется).
4. `publish = false` в suflyor-tts/Cargo.toml.
5. Включить licenses-гейт: в deny.toml `[licenses] allow` добавить
   `"GPL-3.0-only", "GPL-3.0-or-later", "CDLA-Permissive-2.0", "BSL-1.0",
   "NCSA"` (обоснование каждой — в ревью-доке §3), в security.yml к
   `check ... advisories bans sources` дописать `licenses`. Локально прогнать
   `cargo deny check licenses` ×3 крейта — должно стать зелёным.
6. Триаж Dependabot: [#1](https://github.com/PavelLizunov/suflyor/pull/1)
   (reqwest) и [#2](https://github.com/PavelLizunov/suflyor/pull/2) (sha2)
   закрыть с комментарием «manual migration planned —
   docs/goal-deps-updates-2026-07-09.md»; [#4](https://github.com/PavelLizunov/suflyor/pull/4)
   (rfd) оставить открытым до фазы 3.

Гейт: fmt+clippy+test ×3 крейта; smoke не нужен (кода нет). Коммит вида
`chore(deps): quick wins from dependency review 2026-07-09`.

## Фаза 1 — Slint 1.16.1 → 1.17.1 (приоритет; ~половина сессии с ретестом)

**Разведанные факты (1.17.0 от 24.06 + 1.17.1 от 07.07):**
- `unstable-winit-030` СУЩЕСТВУЕТ в 1.17.1 (winit остался 0.30) — наш
  kbd_shortcuts-фильтр (`on_winit_window_event`, Ctrl+C/V/X/A на любой
  раскладке) по имени фичи переживает бамп. Компиляционный дрейф unstable-API
  всё же возможен — проверит сборка.
- `raw-window-handle-06` — на месте (HWND-poking жив).
- **`mcp` теперь фича самого крейта `slint`** (и на selector'е тоже осталась).
  → Убрать из манифеста ВСЮ строку `i-slint-backend-selector = { version =
  "=1.16.1", features = ["mcp"] }` (пин `=` был footgun'ом) и добавить `"mcp"`
  в фичи `slint`. Проверить форвардинг: `cargo tree -e features -i
  i-slint-backend-selector | grep mcp`.
- `i-slint-backend-testing = "1.17"` существует.
- Breaking 1.17.0 нас не трогают: wgpu-фичи (не используем), fontique-ренейм
  (не используем), deprecated `init()` — в нашем коде вызовов НЕТ (grep 09.07).
- Полезные фиксы, бьющие прямо в наши поверхности: ListView drag при разной
  высоте строк (стенограмма!), TextEdit-скролл при движении курсора, elision
  с Windows-переносами, панtérы GridLayout/Timer.restart.
- **Поведенческое изменение: каретка теперь ВИДНА в read-only TextInput** —
  а весь наш selectable-текст (тайлы/сводки/архив/стенограмма) это read-only
  TextInput. После бампа посмотреть глазами: если каретка мешает — искать
  свойство/скрывать, это пункт визуального чек-листа, не блокер сборки.

**Шаги:**
1. slint-experiment/Cargo.toml: `slint = "1.17"` (фичи: raw-window-handle-06,
   unstable-winit-030, +mcp), `slint-build = "1.17"`,
   `i-slint-backend-testing = "1.17"` (dev), строку selector — удалить.
2. `cargo build --bin overlay-host` → чинить компиляцию (ожидаемо ~0 правок;
   если unstable-winit API дрейфанул — правки локализованы в
   src/bin/overlay_host/kbd_shortcuts.rs).
3. Гейт полный ×3 + `cargo test --test i18n_guard` (gettext/@tr дрейф).
4. MCP-smoke: release-сборка, `SLINT_MCP_PORT=9123` → initialize →
   list_windows → take_screenshot (протокол в CLAUDE.md). Заодно проверить,
   не начал ли 1.17 наконец отдавать accessibility-дерево (в 1.16 было 0
   детей у всех окон — если починили, это разблокирует автокликер).
5. Live smoke по протоколу UI-диффа: бар + тайл + Настройки + Архив + Память +
   стенограмма (ListView с разными высотами!) + плеер + «По голосам», обе
   локали, скрины CopyFromScreen. Отдельно: каретка в read-only, Ctrl+C/V/X/A
   на кириллической раскладке, F8-vision, стелс.
6. После бампа: перепроверить 5 ignore-записей deny.toml (`cargo deny check
   advisories` покажет `advisory-not-detected` для выбывших — например
   ttf-parser/quick-xml могли смениться версией в 1.17-дереве; тогда чистить
   С УМОМ: запись жива, пока жива хоть в одном крейте).
7. retest-HTML пункты для тестера — по изменённым поверхностям (короткий,
   не полная регрессия).

## Фаза 2 — global-hotkey 0.6.4 → 0.8.0 (мелкая, ~30 мин + smoke)

**Факты:** breaking-изменений API register/unregister/HotKey/receiver в
changelog НЕТ (0.6.0 уже убирал Sync/Send — мы после этого). 0.7 — только
Linux/X11. 0.8 — win-фикс: sleep 50ms в release-detection-loop (меньше CPU
при удержании клавиш — нам в плюс), MSRV 1.77 (у нас 1.96). 
**Шаги:** bump → гейт → live smoke ВСЕХ хоткеев по таблице CLAUDE.md
(F1/F3/F4/F6/F7/F9+Shift, F8+Shift/Ctrl, Shift+Alt+1/2/3): лог
«hotkey registered» ×N при старте + живой тык каждого.

## Фаза 3 — rfd 0.15.4 → 0.17.2 (мелкая)

**Факты:** изменения 0.16/0.17 не про наш путь (set_parent ?Sized; удалены
tokio/async-std фичи — не используем; wayland — не наша платформа; MSRV 1.88 ok).
У нас rfd только в Settings Export/Import (sync FileDialog, IFileDialog).
**Шаги:** взять [PR #4](https://github.com/PavelLizunov/suflyor/pull/4) (или
bump локально и PR закрыть) → гейт → smoke: оба диалога открываются, файл
пишется/читается, отмена не ломает.

## Фаза 4 — reqwest 0.12.28 → 0.13.4 (критический путь; отдельная сессия)

**Факты:**
- Фича `rustls-tls` ПЕРЕИМЕНОВАНА в `rustls`. Наш манифест:
  `features = ["stream","json","multipart","rustls-tls"]` →
  `["stream","json","multipart","rustls"]` (все четыре существуют в 0.13.4 —
  проверено по манифесту crates.io).
- Дефолтный крипто-провайдер rustls теперь **aws-lc** (не ring).
  aws-lc-sys на x86_64-windows-msvc собирается из C/asm → может потребовать
  CMake+NASM локально и в CI. План Б (lean, без новых тулчейнов): фича
  `rustls-no-provider` + прямая зависимость `rustls` с фичей `ring` +
  `rustls::crypto::ring::default_provider().install_default()` один раз при
  старте. Сначала попробовать дефолт: если `cargo build` пройдёт без NASM —
  оставить aws-lc.
- Корневые сертификаты: по умолчанию `rustls-platform-verifier` (валидация
  через Windows-store) — поведенчески норм для api.groq.com/github.com;
  бонус — webpki-roots может уйти из дерева.
- `query`/`form` фичи выключены по дефолту — нам НЕ нужны (grep 09.07: ни
  одного `.query(`/`.form(` в коде).
**Шаги:** bump+фичи → выбор провайдера (см. выше) → гейт → ЖИВОЙ smoke всех
сетевых путей: облачный ask + SSE-стриминг ответа, vision (скрин→ответ),
local_ai health-poll + bearer, «Проверить связь» (Hermes/облако), update.rs
(проверка релиза GitHub + скачивание с SHA-256-верификацией). Закрыть PR #1
(в фазе 0 уже закрыт — тут просто не переоткрывать). retest-HTML не нужен,
если UI не менялся; лог-подтверждения путей — в коммит-сообщение.

## Фаза 5 — rodio 0.20.1 → 0.22.2 (+ оценка timestretch 0.5) — вместе со следующей работой над плеером

**Факты rodio (двойная волна переименований):**
- 0.21: `OutputStreamBuilder` обязателен; `Sink::try_new`→`connect_new`;
  все Source теперь f32; `current_frame_len()`→`current_span_len()`;
  `SamplesBuffer::new(nz!(1), nz!(16000), ..)` (NonZero-типы); дефолтный
  декодер — Symphonia (нам всё равно: кормим сырой PCM).
- 0.22: `Sink`→`Player`, `OutputStream`→`MixerDeviceSink`, `SpatialSink`→
  `SpatialPlayer`; cpal 0.18.
- Выгода дерева: уходит windows 0.54 (cpal 0.15.3 — единственный его
  потребитель).
- Затронутый код: player-glue + наш `StretchSource` (impl Source поверх
  timestretch WSOLA) — переписывается под новый трейт (f32/span/NonZero).
**Факты timestretch 0.4.0→0.5.0 (опубликован 09.07 БЕЗ GitHub-релиза):**
- Крейт пивотнулся в «EDM/DJ-first» (деки, CDJ-фейдер, warm-start seek/cue/
  loop API). Но в 0.5.0 вошли «Stage 1+6: modulation stability and realtime
  pitch quality» и merge PR «streaming-modulation-and-pitch-quality» —
  потенциально лечит наш известный ⚠ WSOLA boundary-seam (~1/сек).
- Молодой single-owner крейт: перед бампом РЕВЬЮ diff 0.4.0→0.5.0 (API
  `stretch::Wsola` мог поехать; интеграция v0.30 ревьюила именно 0.4.0).
- Warm-start seek API может заменить наш seek-by-reslicing — оценить.
**Шаги:** rodio bump + StretchSource-рефактор → (опционально в тот же заход)
timestretch 0.5 после ревью diff'а → гейт → LISTEN-ретест (1×/1.25×/2×/3×:
питч, швы, старт/пауза/сик, громкость-ползунок) → retest-HTML для владельца.
Проверить дерево: `cargo tree --target x86_64-pc-windows-msvc -d | grep windows` —
windows 0.54 должен исчезнуть.

## Фаза 6 — sha2 0.10 → 0.11 (микро, когда угодно)

**Факты:** digest 0.11; типы-алиасы стали newtype; фичи asm/std удалены (не
используем); edition 2024, MSRV 1.85 (ok). У нас 3 вызова в update.rs
(SHA-256 инсталлятора перед запуском).
**Шаги:** bump → поправить, если поменялась сигнатура `Sha256::new/update/
finalize` (скорее всего нет — Digest-трейт) → гейт → smoke: «Проверить
обновления» в Настройках (реальная верификация digest'а с GitHub API).

## Watch-list (не фазы — просто следить)

- **ort 2.0 final** (сейчас rc.12 через transcribe-rs 0.3.11) — при выходе
  transcribe-rs на финальном ort проверить DirectML-путь GigaAM.
- **sherpa-onnx 1.13.4+** — бамп сайдкара при следующем TTS-заходе.
- **slint 1.17.x** патчи — Dependabot принесёт (после фазы 1 пин снят).
- **timestretch** — появление GitHub-релиза/changelog для 0.5.x.
- **reqwest 0.12.x** security-патчи, пока фаза 4 не сделана (Dependabot
  показывает только 0.13-мажор; 0.12-патчи придут через `cargo update`).

## Definition of done

Фазы 0–3 закрыты → в дереве не остаётся ни одного «дешёвого» отставания;
фазы 4–6 — по расписанию владельца. После каждой фазы обновить этот док
(строку фазы → DONE + коммит-хеш). Релиз — только по «релизь».
