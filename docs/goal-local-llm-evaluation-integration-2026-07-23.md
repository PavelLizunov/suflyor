# План интеграции результатов local-LLM lab, 2026-07-23

## Статус документа

Это план, а не реализация. Код, конфигурация, модели и пользовательский интерфейс
в этой задаче не меняются.

Источник задания:

`https://github.com/PavelLizunov/local-llm-evaluation-lab/blob/codex/publish-v2-results/SUFLYOR-INTEGRATION-HANDOFF.md`

На 2026-07-23 получить содержимое источника из этой рабочей среды не удалось:
публичный web-доступ не видит файл, подключённый GitHub API возвращает 404 на
репозиторий, а прямой сетевой доступ из checkout запрещён. Поэтому ниже нет
выдуманных победителей, численных порогов или координат моделей. Первый этап
плана — обязательный source gate; до его закрытия нельзя менять поставляемые
модели или автоматическую policy выбора.

## Цель

Перенести в suflyor только проверенные выводы лаборатории:

- закрепить поддерживаемые профили моделей и их артефакты;
- выбирать или советовать профиль по фактическому железу без ложной уверенности;
- запускать его через существующий `llama-server`;
- проверять готовность до сохранения выбора;
- сохранить ручное управление и полностью локальный F8 vision;
- иметь воспроизводимый regression/live-eval для будущей смены модели.

Не переносить сам evaluation lab в приложение. Производственный стек остаётся
Pure Rust + Slint, а лабораторные скрипты и датасеты остаются инструментом
разработки.

## Что уже есть в suflyor

Текущая реализация уже закрывает большую часть механики:

- `overlay-backend/src/local_ai.rs` закрепляет URL, размер и SHA-256 Gemma 4B,
  Gemma 12B и обоих vision-проекторов;
- тот же модуль выбирает CUDA, Vulkan или CPU, скачивает `llama.cpp`, запускает
  сервер с context 8192 и умеет переключать 4B/12B;
- `slint-experiment/src/bin/overlay_host/settings_local_ai.rs` сериализует
  install/download/switch и записывает выбор только после успешного запуска;
- `slint-experiment/ui/settings_panel.slint` уже имеет один пользовательский
  сценарий «быстрее 4B / умнее 12B»;
- `overlay-backend/tests/ai_eval.rs` содержит model-free инварианты, но live
  проверка реального endpoint пока является заглушкой.

Главные дыры перед применением лабораторного результата:

1. Модель описана набором констант и `ai_local_quality: bool`, а не явным
   профилем с требованиями, vision-возможностями и provenance.
2. Определяется тип GPU, но не конкретный адаптер, доступная VRAM и доступная
   system RAM.
3. Переключатель считает `/v1/models` достижимым при любом HTTP-ответе; это не
   доказывает 2xx, загрузку нужного GGUF или рабочий completion.
4. При неуспешном переключении новый выбор не сохраняется, но прежний сервер не
   восстанавливается полноценной транзакцией.
5. Нет исполняемого suflyor-eval, который воспроизводит критичные сценарии из
   handoff на реальном локальном endpoint.

## Неподвижные правила решения

- Не добавлять Node.js, Ollama, Python или web engine в поставляемое приложение.
- Не вводить универсальный каталог произвольных моделей.
- Не скачивать и не переключать модель без явного действия пользователя.
- Не считать `Unknown` рекомендацией младшей модели.
- Не складывать VRAM нескольких карт и dedicated VRAM с shared RAM iGPU.
- Профиль — это модель плюс режим `TextOnly` или `VisionRequired`; нельзя
  рекомендовать text-only модель, если включён локальный F8.
- Все скачиваемые веса и проекторы должны иметь закреплённые URL, точный размер,
  SHA-256 и проверенную лицензию распространения.
- Ошибка новой модели не должна разрушать работавший прежний профиль.
- Лабораторный score служит входом в policy, но не подменяет проверку запуска на
  конкретном ПК.

