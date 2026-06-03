# Overlay host modularization plan

Дата среза: 2026-06-03.

Цель: разрезать `slint-experiment/src/bin/overlay_host.rs` на несколько
маленьких файлов без изменения поведения приложения.

Этот документ является инструкцией к реализации. Код приложения здесь не
меняется.

## 1. Текущее состояние

На момент скана:

- `slint-experiment/src/bin/overlay_host.rs` примерно `9884` строки;
- размер файла примерно `437 KB`;
- файл уже изменен в рабочем дереве;
- рядом есть библиотечные модули:
  - `app_state.rs`;
  - `capture.rs`;
  - `logging.rs`;
  - `markdown.rs`;
  - `runtime_state.rs`;
  - `slint_events.rs`;
  - `slint_session.rs`;
  - `win32.rs`.

Текущий `overlay_host.rs` является одновременно:

- composition root приложения;
- владельцем Slint generated UI types;
- registry открытых окон;
- router hotkeys;
- Settings controller;
- Diagnostics controller;
- Wizard controller;
- tile manager;
- Vision/capture flow;
- PTT/follow-up/regenerate flow;
- updater UI;
- local-AI installer UI;
- recovery offer;
- copy/report helpers;
- набором regression tests.

Главный риск: новые окна и режимы добавляются вручную в несколько разных мест.
Из-за этого уже появлялся класс ошибок, где окно создано правильно, но забыто в
stealth/theme/lifecycle синхронизации.

## 2. Главный принцип

Разбиение должно быть сначала механическим, а не продуктовым.

Правильно:

- вынести код в новый файл;
- оставить поведение тем же;
- прогнать проверки;
- сделать ручной smoke;
- только потом переходить к следующему контуру.

Неправильно:

- одновременно переносить код и менять UX;
- одновременно чинить старые баги и менять архитектуру;
- сразу тащить все в `src/lib.rs`;
- делать большой `AppContext`, который знает вообще все.

## 3. Где создавать новые файлы

На первом этапе лучше держать модули рядом с бинарем:

```text
slint-experiment/src/bin/overlay_host.rs
slint-experiment/src/bin/overlay_host/
  window_lifecycle.rs
  diagnostics.rs
  hotkeys.rs
  wizard.rs
  recovery.rs
  vision_capture.rs
  local_ai_ui.rs
  updater_ui.rs
  settings_controller.rs
  tile_controller.rs
```

В `overlay_host.rs` подключать их явно:

```rust
#[path = "overlay_host/window_lifecycle.rs"]
mod window_lifecycle;
#[path = "overlay_host/diagnostics.rs"]
mod diagnostics;
```

Почему не сразу `src/lib.rs`: многие функции завязаны на `mod ui { slint::include_modules!(); }`
и конкретные Slint-типы (`TileWindow`, `SettingsWindow`, `WizardWindow`,
`OverlayBarWindow`). Перенос в library crate потребует отдельного решения, где
живет generated UI API. Это можно сделать позже, но не первым шагом.

## 4. Что должно остаться в `overlay_host.rs`

После первого большого этапа `overlay_host.rs` должен быть composition root:

- `mod ui`;
- основные `use`;
- создание `OverlayBarWindow`;
- создание shared state;
- создание runtime/events/bridge;
- создание window slots;
- вызовы `wire_*` функций из модулей;
- запуск `overlay.run()`;
- минимальные glue-типы, которые пока слишком связаны с `main`.

Целевой размер после первой волны: меньше `3000-4000` строк.

Идеальный финальный размер: меньше `1500-2500` строк.

## 5. Кандидаты на перенос

### 5.1. `window_lifecycle.rs`

Перенести первым.

Сюда должны уйти:

- `TileWindows` или новый wrapper вокруг него;
- `set_global_stealth`;
- `global_stealth`;
- `set_global_scheme`;
- `global_scheme`;
- `set_global_tile_opacity`;
- `global_tile_opacity`;
- `present_window_stealth_aware`;
- `apply_scheme_*`;
- `refresh_open_tiles`;
- единый `apply_stealth_to_open_windows(on)`;
- единый `apply_scheme_to_open_windows(scheme)`;
- window registry / surface registry.

