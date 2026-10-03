# Отчёт о сборке и полевой верификации релизного кандидата v0.38.1-rc.5 (RELEASE-EVIDENCE-RC5.md)

**Дата:** 2026-10-03  
**Версия:** `0.38.1-rc.5`  
**Базовый коммит исходников:** `7cfcd4d9f62edf19b1a7e2328de3e86d32fea6d5`  
**Целевой воркер сборки:** `windows-worker` (WINBRAT, Windows 11 x86_64 MSVC)  
**Инструмент упаковщика:** NSIS v3.x (`makensis.exe`)

---

## 1. Скомпилированные бинарные артефакты

Сборка выполнена по официальному скрипту `scripts/build-slint-release.ps1 -Installer` в релизном профиле:

| Артефакт | Размер | SHA-256 Хэш | Назначение |
|---|:---:|---|---|
| **`overlay-host.exe`** | 69.38 MB | `48457E1BACE1215CDCFF8C6FD7A355EF7996B033D7BE30FDD1F7FB6A2FF5F9B6` | Основной исполняемый файл приложения (Slint UI + хост) |
| **`suflyor-tts.exe`** | 18.84 MB | `BD393508CB001869967B5CB228006EEB848C3A46117A1CBDECD759D28E868551` | Сайдкар нейросетевого синтеза речи Piper (sherpa-onnx) |
| **`suflyor-teratts.exe`** | 22.00 MB | `46DFBB972CF4FB94BB648D11F6F17AF8C62D83ADD58CEF0A9ED7AD14C229EB94` | Сайдкар экспериментального синтеза речи TeraTTSv2 |
| **`DirectML.dll`** | 17.67 MB | `9C9E6D822561C6C41B90E6994B3E8857CF1D66DBFB1E0C4C799C7C89B4E92DA1` | Редистрибутивная библиотека DirectML 1.15.4 (DML-fac7597) |
| **`suflyor-slint-setup.exe`** | 37.34 MB | `1129FE3600B837115644C24C8B528CA14367E1356476F2626166ADA42B6A5BA3` | Готовый установщик NSIS для конечных пользователей |

---

## 2. Результаты тестов и статических проверок

1. **Тест синхронизации версий (`version_guard`):**
   ```
   running 2 tests
   test cargo_toml_version_matches_macos_info_plist_version ... ok
   test cargo_toml_version_matches_nsi_product_version ... ok
   test result: ok. 2 passed; 0 failed; finished in 0.00s
   ```
2. **Проверка манифестов:**
   - `slint-experiment/Cargo.toml`: `version = "0.38.1-rc.5"`
   - `scripts/slint-installer.nsi`: `!define PRODUCT_VERSION "0.38.1-rc.5"`
   - `slint-experiment/macos/Info.plist`: `0.38.1-rc.5`
   - `overlay-host.exe` FileVersion: `0.38.1-rc.5`

---

## 3. Полевая проверка запуска на физической машине (Smoke Test)

- **Исполняемый файл:** `C:\suflyor-release-rc5\slint-experiment\target\release\overlay-host.exe`
- **Прогон жизненного цикла:** Запуск процесса в фоне (`PID 4188`), мониторинг в течение 5 секунд, штатная работа, корректная запись в лог `%APPDATA%\suflyor\overlay-host.log`.
- **Лог запуска:**
  ```
  [01:49:08Z] === suflyor overlay-host v0.38.1-rc.5 start ===
  [01:49:08Z] config loaded: ai_provider=codex model=gpt-mock-safe
  [01:49:08Z] [overlay-host] catalog: indexed 1 sessions (2 skipped, 0 failed)
  [01:49:08Z] bar pinned at (0, 24)
  ```
- **Обработка окружения:** В виртуальной машине без GPU приложение корректно распознает отключённый DWM/OpenGL и штатно откатывается на программный рендеринг (`SLINT_BACKEND=software`), не падая и не блокируя пользователя.