## Этап 0. Зафиксировать handoff как проверяемый вход

До изменения кода получить доступный снимок указанного файла и записать в этом
документе:

- полный commit SHA ветки `codex/publish-v2-results`;
- SHA-256 самого handoff;
- дату и среду прогона;
- точные model id, GGUF-квантизации и vision-проекторы;
- версию/build `llama.cpp`, backend, полный канонический resource launch-plan и
  его SHA-256;
- тестовые сценарии, число прогонов и правила pass/fail;
- качество, TTFT, tokens/sec, peak VRAM и peak/available RAM;
- известные проигрыши, ограничения и confidence результата;
- лицензию каждого рекомендуемого артефакта.

Свести это в таблицу без интерпретации:

| Профиль | Роль | Артефакт | Качество | Производительность | Память | Решение |
|---|---|---|---|---|---|---|
| из handoff | основной/резервный/отклонён | exact id + hash | измерено | измерено | измерено | ship/не ship |

Source gate закрыт только если значения можно проверить по опубликованным
артефактам lab. При конфликте handoff с текущими живыми инвариантами suflyor
(vision, context 8192, RU/EN, streaming) сначала повторить лабораторный прогон,
а не ослаблять продуктовый контракт.

## Этап 1. Ввести manifest поддерживаемых профилей

Минимально вынести модельные данные из `local_ai.rs` в
`overlay-backend/src/local_ai/model_manifest.rs`.

Предлагаемые чистые типы:

```text
ModelId
VisionMode = TextOnly | Required
ModelProfile { model_id, vision_mode }
Artifact { url, file_name, size, sha256, license }
MemoryThreshold { measured_min_mib, recommended_min_mib }
VramRequirement = NotRequired | Threshold(MemoryThreshold)
LaunchResourcePlan {
  backend, llama_build, context_tokens, parallel_sequences, batch_size,
  ubatch_size, n_gpu_layers, device_binding, split_mode, tensor_split,
  main_gpu, kv_cache_types, kv_offload, flash_attention, op_offload,
  mmap, mlock, projector_artifact?, projector_offload
}
LaunchResourceFingerprint = sha256(canonical_v1(LaunchResourcePlan))
BackendMeasuredFit {
  launch_resource_fingerprint,
  free_vram: VramRequirement,
  available_system_ram: MemoryThreshold
}
SupportedProfile { profile, model, projector?, backend_measured_fits, eval_provenance }
```

`BackendMeasuredFit` — отдельная запись для каждого полного измеренного
`LaunchResourcePlan`, а не усреднение по backend/build/context. Каноническая
версия `v1` сериализует перечисленные поля в фиксированном порядке и явной
кодировке; абсолютные пути, порт и UUID конкретной лабораторной карты в неё не
входят. Полный runtime `LaunchPlan` добавляет к этому только профиль, проверенные
пути артефактов, стабильный alias, порт и текущую привязку адаптера.

Все влияющие на память или производительность значения должны быть явными, а не
зависеть от меняющихся default/`auto` llama.cpp. В fingerprint входят в том числе
слои GPU, выбор/разделение устройства, batch/parallel, KV-cache и его offload,
flash/op offload, mmap/mlock, а также exact projector и его offload. Новое
resource-влияющее поле сначала добавляется в каноническую версию и заново
калибруется; несовпадение fingerprint не может использовать старый fit и даёт
`Unknown`.

`measured_min_mib` — минимум, на котором именно этот полный профиль был
воспроизводимо проверен; `recommended_min_mib` — этот минимум с явно записанным
запасом, поэтому он не может быть ниже measured. Free VRAM и available system RAM
всегда имеют свои независимые пределы. Для CPU `free_vram = NotRequired`; ноль
не используется как неявный заменитель отсутствующей VRAM. Source gate обязан
записать peak, выведенные четыре порога и правило запаса, из которого они
получены.

