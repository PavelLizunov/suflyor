import QtQuick
import qs.Commons as Commons
import "../../../omarchy-plugin"

// Offscreen preview of a secondary window (views/<Type>View.qml) with sample state.
// `scene` names the window type without "Window", e.g. "Archive".
Item {
  id: root
  width: 860
  height: 600
  property string locale: "ru"
  property string scene: "Archive"
  // Ready once the catalog is in and animations have finished.
  property bool ready: settled.fired
  Timer { id: settled; property bool fired: false; interval: 700; running: service.catalog["Close"] !== undefined || root.locale !== "ru"; onTriggered: fired = true }

  readonly property bool ru: locale === "ru"
  function samples() {
    var ru = root.ru
    return {
      "Archive": {
        "results": [
          { "id": "s1", "title": ru ? "Собеседование, бэкенд" : "Backend interview", "subtitle": ru ? "сегодня, 14:05 · 47 мин" : "today, 14:05 · 47 min", "meta": "112 реплик", "name": "", "has-data": true, "has-summary": true, "has-recordings": true, "has-debrief": false },
          { "id": "s2", "title": ru ? "Созвон с командой" : "Team call", "subtitle": ru ? "вчера, 11:00 · 32 мин" : "yesterday, 11:00 · 32 min", "meta": "64 реплики", "name": "", "has-data": true, "has-summary": false, "has-recordings": true, "has-debrief": false },
          { "id": "s3", "title": ru ? "Без названия" : "Untitled", "subtitle": ru ? "23 октября · 5 мин" : "Oct 23 · 5 min", "meta": "", "name": "", "has-data": false, "has-summary": false, "has-recordings": false, "has-debrief": false },
        ],
        "selected-index": 0, "confirm-delete-index": -1, "confirm-resummary-index": -1, "renaming-index": -1,
        "summary": ru ? "3 сессии" : "3 sessions",
      },
      "Transcript": {
        "heading": ru ? "Собеседование, бэкенд" : "Backend interview", "has-audio": true, "time-text": "03:12 / 47:05", "progress": 0.07,
        "volume": 0.8, "speed-index": 1, "speaker-count": 2, "has-diarization": true, "can-diarize": true, "active-line": 1, "search-count": 0,
        "speakers": [{ "id": 0, "label": ru ? "Вы" : "You", "color": "#7aa2f7ff" }, { "id": 1, "label": ru ? "Интервьюер" : "Interviewer", "color": "#e0af68ff" }],
        "lines": [
          { "offset-label": "02:58", "speaker": ru ? "Интервьюер" : "Interviewer", "speaker-color": "#e0af68ff", "text": ru ? "Как бы вы обеспечили идемпотентность платежей?" : "How would you make payments idempotent?", "display-text": ru ? "Как бы вы обеспечили идемпотентность платежей?" : "How would you make payments idempotent?", "start-ms": 178000, "checked": false, "marked": false, "matched": false },
          { "offset-label": "03:06", "speaker": ru ? "Вы" : "You", "speaker-color": "#7aa2f7ff", "text": ru ? "Ключ идемпотентности в заголовке и уникальный индекс в базе." : "An idempotency key header and a unique index in the database.", "display-text": ru ? "Ключ идемпотентности в заголовке и уникальный индекс в базе." : "An idempotency key header and a unique index in the database.", "start-ms": 186000, "checked": false, "marked": true, "matched": false },
        ],
      },
      "Palette": {
        "query": "kubectl", "selected-index": 0, "recent-chips": ["kubectl", "SLO"],
        "results": [
          { "key": "kubectl apply -f manifest.yaml", "title": "kubectl apply", "preview": ru ? "декларативное создание или обновление ресурсов" : "declarative create/update", "source": "commands" },
          { "key": "kubectl rollout undo", "title": "kubectl rollout", "preview": ru ? "откат развёртывания" : "roll a deployment back", "source": "commands" },
          { "key": "SLO", "title": "SLO", "preview": ru ? "целевой уровень обслуживания" : "service level objective", "source": "glossary" },
        ],
      },
      "Help": {},
      "Wizard": { "step": 3, "mode": 0, "ai-level": 0, "stt-level": 0, "mic-level": 4, "sys-level": 3,
        "mic-detail": ru ? "Не удалось открыть микрофон. Проверьте, что микрофон подключён и доступ к нему разрешён. Выберите его в разделе Настройки > Аудио > Устройство микрофона, затем вернитесь и нажмите «Записать тест» ещё раз." : "Could not open the microphone. Check that a microphone is connected and access is allowed. Select it in Settings > Audio > Microphone device, then return and press Record test again." },
      "RecoverOffer": { "has-qa": true, "last-question": ru ? "Что такое идемпотентность?" : "What is idempotency?", "last-answer": ru ? "Повторный запрос даёт тот же результат." : "Repeating a request gives the same result.", "transcript-preview": ru ? "…ключ идемпотентности в заголовке…" : "…an idempotency key header…" },
      "TextAsk": { "active-profile": "SRE", "query": ru ? "Чем PUT отличается от PATCH?" : "How is PUT different from PATCH?" },
    }
  }

  FakeService { id: service; locale: root.locale }
  RemoteWindow {
    id: win
    winId: root.scene + "Window-1"
    type: root.scene + "Window"
    visible: true
    Component.onCompleted: replace(root.samples()[root.scene] || {}, true, null)
  }

  Loader {
    anchors.fill: parent
    source: Qt.resolvedUrl("../../../omarchy-plugin/views/" + root.scene + "View.qml")
    onLoaded: { item.win = win; item.service = service }
  }
}