Главная задача: больше не должно быть трех-четырех мест, где вручную
перечисляются `tiles`, `palette`, `settings`, `text_ask`, `wizard`, `help`.

Минимальный целевой API:

```rust
pub(crate) struct WindowRegistry {
    pub tiles: TileWindows,
    pub settings: Rc<RefCell<Option<SettingsWindow>>>,
    pub palette: Rc<RefCell<Option<PaletteWindow>>>,
    pub text_ask: Rc<RefCell<Option<TextAskWindow>>>,
    pub wizard: Rc<RefCell<Option<WizardWindow>>>,
    pub help: Rc<RefCell<Option<HelpWindow>>>,
}

impl WindowRegistry {
    pub(crate) fn apply_stealth(&self, on: bool);
    pub(crate) fn apply_scheme(&self, scheme: i32);
    pub(crate) fn refresh_tiles_chip(&self, overlay: &OverlayBarWindow);
}
```

Capture overlay можно оставить отдельным контуром: он persistent, pre-stealthed и
не должен жить в обычном registry на тех же правилах.

Критерии приемки:

- stealth toggle из bar обновляет все открытые окна;
- stealth toggle из Settings обновляет все открытые окна;
- stealth toggle из Wizard обновляет все открытые окна;
- Help не выпадает из stealth;
- новые окна имеют один понятный способ зарегистрироваться.

### 5.2. `diagnostics.rs`

Перенести вторым.

Сюда должны уйти:

- `HotkeyDiag`, если `hotkeys.rs` еще не создан;
- `hotkey_diag_row`, либо чтение результата из `hotkeys.rs`;
- `populate_diagnostics`;
- `build_diag_report`;
- `redact_ipv4`;
- `redact_urls`;
- helpers redaction;
- wiring `on_diagnostics_check_all_clicked`;
- wiring `on_diagnostics_copy_report_clicked`;
- Vision live-check для `Check all`;
- форматирование diagnostics rows.

Лучше разделить внутри файла:

```text
config snapshot/readiness
live checks
report building
redaction
slint wiring
tests
```

Критерии приемки:

- `Check all` проверяет AI, STT, mic, system audio и Vision;
- `Copy report` не раскрывает bearer/API keys/transcript/profile/screenshot;
- report честно различает `configured` и `live checked`;
- все старые copy/redaction tests перенесены и проходят.

### 5.3. `hotkeys.rs`

Перенести третьим.

Сюда должны уйти:

- создание `GlobalHotKeyManager`;
- регистрация F1/F3/F4/F6/F8/F9/Shift+F9;
- структура результата регистрации;
- polling timer;
- dispatch hotkey events.

Важно: `hotkeys.rs` не должен напрямую знать всю бизнес-логику. Лучше дать ему
набор callbacks:

```rust
pub(crate) struct HotkeyActions {
    pub toggle_palette: Rc<dyn Fn()>,
    pub toggle_help: Rc<dyn Fn()>,
    pub manual_tile: Rc<dyn Fn()>,
    pub vision_capture: Rc<dyn Fn(bool)>,
    pub ask: Rc<dyn Fn(bool)>,
}
```

Можно начать проще: перенести регистрацию и diag-result, а dispatch оставить в
`main`. Главное не делать огромный перенос за один раз.

Критерии приемки:

- все hotkeys регистрируются как раньше;
- конфликт конкретной клавиши виден в Diagnostics;
- F1/F3/F4/F6/F8/F9/Shift+F9 работают вживую;
- hotkey manager unavailable не показывается как конфликт всех клавиш.

### 5.4. `wizard.rs`

Перенести после `window_lifecycle.rs` и `diagnostics.rs`.

Сюда должны уйти:

- `open_wizard`;
- `wire_wizard_steps`;
- `refill_wizard_summary`;
- wizard-specific checks;
- stealth toggle inside wizard;
- кнопка `Open diagnostics`;
- кнопка `Install local AI`.