Политика выбирает fit только при точном `LaunchResourceFingerprint` планируемого
запуска. Она не переносит результат CUDA на Vulkan/CPU или наоборот и не
смешивает пороги разных backend или launch-аргументов.

Manifest содержит только профили, прошедшие source gate. Для текущих Gemma
первый рефактор должен быть behavior-preserving: те же URL, размеры, SHA-256,
имена файлов и launch args. Замена или добавление модели идёт отдельным
инкрементом после тестов manifest.

Конфигурацию мигрировать аддитивно:

- добавить `ai_local_profile_id: Option<ModelProfileId>` и
  `ai_local_nvidia_adapter_uuid: Option<String>`;
- старые поля сохранять до успешного разрешения legacy-состояния, а не
  придумывать profile id по fallback-файлу;
- legacy-конфиг не получает UUID, вычисленный по имени или индексу GPU: до
  явной успешной пробы CUDA он остаётся `None`;
- после успешного switch сохранять profile id и UUID той NVIDIA-карты, на
  которой прошёл readiness, одной commit-операцией;
- неизвестный или удалённый id не угадывать: оставить текущий рабочий профиль и
  показать, что рекомендацию надо пересчитать.

Legacy resolver обязан быть чистым и иметь следующую таблицу, проверяемую без
запуска сервера. `VisionRequired` выбирается, когда `ai_local_vision == true`,
либо когда F8 фактически направлен в выбранный local text-server: это
`vision_provider == "same"` при `ai_provider == "local"`, или
`vision_provider == "local"` с пустым/совпадающим local base URL (его fallback
также ведёт к этому серверу). Если второй случай есть, а `ai_local_vision ==
false`, это противоречивое `NeedsVisionRepair`, которое не получает profile id.
Отдельный cloud endpoint, выключенный vision и отдельный local endpoint с другим
base URL не добавляют требования к *этому* профилю; неизвестный provider —
`NeedsVisionRepair`. Таким образом проверены и флаг capability, и фактический
маршрут F8, а не только `ai_local_quality`.

При `ai_local_quality == false` resolver выбирает только 4B-вариант нужного
vision-режима, причём vision-вариант требует точный projector. При
`ai_local_quality == true` он выбирает только 12B-вариант: отсутствие 12B GGUF,
а для `VisionRequired` отсутствие подходящего 12B projector или нужного build,
возвращает `NeedsRequestedArtifact`/`NeedsVisionArtifact`, но никогда не 4B и
никогда не 12B text-only. Пока результат не `Resolved`, прежний уже работающий
child не останавливается, новый profile id не сохраняется и следующий autostart
не подменяет намерение пользователя другим профилем; Settings предлагает явное
скачивание/исправление.

UUID — единственный сохранённый ключ CUDA-адаптера. На restart отсутствие или
исчезновение сохранённого UUID даёт явный `SavedAdapterMissing` / `NeedsAdapterBind`:
launcher не подставляет одноимённую карту, не меняет config и не уходит молча
на CPU/Vulkan. Только явная команда пользователя «привязать и проверить» может
выбрать обнаруженный UUID и сохранить его после полной транзакции readiness.
Для CPU-плана UUID не требуется; AMD/Intel не получают подменяющий UUID-механизм
и до калиброванного источника VRAM остаются `Unknown`.

Критерии этапа:

- один источник истины для download, label, projector и launch;
- ни один artifact не запускается до проверки размера и SHA-256;
- `components.rs`, active-stack label и Settings получают имя из manifest, а не
  распознают его по подстроке имени файла;
- старые `config.json` продолжают открываться; однозначно resolved legacy
  состояние сохраняет exact profile id, а неоднозначное остаётся явным pending
  состоянием без скрытого fallback;
- тесты migration покрывают обе величины `ai_local_quality`, наличие/отсутствие
  12B и projector, build-гейт, `ai_local_vision` и все `vision_provider`;
