# Текущее состояние репозитория Suflyor (CURRENT_STATE.md)
**Дата актуализации:** 2026-10-02
**Активная ветка:** `codex/research-reconciliation`
**Текущая версия продукта:** `0.38.1-rc.4` (`slint-experiment/Cargo.toml` и `scripts/slint-installer.nsi`)
**Базовый зафиксированный коммит аудита:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`
**Коммит архива полного исследования (119/119):** `090c8a19a6bdbc97985f6131127426821a46eff3`

---

## 1. Архитектура и компоненты (Pure Rust + Slint)

В репозитории 5 независимых крейтов Rust (без корневого Cargo workspace) и вспомогательные сайдкары:
- **`slint-experiment/` (`overlay-host`):** Оверлейный графический интерфейс Slint, трей, Win32/macOS управление окнами, WDA stealth, опрос горячих клавиш.
- **`overlay-backend/`:** Доменный движок (UI-free): клиенты AI (Cloud/Local), захват звука WASAPI/CoreAudio, распознавание речи GigaAM (in-process ONNX), SQLite каталог сессий `Store`, долговременная память, управление локальными серверами (JobObject).
- **`suflyor-tts/` (`suflyor-tts.exe`):** Сайдкар нейросетевого синтеза речи Piper (sherpa-onnx) и диаризации. Изолированный процесс (правило запрета двух ONNX runtime в одном бинарнике).
- **`suflyor-teratts/` (`suflyor-teratts.exe`):** Экспериментальный сайдкар TeraTTSv2 (ort). Отдельный процесс.
- **`suflyor-wsola/`:** Вспомогательная библиотека растяжения аудио по времени с сохранением высоты тона.
- **`suflyor-mlx/`:** Сайдкар для Apple Silicon (Swift/MLX).
- **`scripts/`:** Нативные скрипты сборки, инсталлятор NSIS, нативный шлюз (`git-gate-native.ps1`).

---

## 2. Активные правила и контракты

1. **Единая точка правды:**
   - Корневой `AGENTS.md` — общие правила для агентов.
   - `docs/agent-contract.md` — оперативный контракт и правила безопасности.
   - `CURRENT_STATE.md` — этот файл.
   - `DEFECTS.md` — журнал дефектов (P0–P3).
   - Все старые чартеры `goal-*.md`, отчёты `audit-*` и чек-листы `retest-*.html` перенесены в архивную историю Git (доступ через `git show 090c8a19:<путь>`).
2. **Правила Git:**
   - Никаких прямых коммитов в `master`.
   - Одно изменение — одна ветка — понятный коммит — PR.
   - Слияние только после зелёного CI на целевых платформах.
   - История Git не переписывается.
3. **Удалённые воркеры homelab:**
   - `windows-worker` (WINBRAT): сборка и тестирование Windows (`cargo test`, `git-gate-native.ps1`).
   - `mac-worker` (mm4.local): сборка и тестирование macOS (`memory_pressure >= 40%`, `CARGO_BUILD_JOBS=2`).
   - На контроллере DSH компиляция и тяжёлые тесты не запускаются.

---

## 3. Статус тестов и проверок

- **Rust unit/integration тесты:** 1 221 тест в кодовой базе (overlay-backend: 827, slint-experiment: 292, suflyor-teratts: 75, suflyor-tts: 22, suflyor-wsola: 5).
- **Исследовательские тесты аудита (Python):** 526 тестов в архиве ветки `090c8a19` со 100% покрытием всех 119 пунктов Grok.
- **Интеграция Hermes:** 3 теста в `integrations/hermes-plugin/tests/`.