Не переносить в этот файл general diagnostics internals. Wizard должен вызывать
публичные helpers из `diagnostics.rs`.

Критерии приемки:

- wizard открывается на первом запуске;
- wizard можно открыть из Settings;
- step checks работают;
- `Open diagnostics` открывает Settings на нужной вкладке и запускает проверку;
- stealth из wizard обновляет все окна через `WindowRegistry`.

### 5.5. `recovery.rs`

Сейчас в `overlay_host.rs` уже появились recovery-функции.

Сюда должны уйти:

- `RECOVERY_CONTEXT_HEADER`;
- `RECOVERY_CONTEXT_FOOTER`;
- `build_recovery_block`;
- `strip_recovery_block`;
- `compose_recovery_context`;
- `seed_recovery_context`;
- `open_recover_offer`;
- recovery tests.

Критерии приемки:

- recovery block не дублируется;
- существующий context не затирается;
- recovery offer уважает stealth/theme/window lifecycle;
- tests остаются зелеными.

### 5.6. `vision_capture.rs`

Сюда должны уйти:

- `fire_f8_vision_capture`;
- `launch_vision_for_bgra`;
- route-specific Vision helpers;
- screenshot/crop bridge;
- future prompt presets;
- future translate mode.

Не переносить низкоуровневый BGRA capture из `src/capture.rs`: он уже отдельный.

Критерии приемки:

- F8 region capture работает;
- bar capture chip запускает тот же flow;
- Vision tile streaming работает;
- stealth capture overlay не мигает;
- mixed-DPI smoke остается отдельной ручной проверкой.

### 5.7. `local_ai_ui.rs`

Сюда должны уйти только UI wiring и Slint callbacks, не backend installer.

Перенести:

- Settings callbacks для local AI install;
- progress updates;
- cancel;
- apply install result to config/UI;
- warmup UI messages.

Не переносить:

- `overlay-backend/src/local_ai.rs`;
- SHA-256;
- downloads;
- process management.

Критерии приемки:

- Install local AI запускается из Settings;
- progress не замораживает UI;
- cancel работает;
- after install обновляет config и Diagnostics.

### 5.8. `updater_ui.rs`

Сюда должны уйти:

- update check callback;
- download installer callback;
- status messages;
- `run_installer` result handling;
- app quit after successful spawn.

Backend `overlay-backend/src/update.rs` остается отдельно.

Критерии приемки:

- check update работает;
- download failure остается в UI;
- если installer не spawn-нулся, приложение не закрывается;
- если spawn успешен, приложение закрывается как раньше.

### 5.9. `settings_controller.rs`

Переносить не первым. Это большой и рискованный кусок.

Сюда уйдут:

- `open_settings`;
- `fetch_models`;
- `ModelTarget`;
- `active_stack_label`;
- `refresh_profiles`;
- `populate_token_status`;
- `apply_server_preview`;
- import/export config handlers;
- language/theme/style callbacks;
- AI/STT/Vision settings callbacks.

Лучше делить Settings дополнительно на подмодули:

```text
settings_controller.rs
settings_ai.rs
settings_stt.rs
settings_vision.rs
settings_profiles.rs
settings_import_export.rs
settings_diagnostics_tab.rs
```

Но сначала можно вынести все Settings в один файл, если механический diff
остается понятным.

Критерии приемки:

- Settings открывается без flash;
- все вкладки сохраняют настройки;
- import/export/server-preview работает;
- theme/language меняются как раньше;
- Diagnostics tab работает через `diagnostics.rs`.

### 5.10. `tile_controller.rs`

Переносить после window lifecycle.

Сюда могут уйти:

- `wire_tile_drag`;
- `present_tile_window`;
- `apply_tile_hwnd_with_monitor`;
- `toggle_tile_maximize`;
- `wire_copy`;
- `wire_voice_followup`;
- `wire_escalate`;
- conversation copy helpers;
- per-tile conversation map helpers;
- tile streaming install helpers.