- config round-trip сохраняет profile id и UUID, но CUDA-autostart невозможен
  без явной привязки UUID.

## Этап 2. Нативный снимок железа и чистая policy

Добавить два небольших backend-модуля:

- `local_ai/hardware.rs` — сбор фактов;
- `local_ai/selection.rs` — чистое решение без IO.

`HardwareSnapshot` должен содержать:

- total/available system RAM;
- список адаптеров, а не один общий GPU;
- стабильный id адаптера, имя, backend, dedicated total/free VRAM;
- признак shared/unified memory;
- выбранный execution adapter и объяснение выбора;
- `Unknown` для каждой метрики, которую нельзя получить надёжно.

Для NVIDIA получать одной CSV-командой как минимум UUID, PCI bus id, name,
memory.total и memory.free. Сохранённым ключом является UUID; индекс и имя —
только отображение. Для AMD/Intel не считать `Win32_VideoController.AdapterRAM`
надёжной свободной VRAM и не выдавать положительную рекомендацию без
калиброванного источника.

Результат чистой policy:

```text
Recommended(profile, reason)
Borderline(profile, reason)
NotRecommended(reason)
Unknown(reason)
```

Правила:

- сравнивать только с измеренными порогами полного профиля из manifest;
- выбрать `BackendMeasuredFit` только для точного
  `LaunchResourceFingerprint` планируемого запуска и сохранённого execution
  adapter;
- при локальном F8 рассматривать только `VisionRequired`;
- `Recommended` требует измеренного backend, free VRAM не ниже собственного
  `recommended_min_mib` и available RAM не ниже собственного
  `recommended_min_mib`;
- `Borderline` разрешает только явную пробу пользователя, если обе известные
  метрики не ниже соответствующих `measured_min_mib`, но хотя бы одна ниже
  своего recommended-предела;
- `NotRecommended` возвращается при известной обязательной метрике ниже её
  measured-предела; VRAM `NotRequired` для CPU не участвует в сравнении;
- `NotRecommended` возможен только когда все нужные метрики известны;
- отсутствующая метрика или lab-профиль всегда дают `Unknown`;
- в первом MVP Vulkan остаётся `Unknown` даже при наличии lab-fit и достаточной
  памяти: `--list-devices` доказывает только доступность backend, а не то, что
  модель действительно offload-нулась на выбранный адаптер. Положительная
  Vulkan-рекомендация возможна лишь отдельным инкрементом с эквивалентным
  post-load доказательством backend, привязки устройства и числа offload layers;
- policy ничего не скачивает, не запускает и не пишет в конфигурацию.

Unit-тесты покрывают equality и 1 MiB ниже каждого из четырёх независимых
порогов (measured/recommended VRAM и measured/recommended RAM), а также fit
CUDA, Vulkan и CPU, чтобы порог одного backend не применялся к другому. Они
также меняют по одному полю `LaunchResourcePlan` (GPU layers, device/split,
KV/cache, projector/offload, batch/parallel) и требуют `Unknown` при новом
fingerprint; Vulkan с достаточным fit в MVP тоже обязан вернуть `Unknown`.
Дополнительно: CPU-only, dedicated GPU + iGPU, две GPU, перестановку одинаково
названных NVIDIA-карт, пропавший UUID, недостаточный vision-профиль и
отсутствие измерений.

## Этап 3. Один launch-plan и строгая готовность

Собрать аргументы `llama-server` чистой функцией
`launch_plan(profile, hardware, root)`. Ею должны пользоваться install,
autostart/watchdog, engine verify и ручное переключение.

План всегда передаёт `--alias <ModelId из manifest>` и этот же id в readiness
completion. Alias отделяет API-контракт от абсолютного `-m` пути, который
llama.cpp иначе возвращает как model id. Один и тот же конструктор возвращает
канонический `LaunchResourcePlan`/fingerprint и фактические argv/env; никакой
путь запуска не может добавлять resource-влияющий аргумент вне него.

