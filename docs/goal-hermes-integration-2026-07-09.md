# Hermes × suflyor — интеграция (ТЗ владельца 2026-07-09)

Хотелки владельца: (1) разрешать Hermes читать созвоны; (2) дергать Hermes для
подготовки профиля созвона; (3) отдавать профиль из suflyor в Hermes; (4) память
suflyor→Hermes и Hermes→suflyor. Работа соло, без агентов ([[no-subagents-solo-work]]).

## Разведанные факты (всё проверено по коду, не по докам)

**Hermes у владельца:** ЛОКАЛЬНЫЙ hermes-agent 0.18.0 (Nous Research), editable-инсталл
в `%LOCALAPPDATA%\hermes\hermes-agent` (полные исходники). `~/.hermes` ещё НЕ создан
(не инициализирован). Мозг = llama.cpp Qwen-35B на `0.0.0.0:8080` (жив; к нему же
ходит ops-1 по tailnet). API-сервер Hermes: платформа gateway, включается в
`~/.hermes/config.yaml` → `platforms.api_server.{enabled:true, extra.key:...}`,
дефолт 127.0.0.1:8642, bearer = key, `/v1/chat/completions` (+ /runs, /models).
Плагины: `~/.hermes/plugins/<name>/` = `plugin.yaml` (kind: standalone) +
`__init__.py::register(ctx)`; `ctx.register_tool(name, toolset, schema, handler,
check_fn, emoji)` (handler sync/async fn(args)->str), `ctx.register_command()` для
/слэш-команд; включение: `plugins.enabled: ["suflyor"]`. Зависимости плагину не
нужны — stdlib urllib.

**suflyor:** входящего HTTP нет (инвариант «no IPC surface» — осознанно ломаем
одним выключенным-по-умолчанию мостом, это ADR-решение этого дока). Стор имеет всё
для чтения: `list_sessions/get_session/session_utterances/session_ai_turns/search`
(FTS5 bm25), саммери = `AiTurn.purpose=="summary"`. Запись в память ЕСТЬ безопасная:
`insert_candidate(NewMemoryCandidate)` → очередь предложений → владелец одобряет в
«Память» (инвариант одобрения сохраняется!). Профили: `ContextProfile{name,context}`
в конфиге + `active_profile`. tokio/serde_json/reqwest в backend есть; для сервера
берём `tiny_http` (sync, крошечный) в отдельном потоке.

## Архитектура (3 части)

### 1. Мост в suflyor (Hermes → suflyor), `overlay-backend/src/bridge.rs`
- tiny_http, bind СТРОГО `127.0.0.1:<port>` (дефолт 8654), Bearer-токен обязателен
  (генерится при включении), выключен по умолчанию.
- Конфиг: `hermes_bridge_enabled=false`, `hermes_bridge_port=8654`,
  `hermes_bridge_token=""`.
- Эндпоинты (JSON): GET /health · GET /sessions?limit · GET /sessions/{id}
  (транскрипт, кап по символам) · GET /sessions/{id}/summary · GET /search?q&limit ·
  GET /memory (approved) · GET /profiles · POST /memory/suggest {text,reason?}
  (→ очередь одобрения, НЕ в approved) · POST /profiles {name,context,set_active?}
  (upsert). 401 без токена; body-кап 256KB; никаких секретов в ответах; тела не
  логируются.
- Дизайн для тестов: чистая `dispatch(method, path, query, body) -> (status, json)`
  поверх Store/SharedConfig; tiny_http только в тонком цикле потока.
- Тумблер в Настройках поднимает/глушит сервер на лету (AtomicBool + unblock()).

### 2. suflyor → Hermes: подготовка профиля
- Конфиг: `hermes_api_url="http://127.0.0.1:8642/v1"`, `hermes_api_key=""`.
- Настройки → секция «Hermes»: url/key, «Проверить связь» (reuse `ai::test_connection`),
  тумблер моста + порт + токен (показ/копирование/регенерация).
- В секции профилей: поле «вводная» + кнопка «Подготовить профиль (Hermes)» →
  detached-запрос `/v1/chat/completions` (модель "hermes-agent", max_tokens большой,
  таймаут щедрый — агент может ресёрчить) → ответ = `ContextProfile{name: из вводной,
  context: ответ}` + активировать + статус. Живой путь ответов НЕ трогаем (латентность
  агента несовместима с лайвом) — Hermes только для тяжёлых задач.

### 3. Hermes-плагин (в нашем репо `integrations/hermes-plugin/suflyor/` + установка)
- plugin.yaml + __init__.py (stdlib urllib): тулзы suflyor_status, recent_sessions,
  get_transcript, get_summary, search, get_memory, suggest_memory, get_profiles,
  set_profile; слэш `/suflyor` (статус). env: SUFLYOR_BRIDGE_URL (дефолт
  http://127.0.0.1:8654), SUFLYOR_BRIDGE_TOKEN.
- Установка: скрипт copy → `~/.hermes/plugins/suflyor/` + README с точным сниппетом
  `~/.hermes/config.yaml` (api_server + plugins.enabled + model из кита владельца).

## Безопасность (репо публичный, данные — интервью)
Мост off-by-default; loopback-only; токен обязателен; чтение — только текст
(транскрипт/саммери/память/профили), НИКОГДА аудио/скрины/ключи; запись — только
очередь-предложений памяти + upsert профиля; тела запросов капятся; ответы об
ошибках generic. Плагин — только у владельца в ~/.hermes (не в PATH Hermes-репо).

## Порядок
P1 конфиг+bridge.rs+тесты → P2 wiring в host + Настройки (Hermes-секция) →
P3 подготовка профиля → P4 плагин+инсталл+README → gate → build/install/relaunch →
retest-HTML. Бэклог (не сейчас): AskRoute::Agent для «спросить агента» с тайла;
/runs+SSE прогресс; LiteLLM-роутинг.