Не делать это первым: tile flow связан с bridge/events/AI streaming и легко
зацепить поведение.

## 6. Предлагаемый порядок работ

### Phase 0 — freeze and baseline

Перед началом:

1. Дождаться чистого или понятного рабочего дерева.
2. Зафиксировать текущий diff, если он принадлежит другому агенту.
3. Прогнать:

```text
cargo fmt --manifest-path slint-experiment/Cargo.toml --check
cargo clippy --manifest-path slint-experiment/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path slint-experiment/Cargo.toml
```

4. Сделать ручной smoke:

```text
F1 Help
F4 Palette
F6 manual tile
F8 capture
F9 ask
Shift+F9 cloud ask
Settings open/close
Wizard open
Stealth preview
```

### Phase 1 — window lifecycle and registry

Создать:

```text
src/bin/overlay_host/window_lifecycle.rs
```

Перенести:

- global stealth;
- global scheme;
- global tile opacity;
- `present_window_stealth_aware`;
- `apply_scheme_*`;
- registry открытых окон;
- `apply_stealth`.

Цель: исправить класс ошибок, где новое окно не попало в stealth/theme sync.

После Phase 1 обязательно проверить:

- открыть Help при stealth off;
- включить stealth из bar;
- проверить, что Help скрыт из capture;
- повторить из Settings;
- повторить из Wizard.

### Phase 2 — diagnostics

Создать:

```text
src/bin/overlay_host/diagnostics.rs
```

Перенести:

- diagnostics properties population;
- live-check callbacks;
- copy report;
- redaction;
- tests.

После Phase 2 проверить:

- `Check all`;
- Vision row;
- hotkeys row;
- `Copy report`;
- redaction tests.

### Phase 3 — hotkeys

Создать:

```text
src/bin/overlay_host/hotkeys.rs
```

Начать с регистрации и diagnostics-result. Dispatch можно оставить в `main`,
если иначе diff слишком большой.

После Phase 3 проверить все hotkeys.

### Phase 4 — wizard and recovery

Создать:

```text
src/bin/overlay_host/wizard.rs
src/bin/overlay_host/recovery.rs
```

Перенести wizard и recovery без изменения UI.

После Phase 4 проверить:

- first-run wizard;
- manual wizard open from Settings;
- recovery offer;
- stealth from wizard;
- open diagnostics from wizard.

### Phase 5 — Vision capture

Создать:

```text
src/bin/overlay_host/vision_capture.rs
```

Перенести F8/capture flow.

После Phase 5 проверить:

- F8 region selection;
- capture chip;
- Vision tile;
- cloud/local Vision route;
- screen-share stealth.

### Phase 6 — updater and local AI UI

Создать:

```text
src/bin/overlay_host/updater_ui.rs
src/bin/overlay_host/local_ai_ui.rs
```

Перенести только UI callbacks.

Backend остается в `overlay-backend`.

### Phase 7 — Settings and tiles

Создать:

```text
src/bin/overlay_host/settings_controller.rs
src/bin/overlay_host/tile_controller.rs
```

Это самые большие переносы. Делать только после стабилизации lifecycle,
diagnostics и hotkeys.

## 7. Как избегать большого `super::*`

Для первого механического переноса допустимо временно использовать:

```rust
use super::*;
```

Но после каждого этапа лучше сужать imports.

Целевой стиль:

```rust
use crate::ui::{SettingsWindow, TileWindow};
use slint::{ComponentHandle, SharedString};
use slint_replay::win32::{grab_hwnd, set_stealth};
```

Правило: если модуль требует слишком много случайных imports, значит граница
модуля выбрана слишком широко.

## 8. Какие типы сделать `pub(crate)`

Часть типов сейчас локальна в `overlay_host.rs`. При переносе их придется
открыть внутри crate root бинаря:

- `OverlayBarBridge`;
- `StreamingTile`;
- `AskRoute`;
- `LiveRoute`;
- `TileWindows`;
- `HotkeyDiag`;
- `ModelTarget`;
- helper callbacks для Settings/Wizard.