Вход `launch_plan` включает сохранённые profile id и NVIDIA UUID. Сначала UUID
сопоставляется с текущим выводом `nvidia-smi`, затем план ограничивает
видимость именно этой карты и проверяет список устройств. Если UUID не найден
или виден не ровно один ожидаемый CUDA-девайс, функция возвращает явную ошибку
до spawn. Индекс GPU, порядок списка и display name не являются fallback.

Для выбранной NVIDIA-карты:

- ограничить видимость сохранённым UUID;
- проверить `--list-devices`;
- выбрать ровно одно ожидаемое CUDA-устройство;
- не переходить молча на другую карту.

После загрузки CUDA-план получает отдельное доказательство accelerator use, а
не только успешный HTTP. Для фиксированного llama build запуск захватывает
служебный stdout/stderr и build-specific parser должен подтвердить planned
число offloaded layers. Затем `nvidia-smi` обязан показать именно PID дочернего
`llama-server` на сохранённом UUID с ненулевой памятью процесса; проверка
выполняется после фиксированного completion, а не до model load. Несовпадение
PID/UUID, ноль offload, неизвестный формат доказательства или CPU fallback
отклоняет CUDA-план. Такое доказательство остаётся только диагностикой с
redaction путей и не показывается в UI. CPU-план доказывает именно CPU-режим;
неуспешный CUDA-тест не сохраняется как GPU-профиль и может предложить CPU
только как отдельный явный план пользователя, а не как скрытый success.

Тесты restart/launch обязаны доказать: сериализованный UUID переживает restart;
та же карта выбирается после перестановки строк `nvidia-smi`; пропавший UUID не
вызывает spawn и не изменяет config; явная повторная привязка меняет UUID лишь
после readiness, а следующий restart использует уже новый UUID.

Readiness-контракт после запуска:

1. Child остаётся жив.
2. `GET /v1/models` возвращает 2xx.
3. Ответ содержит ровно один `data[].id`, в точности равный manifest `ModelId`
   из `--alias`, а не путь GGUF.
4. Короткий `POST /v1/chat/completions` с тем же alias возвращает 2xx и валидный
   непустой `choices`.
5. Для `VisionRequired` отдельный короткий image request подтверждает projector.
6. Для CUDA post-load accelerator proof выше подтверждает backend, выбранный
   UUID и offload; для Vulkan в MVP результат остаётся `Unknown`.
7. Временный 503 во время прогрева повторяется до ограниченного deadline; 404,
   постоянный 503, malformed JSON, другая модель, отсутствие alias, failure
   accelerator proof и ранний child exit — ошибка.

`switch_local_model` сделать транзакцией:

- сохранить прежний `LaunchPlan` и tracked child;
- остановить только принадлежащий suflyor listener;
- запустить и проверить новый профиль;
- сохранить candidate-config через маленький внедряемый `ConfigStore` только
  после полной readiness, но до замены shared config, tracked handle и UI;
- только после успешного save заменить tracked handle, shared config и UI;
- при ошибке завершить новый child, освободить порт, восстановить прежний
  `LaunchPlan` и заново проверить readiness;
- при ошибке save candidate остаётся только в памяти, прежний on-disk и shared
  config не меняются, а новый child завершается до rollback;
- отдельно вернуть `RejectedAndRestored` и `RejectedRestoreFailed`, включая
  причину отказа save без технических деталей в UI;
- никогда не показывать новый профиль активным до commit транзакции.

Регрессии нужны для каждого отказа readiness и обязаны проверять не только
результат, но и восстановление старой модели, projector, config и child
tracking. В частности, fake `ConfigStore`, который детерминированно отвергает
atomic save, обязан доказать: новый child убит, прежний `LaunchPlan` снова
ready и tracked, disk/shared config и UI по-прежнему указывают на старый
профиль. Отдельные тесты argv/readiness требуют один `--alias`, точное
равенство id и отказ для пути GGUF, другого alias, нескольких записей или
не-2xx; CUDA evidence тестируется с mismatched PID/UUID и нулевым offload.

