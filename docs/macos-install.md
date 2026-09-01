# Installing Suflyor on macOS / Установка Suflyor на macOS

Suflyor currently supports Apple Silicon and macOS 14.2 or newer. The
app is locally signed and is not notarized by Apple, so the first
launch requires an explicit Gatekeeper confirmation.

## Русский

### 1. Скачать и перенести приложение

1. Скачайте `Suflyor-<version>-macos-arm64.dmg` только со страницы
   [GitHub Releases](https://github.com/PavelLizunov/suflyor/releases).
2. Откройте DMG и перетащите **Suflyor** на ярлык **Applications**.
3. Дождитесь окончания копирования, затем извлеките DMG.

Исполняемый файл уже имеет правильные права. Команды `chmod`, отключение
Gatekeeper и удаление quarantine-атрибутов в Terminal не требуются.

### 2. Разрешить первый запуск

1. Откройте Finder → **Applications** и один раз запустите **Suflyor**.
2. Если macOS заблокирует приложение, закройте предупреждение.
3. Откройте **System Settings → Privacy & Security** и прокрутите до раздела
   **Security**.
4. Нажмите **Open Anyway** рядом с сообщением о Suflyor, подтвердите пароль или
   Touch ID, затем ещё раз нажмите **Open**.

Кнопка **Open Anyway** появляется только после неудачной попытки запуска и
доступна примерно час. Это штатный способ Apple для приложения от неизвестного
разработчика: [инструкция Apple](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac).

Suflyor не показывает значок в Dock. После запуска ищите панель приложения и
пункт **Suflyor** в строке меню macOS. Через него доступны **Show Suflyor**,
**Hide Suflyor** и **Quit Suflyor**.

### 3. Выдать разрешения на запись

При первом использовании соответствующей функции разрешите доступ в системном
диалоге. На обычном Mac эти TCC-разрешения нельзя выдать приложению одной
командой Terminal: каждое из них подтверждает пользователь. Проверить разрешения
можно вручную:

- **Privacy & Security → Microphone → Suflyor** — микрофон;
- **Privacy & Security → Screen & System Audio Recording → Suflyor** — системный
  звук, снимки экрана и Vision/OCR;
- **Privacy & Security → Accessibility → Suflyor** — требуется для действий с
  выделенным текстом, которые отправляют Command+C.

Если Suflyor ещё нет в списке микрофона или записи экрана, запустите функцию,
которой требуется разрешение: начните тестовую сессию для аудио либо вызовите
захват экрана. В разделе Accessibility нажмите **+** и добавьте
`/Applications/Suflyor.app` вручную. После изменения разрешения выберите
**Quit Suflyor** в строке меню и запустите приложение снова.

### 4. Проверить установку

1. Убедитесь, что панель Suflyor видна; при необходимости выберите
   **Suflyor → Show Suflyor** в строке меню.
2. Начните короткую тестовую сессию.
3. По очереди включите микрофон и системный звук и убедитесь, что оба источника
   дают непустую расшифровку.
4. Только после этого используйте приложение на реальной встрече.

При первом переходе с ad-hoc сборки на стабильную локальную подпись macOS
попросит выдать разрешения ещё один раз. Следующие сборки, подписанные той же
identity и установленные поверх `/Applications/Suflyor.app`, должны сохранять
code identity; смена подписи или возврат к ad-hoc снова потребует проверки
разрешений.

## English

### Install and approve the first launch

1. Download `Suflyor-<version>-macos-arm64.dmg` from
   [GitHub Releases](https://github.com/PavelLizunov/suflyor/releases).
2. Open the DMG, drag **Suflyor** onto **Applications**, wait for the copy to
   finish, and eject the image.
3. Open Finder → **Applications** and try to launch **Suflyor** once.
4. If macOS blocks it, open **System Settings → Privacy & Security**, scroll to
   **Security**, choose **Open Anyway**, authenticate, and confirm **Open**.

No `chmod`, Gatekeeper disablement, or Terminal quarantine workaround is
needed. Apple documents the supported override flow
[here](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac).
The app intentionally has no Dock icon; use the **Suflyor** menu-bar item to
show, hide, or quit the overlay.

### Grant capture permissions

On an unmanaged Mac, no Terminal command can grant these TCC permissions to an
app; the user must approve each one. Enable Suflyor under:

- **Privacy & Security → Microphone** for microphone capture;
- **Privacy & Security → Screen & System Audio Recording** for system audio,
  screenshots, and Vision/OCR;
- **Privacy & Security → Accessibility** for selected-text actions that send
  Command+C.

Trigger the related capture feature first if Suflyor is not yet listed under
Microphone or Screen & System Audio Recording. Under Accessibility, use **+** to
add `/Applications/Suflyor.app` manually. After changing a permission, quit
Suflyor from its menu-bar item and reopen it. Run a short test session and
confirm non-empty transcription from both microphone and system audio before
relying on it in a meeting.

The first switch from an ad-hoc build to a stable local signing identity needs
one new permission approval cycle. Later builds signed by that same identity and
installed over `/Applications/Suflyor.app` should keep the same code identity;
changing the identity or returning to ad-hoc signing requires another permission
check.
