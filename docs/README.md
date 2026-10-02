# Документация Suflyor (docs/)

Этот каталог содержит актуальное ядро оперативной документации проекта Suflyor. Весь исторический балласт (старые ретесты, дампы скриншотов, выполненные планы миграций) архивирован в историю Git для поддержания чистоты репозитория.

---

## 1. Активные документы

- **[`AGENTS.md`](AGENTS.md)** — таксономия документации и общие правила сопровождения файлов.
- **[`agent-contract.md`](agent-contract.md)** — операционный контракт для работы агентов (методология VPNRouter).
- **[`architecture.md`](architecture.md)** — архитектурный обзор приложения (Rust + Slint, 5 крейтов, сайдкары).
- **[`winbrat-recovery.md`](winbrat-recovery.md)** — регламент обслуживания и восстановления Windows-воркера (WINBRAT).
- **[`read-aloud-status.md`](read-aloud-status.md)** — архитектурные инварианты и статус сайдкаров TTS (Piper) и OCR (Tesseract).
- **[`memory-architecture.md`](memory-architecture.md)** — архитектура долговременной памяти и схемы SQLite.
- **[`slint-design-system-and-safe-redesign-plan.md`](slint-design-system-and-safe-redesign-plan.md)** — дизайн-система Slint UI и цветовые токены.
- **[`retest-template.html`](retest-template.html)** — золотой шаблон чек-листа для верификации релизов.
- **[`REVIEW_AGENT_PROMPT.md`](REVIEW_AGENT_PROMPT.md)** — регламент независимого рецензирования кода перед слиянием.

---

## 2. Доступ к архивированным документам

В соответствии с правилом «История не переписывается», все исторические документы и отчёты сохранены в истории коммитов Git и доступны для чтения в любой момент:

- **Базовый коммит завершённого исследования Grok (119 кандидатов, квитанции, синтаксическая карта):**
  `090c8a19a6bdbc97985f6131127426821a46eff3` (ветка `codex/research-reconciliation`).
- **Просмотр любого архивированного файла без извлечения на диск:**
  ```bash
  git show 090c8a19:docs/retest-v0.38.1-rc.4-fixes.html
  git show 090c8a19:docs/audit-2026-08-03-ui-visual-method/README.md
  git show 090c8a19:docs/agent-map/reconciliation/candidates.json
  ```
- **Просмотр списка файлов в архиве:**
  ```bash
  git ls-tree -r --name-only 090c8a19 docs/
  ```