## Этап 4. Встроить рекомендацию в существующие Настройки

Не создавать новый экран. В текущем AI-разделе под выбором локальной модели
добавить компактный блок:

- выбранный адаптер и доступную память;
- `Recommended` / `Borderline` / `NotRecommended` / `Unknown`;
- короткую причину без сырых URL, LAN-IP и командной строки;
- кнопку явного download/test/switch только там, где действие возможно;
- ручной выбор поддерживаемых установленных профилей всегда оставить.

При `Unknown` текст не должен обещать, что младшая модель точно запустится. При
`Borderline` действие называется пробой, а не безопасной рекомендацией. После
ручного выбора приложение не переключает профиль самостоятельно на старте или
между запросами.

Новые user-facing строки в `.slint` оформляются через `@tr`, получают русские
`msgid/msgstr`, используют ASCII/SVG вместо редких Unicode-глифов. Все новые
transient status/result properties сбрасываются в `populate_token_status`.

## Этап 5. Превратить live-eval заглушку в воспроизводимую проверку

Расширить `overlay-backend/tests/ai_eval.rs`, не добавляя lab runtime в продукт:

- оставить pure-инварианты в обычном `cargo test`;
- добавить недефолтную Cargo feature `local-ai-eval`; ignored live-тест
  компилируется и запускается только с `SUFLYOR_EVAL=1` и этой feature;
- endpoint, model и fixture задавать явно, без чтения пользовательского
  `%APPDATA%\suflyor\config.json`;
- не менять body или defaults production `ai::complete`. В feature-gated
  test-only API `ai::complete_for_eval` вынести построение request body и
  передавать только туда закреплённые `EvalDecoderSettings`:
  `seed=424242`, `temperature=0.0`, `top_k=1`, `top_p=1.0`, `min_p=0.0` и
  `repeat_penalty=1.0`. Если source gate фиксирует другую поддерживаемую
  версией llama.cpp детерминированную схему, поменять весь набор и fixture
  одной проверяемой записью, а не молча полагаться на server defaults;
- test-only путь делает один non-stream request с этими полями, тогда как
  существующий `ai::complete` продолжает отправлять ровно текущий production
  request без decoder-параметров и сохраняет свои retry/no-think semantics;
- unit-тест с capture-server проверяет точный JSON test-only body и отдельно
  отсутствие новых decoder-полей в body `ai::complete`;
- live-eval записывает и сверяет с fixture profile id, exact `--alias`, hashes
  артефактов, llama build, `LaunchResourceFingerprint`, fixture version и полный
  `EvalDecoderSettings` (включая их канонический hash). В отчёт попадают только
  эти метаданные и агрегаты, не endpoint и не ответы модели;
- прогнать закреплённые сценарии handoff через `ai::complete_for_eval`;
- проверять структуру summary, отсутствие translation echo, clean session name,
  RU/EN, follow-up, код и vision для соответствующего профиля;
- сохранять только агрегаты и идентификаторы fixture, не transcript/ответы с
  пользовательскими данными;
- падать при несовпадении profile id/alias, llama build, resource fingerprint,
  fixture version или decoder settings.

Команда live-проверки документируется с `--features local-ai-eval`; feature не
включается зависимостями продукта, installer и обычным CI. Это устраняет
flakiness output-проверок, не меняя поведение обычных completion-запросов.

Lab остаётся местом сравнительного исследования моделей. Репозиторий suflyor
хранит маленький acceptance-набор, необходимый для предотвращения регрессии
уже выбранных профилей.

## Этап 6. Порядок поставки

Делать отдельными проверяемыми инкрементами:

