# ТЗ тестера 2026-07-06 — память-ретрив (M2) + скрин-эскалация + drag плашки ввода

Charter: 3 пункта от тестера. Оценка → план → независимая перепроверка → реализация
(порядок реализации: B → C → A, от малого к большому). Ветка: master (после v0.30.0).
Каждый пункт: gate 0/0 + independent review + atomic commit; приёмка владельцем по
retest-HTML. НЕ релизить без «релизь».

## A. Баг 1 — память есть, но суфлёр отвечает «нет информации»

**Реальный корень (разведка 2026-07-06, глубже, чем догадка тестера):** путь ответа
вообще не смотрит на вопрос. `context_for_meeting(base)`
(`overlay-backend/src/memory/context_builder.rs:82`) слепо кладёт в промпт **8
последних одобренных** фактов (капы `MAX_ITEMS=8`, `MAX_BLOCK_CHARS=1200`); факт №9+
молча выпадает → «нет информации». «Точный матч» есть только в пути саммери
(`summary_ref.rs:118`, стем-префикс), к ask-пути он не относится. FTS по памяти,
fuzzy-депов и эмбеддингов в коде НЕТ — только в ADR.

**Фикс (v2 после перепроверки): pure-Rust скорер, БЕЗ FTS5.** Перепроверка
показала: FTS5-вариант был оверинжинирингом с двумя реальными багами (см. вердикт
ниже: memory_items редактируется → INSERT-only-триггерный индекс тихо протухает;
`влада*` не матчит «владислав» — склонения ломают односторонний prefix). Фактов —
десятки, и ВСЕ уже загружены в чинимой функции (`list_memory_items(.., -1)`) —
нужно только переранжирование. FTS5/эмбеддинги вернём на пороге ~150–200 фактов
(триггер из ADR §6).

1. **`score_items(query, items)`** в `context_builder.rs` (чистая fn): токены
   вопроса × токены `text`+`entity` факта, матч = **симметричный префикс** через
   существующий `normalize::common_prefix_len` (общий корень ≥ min(len,4) — паттерн
   `words_match`); скор = число смэтченных токенов вопроса (tie-break: recency).
   Закрывает ОБА критерия приёмки напрямую: «влад»↔«Владислав» ✓,
   «влада»↔«Владислав» ✓ (симметрия), «Писчанкин»↔«Писчаскин» ✓ (общий префикс
   «писча»=5≥4 — даже опечатка матчится без Левенштейна). Стоп-токены: <4 симв.
   (естественно отсекает «кто», «такой», предлоги).
2. **`context_for_meeting(base, query: Option<&str>)`**: query есть и скоры >0 →
   top-8 по скору; query None/пуст ИЛИ 0 матчей → **recency-фоллбэк как сейчас**
   («память ВСЕГДА подмешивается» — жёсткий пол). Капы 8/1200 не трогаем.
   **Девять** колл-сайтов (перепроверка): с вопросом — tile_ask.rs:400
   (typed_question для ✏; None для F9), runtime.rs:1437 (F3 re-ask, `last_q`),
   runtime.rs:1727 (F6, `line.text`), overlay_host.rs:1828 (`req.spec.question`),
   tile_followup.rs:479,621, tile_ptt.rs:325 (текст PTT-вопроса);
   None — vision_capture.rs:494 (Describe без вопроса), slint_session.rs:703.
3. **Приоритет памяти над «не знаю»**: в заголовок блока памяти
   (`format_memory_block`) добавить строку-инструкцию: если вопрос касается этих
   фактов — отвечать ИЗ них, а не «нет информации».
4. Путь саммери (`term_in_tokens`) НЕ трогаем — другой контур, приёмка про ask.
   `normalize::tokenize`/`common_prefix_len` приватные → открыть pub(crate).

Тесты (чистые, без DB — паттерн `context_builder.rs` mod tests): «влад»→
«Владислав…»; «влада»→«Владислав…» (склонение); «тимур писчанкин»→факт (опечатка
через префикс «писча»); 0 матчей → newest-8; >8 фактов, релевантный старый — в
блоке; пустой/None запрос → как сейчас; короткие токены не матчат.

Объём: **S** (context_builder + normalize pub + 9 колл-сайтов + тесты; ни
миграции, ни FTS, ни санитайзера).

## B. Фича 2 — «перегенерировать облаком» для скрин-тайла

**Всё уже есть** (разведка): `can-escalate`/`escalate-clicked`/строка в «⋯»-меню — в
`tile.slint:149,220,435-444`; vision-тайлы просто не ставят флаг
(`vision_capture.rs:411-413`). Скриншот **уже удерживается** в
`bridge.conversations[convo_id].messages` (base64 data-URL, кладётся `PttStreamSink`
на Done) — `fire_regenerate(convo_id, .., AskRoute::Cloud)` переотправляет ТОТ ЖЕ
скрин в облако (prep_model, vision-capable) без нового хранилища.