Использовать `pub(crate)`, не `pub`, чтобы не превращать внутренности бинаря в
публичный API.

## 9. Контракты, которые нельзя сломать

### Stealth

- новое окно не должно появляться в capture до применения WDA;
- открытое окно должно получать переключение stealth сразу;
- bar, tiles, Settings, palette, text ask, wizard, Help должны обновляться через
  один общий путь;
- capture overlay остается pre-stealthed special case.

### Theme

- новое окно получает текущий `Theme.scheme`;
- изменение темы обновляет открытые окна;
- будущие окна наследуют текущую тему.

### DPI and placement

- Slint отвечает за естественный размер окна;
- Win32 helper только размещает окно;
- tile default size должен оставаться синхронизированным с Slint preferred size;
- mixed-DPI проверяется вручную.

### UI thread

- Slint properties обновляются только из event loop;
- тяжелые AI/STT/audio/network операции остаются off-thread;
- callbacks не держат config lock во время долгой операции.

### Secrets

- Diagnostics report не содержит bearer/API keys/transcript/profile/screenshot;
- parse/import errors не должны echo-ить токены;
- local paths и hostnames в report требуют redaction policy.

## 10. Минимальные проверки после каждого этапа

Команды:

```text
cargo fmt --manifest-path slint-experiment/Cargo.toml --check
cargo clippy --manifest-path slint-experiment/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path slint-experiment/Cargo.toml
```

Ручной smoke:

```text
open/close Settings
open/close Help
open/close Palette
open Text Ask
open Wizard
toggle stealth from bar
toggle stealth from Settings
toggle stealth from Wizard
spawn tile
drag tile
copy tile answer
F8 capture
Diagnostics Check all
Copy diagnostics report
```

Release-level smoke:

```text
screen-share preview with stealth on
F1/F3/F4/F6/F8/F9/Shift+F9
two monitors
mixed DPI
local AI install/cancel/retry
updater spawn failure
server-only import
recovery offer after unfinished session
```

## 11. Definition of done

Разбиение можно считать успешным, если:

- `overlay_host.rs` стал composition root, а не складом всей логики;
- нет поведения, которое надо менять в трех местах при добавлении нового окна;
- window lifecycle централизован;
- diagnostics изолированы;
- hotkey registration изолирован;
- wizard/recovery вынесены;
- Vision flow вынесен;
- Settings и tiles хотя бы отделены или имеют понятный следующий план;
- все тесты и smoke-проверки проходят;
- новые модули не имеют циклической каши из `Rc<RefCell<...>>`, переданной без
  структуры.

## 12. Рекомендуемый первый PR / commit

Самый полезный первый commit:

```text
refactor(overlay-host): introduce window lifecycle registry
```

Содержимое:

- создать `src/bin/overlay_host/window_lifecycle.rs`;
- вынести global stealth/theme helpers;
- ввести `WindowRegistry`;
- заменить ручные stealth loops на `registry.apply_stealth(on)`;
- подключить Help к общему stealth path;
- поведение остальных окон не менять.

Почему именно это первым:

- это решает реальный класс багов;
- это задает архитектурную форму для остальных модулей;
- это уменьшает риск при любом следующем новом окне;
- это можно проверить вручную без глубокого изменения AI/audio потоков.

## 13. Что не делать в первом проходе

- Не переносить Slint generated UI в `src/lib.rs`.
- Не переписывать Settings UI.
- Не менять hotkey mapping.
- Не добавлять новые фичи.
- Не менять storage/session memory.
- Не трогать backend API без необходимости.
- Не делать один огромный `host_context.rs` на все приложение.

## 14. Итог

Правильная цель не в том, чтобы просто получить много файлов. Цель в том, чтобы
у приложения появились явные контуры:

- lifecycle окон;
- диагностика;
- hotkeys;
- wizard/recovery;
- Vision capture;
- installer/update UI;
- Settings;
- tiles.

Тогда новые функции можно будет добавлять модульно, а не протаскивать через
десяток разрозненных callback-блоков в одном почти десятысячестрочном файле.