1. Source gate и зафиксированная таблица результатов — docs only.
2. Manifest + config migration — без изменения выбранной модели.
3. Hardware snapshot + pure policy — без UI и автоматических действий.
4. Общий `LaunchPlan`, strict readiness и rollback переключения.
5. Один блок рекомендации в Settings.
6. Live-eval и Windows acceptance.
7. Только затем — отдельная замена/добавление модели из handoff.

На каждом кодовом инкременте: focused tests, затем полный `scripts/ci.ps1`.
Перед выпуском владелец отдельно проверяет на Windows:

- NVIDIA с достаточной и занятой VRAM;
- AMD/Intel или неизвестную VRAM;
- CPU-only;
- две GPU;
- cold start и прогрев с несколькими 503;
- неуспешный новый профиль с рабочим rollback;
- F8 на каждом `VisionRequired` профиле;
- переключение и перезапуск приложения;
- отсутствие автоматической загрузки/смены модели.

## Предполагаемый минимальный file set реализации

Первый полный MVP должен уложиться примерно в:

- `overlay-backend/src/local_ai.rs`;
- `overlay-backend/src/local_ai/model_manifest.rs`;
- `overlay-backend/src/local_ai/hardware.rs`;
- `overlay-backend/src/local_ai/selection.rs`;
- `overlay-backend/src/local_ai/tests.rs`;
- `overlay-backend/src/config.rs` и его тесты;
- `overlay-backend/Cargo.toml` (только недефолтная feature live-eval);
- `overlay-backend/tests/ai_eval.rs`;
- `slint-experiment/src/bin/overlay_host/settings_local_ai.rs`;
- `slint-experiment/src/bin/overlay_host/settings_controller.rs`;
- `slint-experiment/ui/settings_panel.slint`;
- `slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po`.

Если source gate показывает, что нынешние Gemma остаются победителями, отдельная
замена моделей не нужна: MVP сводится к manifest, честной рекомендации,
readiness/rollback и живому regression-eval.

## Не входит в задачу

- встраивание evaluation lab или его Python/Node-зависимостей;
- Ollama и второй менеджер `llama-server`;
- динамический Hugging Face/Ollama-каталог;
- скачивание произвольного GGUF;
- автоматический benchmark с многогигабайтной загрузкой;
- автоматическая смена модели во время встречи;
- multi-GPU split без отдельного измеренного профиля;
- релиз, installer или изменение версии до owner acceptance.

## Definition of done

- Source gate закрыт доступным immutable-снимком handoff.
- В код попали только модели и пороги, прямо подтверждённые этим снимком.
- Выбор детерминирован, объясним и unit-tested; `Unknown` безопасен.
- Каждый backend-fit имеет отдельные measured/recommended пределы free VRAM и
  available RAM; границы и точный `LaunchResourceFingerprint` проверены
  независимо.
- Legacy migration никогда не превращает запрошенный 12B в 4B и не запускает
  vision-намерение как text-only; все комбинации `ai_local_vision` и
  `vision_provider` покрыты тестом.
- После restart CUDA запускается только на сохранённом UUID; отсутствующий UUID
  требует явной перепривязки и никогда не заменяется другой картой.
- Все пути запуска используют один `LaunchPlan` с manifest `--alias`.
- Готовность проверяет exact alias model и реальный completion/vision; CUDA
  также доказывает PID+UUID и planned offload после model load, а Vulkan до
  эквивалентного доказательства остаётся `Unknown`.
- Неудачный switch, включая failed atomic config save, восстанавливает прежний
  рабочий профиль и не расходится с disk/shared config/UI.
- UI ничего не скачивает и не переключает без явного клика.
- Обычный gate зелёный, а ignored live-eval с `local-ai-eval` зелёный на
  заявленных Windows-профилях и фиксирует decoder settings.
- Владелец визуально проверил изменённый AI-раздел и живьём проверил F8/rollback.