Фикс: в `launch_vision_for_bgra` после блока regenerate (~строка 410) — зеркально:
гейт (vision-эндпоинт локальный + `ai_bearer` непуст + режим ≠ Ocr) →
`set_can_escalate(true)` + `on_escalate_clicked` → бейдж «cloud (escalated)»
(копия tile_followup.rs:353-356) + `fire_regenerate(.., AskRoute::Cloud)`.
Эскалация one-shot (у vision-тайла нет LiveRoute) — соответствует ТЗ.
`.slint` не меняется. Объём: **S (~15 строк)**.

## C. Фича 3 — перемещаемая плашка ввода + запоминание позиции

Drag: проверенный паттерн репо (визард/help/архив/бар) — колбэки
`drag-start-requested`/`drag-moved` + TouchArea за строкой заголовка (копия
`help.slint:161-194`) → `win32::drag_begin/drag_update` (обёртка из
`aux_windows.rs:221-238`).

Персист (первый в приложении — геометрию не хранит ни одно окно):
1. Конфиг: `text_ask_pos: Option<(i32, i32)>` c `#[serde(default)]`; сохранение по
   конвенции `cfg.write()` → `config::save` — в `drag_end`-момент не ловим, пишем
   при submit/cancel (`get_window_rect(hwnd)` перед hide, aux_windows.rs:151,161).
2. **Ловушка (разведка + перепроверка):** центрирование безусловное и в ДВУХ
   местах — `do_reveal` (`window_lifecycle.rs:180-188`) И `fallback_reveal`
   (:200-222); синхронный move после вызова проигрывает гонку. Фикс: новая
   `present_window_stealth_aware_at(win, pos: Option<(i32,i32)>, decorate)`
   (Some → move туда в ОБОИХ reveal-путях), старая fn = однострочный None-враппер
   → 7 остальных колл-сайтов не трогаем.
3. Валидация: сохранённая позиция вне видимых мониторов (отключили монитор;
   у владельца портрет на отрицательном x) → фоллбэк на центр. Чистая fn
   `clamp_to_visible(pos, monitors)` + unit-тест.

Объём: **S–M**.

## Порядок реализации
B (S, 15 строк) → C (S–M) → A (M, основное мясо). Каждый — отдельный коммит через
гейт + независимое ревью. Приёмка: единый `docs/retest-tz-2026-07-06.html`
(A: вопросы про сущности из памяти с опечаткой/сокращением; B: скрин → «⋯» →
Smart cloud → ответ обновился тем же скрином; C: перетащить, закрыть, открыть —
позиция та же; отключение монитора → центр).

## Перепроверка плана
Независимый re-check агент (2026-07-09), все вердикты заземлены в file:line +
живой FTS5-тест (sqlite 3.53, unicode61 remove_diacritics 2):

**B — CORRECT целиком.** tile.slint:149/220/435-444 ✓; vision-тайлы не ставят флаг
(vision_capture.rs:412-414) ✓; скрин живёт в conversations (sink с messages+data-URL
vision_capture.rs:554-560 → store on Done tile_controller.rs:258-278) ✓; 1-turn
regenerate шлёт stored messages как есть (tile_followup.rs:617-625), а reframe при
≥2 user-turns возвращает историю НЕТРОНУТОЙ, если есть Parts (:131-137) — картинка
сохраняется в обоих случаях ✓; Cloud→prep_model=claude-sonnet-4-6 (config.rs:535,
tile_routes.rs:39) vision-capable, JPEG уже даунскейлится (capture.rs:72-91) ✓.
Нюансы: `ep` в точке вставки ещё Option (unwrap на :465) — гейт через
`ep.as_ref().is_some_and(|e| e.is_local)`; OCR-тайл ИМЕЕТ «⋯» (can_regenerate:390,
кнопка безусловна tile.slint:379) — гейт mode≠Ocr обязателен, он же кроет
OCR→VLM-фоллбэк; follow-up ПОСЛЕ эскалации возвращается на Vision-роут (:384,
one-shot) — отметить в retest-HTML.

