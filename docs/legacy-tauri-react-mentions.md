# Legacy Tauri/React Mentions

Короткий список мест, где документация все еще говорит про старый Tauri/React/WebView2 стек.

- `CLAUDE.md:33` — предупреждает, что React/Tauri stack был удален, а упоминания `src-tauri / React / WebView2 / npm` исторические.
- `CLAUDE.md:71` — команды проверки завязаны на старый путь `src-tauri/Cargo.toml`.
- `CLAUDE.md:79` — говорит, что все изменения Tauri windows, React или CSS требуют полного набора проверок.
- `CLAUDE.md:141` — старый способ сборки: `npm run tauri build`, с упоминанием Vite frontend bundle.
- `CLAUDE.md:157` — упоминает ожидание `WebView2 paint`.
- `docs/architecture.md:59` — старый UI-пайплайн: `React TileWindow renders ReactMarkdown + remark-gfm`.
- `docs/architecture.md:62` — раздел `Tauri 2 security model`.
- `docs/architecture.md:77` — старый роутинг: `main.tsx dispatches by URL query param`.
- `docs/architecture.md:89` — хоткеи описаны через события в React: `emit ... -> React`.
- `docs/architecture.md:105` — описание бага старого React/WebView состояния в `Settings.tsx`.
- `docs/architecture.md:117` — таблица critical files указывает старые пути `src-tauri/src/lib.rs`, `src/Overlay.tsx`, `src/Settings.tsx`, `src/TileWindow.tsx`.
- `docs/architecture.md:169` — старые dev/build команды: `npm run dev`, `npm run tauri dev`, `npm run tauri build`.
- `docs/security-audit-2026-05-26.md:34` — описывает стек как `Tauri 2 + React 19 + Vite 7 + react-markdown`.
- `docs/security-audit-2026-05-26.md:82` — старый markdown-render: `TileWindow renders AI responses via ReactMarkdown + remark-gfm`.
- `docs/ADR-001-stack.md:1` — ADR целиком про выбор `React/Tauri vs Iced/Slint/Dioxus`.
- `docs/ADR-001-stack.md:3` — устаревшее решение: `keep React/Tauri`.
- `docs/REVIEW_AGENT_PROMPT.md:13` — описывает проект как `Tauri 2 + React 19`.
- `docs/PHASE-7-CUT-PLAN.md:1` — план удаления старого стека: `removing React/Tauri from the repo`.
- `docs/MIGRATION-PLAN-SLINT.md:216` — фаза миграции: `Cut React, ship v0.2.0`.

Самые явно устаревшие файлы: `docs/architecture.md`, `CLAUDE.md`, `docs/security-audit-2026-05-26.md`, `docs/ADR-001-stack.md`, `docs/REVIEW_AGENT_PROMPT.md`.