**C — CORRECT, 3 поправки.** Паттерн drag копируется (help.slint:161-194,
aux_windows.rs:221-238, win32.rs:688/708); TouchArea не фокусируемая — конфликтов
с forward-focus:input/Esc нет; TouchArea класть за title-зону, НЕ под CloseButton.
(1) present_window_stealth_aware центрирует в ДВУХ местах: do_reveal
(window_lifecycle.rs:180-188) И fallback_reveal (:200-222) — параметр позиции
покрыть в обоих. (2) Колл-сайтов ВОСЕМЬ (aux_windows 164/239/337/1125/2488,
recovery 313, wizard 469, settings_controller 1131) — вместо правки 8 сайтов:
новая `..._at(win, Option<(i32,i32)>, decorate)` + старая fn = однострочный
None-враппер (7 сайтов не трогаем). (3) Reuse-путь (aux_windows.rs:102-112) НЕ
re-present'ит, и он почти мёртв: submit/cancel обнуляют slot (:148-151,157-161) →
каждое открытие = новое окно = present. Конфиг: struct-level `#[serde(default)]`
(config.rs:26) — старый config.json ок; координаты physical px (get_window_rect
:619 ↔ move_window_pos_only :606) — консистентно, отрицательный x портрета ок;
enum_monitors (win32.rs:317) + MonitorRect есть для clamp.

**A — план ЧИНИТ правильную функцию, но 3 NEEDS-FIX:**
1. Корень подтверждён (context_builder.rs:82-93 без вопроса; cap = `.take(8)` в
   format_memory_block:36; store УЖЕ отдаёт ВСЕ активные, limit -1).
2. **Фактическая ошибка:** `search_index` — НЕ external-content (0002_fts.sql:11 —
   standalone FTS5, триггеры INSERT-only :19-32; корпус append-only, там хватает).
   `memory_items` НЕ append-only: text меняют update_memory_item_text
   (sqlite_store.rs:634, правка из v0.26!) и update_memory_item_normalized (:661,
   M1-нормализатор), плюс archive UPDATE (:674) и hard DELETE (:684). INSERT-only
   триггеры = ТИХО протухающий индекс (правленый факт ищется по СТАРОМУ тексту).
   Нужны AI+AU+AD-триггеры; archived фильтровать НЕ в индексе, а JOIN'ом
   `memory_items ... archived_at_ms IS NULL`.
3. **Склонения:** живой тест — `влад*`→«Владислав» ✓ (кириллица кейс-фолдится ✓),
   bm25 ✓ (уже в проде sqlite_store.rs:404), НО `влада*` (вопрос «у Влада») НЕ
   матчит «владислав» (0 hits). ADR это предвидел (memory-architecture.md:109-110,
   drop-last-char-префикс) — план деталь потерял. Фикс: токен ≥5ch → добавить
   стем без 1-2 последних букв с `*`, OR-join. Санитайзация ОБЯЗАТЕЛЬНА
   (`кто-то`/`вопрос:`/`"` → OperationalError, проверено); паттерн `fts_query`
   (aux_windows.rs:1238) реюзать, но join = " OR " (там пробел = неявный AND).
   `normalize::tokenize` (normalize.rs:157) и `is_stopword` (candidates.rs:156)
   ПРИВАТНЫ — поднять до pub(crate).
4. **Колл-сайтов ДЕВЯТЬ, и runtime — НЕ саммери:** runtime.rs:1437 = F3-реаск
   (`last_q` :1431), runtime.rs:1727 = F6 (`line.text` :1722) — оба С вопросом,
   передавать его, не None. Ни один summary-путь context_for_meeting не зовёт.
   vision_capture.rs:494 (Describe) вопроса НЕ имеет (статический промпт) → None.
   tile_ask.rs:400: typed_question при ✏, для голого F9 — None. overlay_host.rs:1828
   = V5-сидинг (`req.spec.question` :1829), «palette» в плане — мислейбл.
   slint_session.rs:703: `text` есть; QA-кэш уже хэширует question-префикс — ок.
5. Скип summary-пути (term_in_tokens summary_ref.rs:118) — DEFENSIBLE, приёмка
   ask-only.

**Ponytail (главное):** memory_items — десятки-сотни строк, и функция УЖЕ грузит
их ВСЕ. Альтернатива без миграции/триггеров/MATCH-инъекций: чистый Rust-скорер
над загруженным vec (токенизация вопроса + симметричный префикс-матч
a.starts_with(b)||b.starts_with(a), min 4ch — кроет Влад↔Владислава в ОБЕ стороны,
чего FTS5-префикс не умеет; сорт по (score, approved_at_ms)); format_memory_block
не меняется. A сжимается M→S, пункты 2-3 выше исчезают. FTS5 вернуть по триггеру
самого ADR (~150-200+ фактов) — ponytail-коммент. Если владелец настаивает на
M2-как-в-ADR — применить фиксы 2-4.

**Порядок B→C→A — верный.** Вердикт: B ship-as-planned, C ship с 3 поправками,
A НЕ начинать до решения FTS-vs-Rust-скорер + фиксов 2-4.
