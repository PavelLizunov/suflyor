import QtQuick
import "../../../omarchy-plugin"

// Offscreen preview of TileView with a realistic answer. Renders the real view
// over an inert service; nothing reaches suflyord.
Item {
  id: root
  width: 460
  height: 420
  property string locale: "ru"
  property bool ready: service.catalog["Close"] !== undefined || locale !== "ru"

  FakeService { id: service; locale: root.locale }

  // suflyord always sends display-text (the rendered form of text); here they match.
  function blocks() {
    var ru = root.locale === "ru"
    return [
      [2, ru ? "Коротко" : "In short", "", false],
      [0, ru ? "Повторный запрос даёт тот же результат, что и первый." : "Repeating a request gives the same result as the first one.", "", false],
      [4, ru ? "GET, PUT и DELETE идемпотентны" : "GET, PUT and DELETE are idempotent", "", false],
      [4, ru ? "POST — нет: каждый вызов создаёт запись" : "POST is not: each call creates a record", "", true],
      [5, "curl -X PUT /users/42 -d '{\"name\":\"Ann\"}'", "bash", false],
    ].map(function (b) { return { "kind": b[0], "text": b[1], "display-text": b[1], "lang": b[2], "marked": b[3] } })
  }

  RemoteWindow {
    id: win
    winId: "TileWindow-7"
    type: "TileWindow"
    visible: true
    Component.onCompleted: replace({
      "sequence": 3,
      "tile-title": root.locale === "ru" ? "Что такое идемпотентность?" : "What is idempotency?",
      "question-text": root.locale === "ru" ? "Что такое идемпотентность в REST?" : "What is idempotency in REST?",
      "trigger-label": "F9",
      "trigger-tone": "accent",
      "source-label": "AI · haiku",
      "convo-id": 4,
      "can-copy": true, "can-regenerate": true, "can-speak": true, "can-escalate": true,
      "body-opacity": 1.0,
      "answer-terms": [{ "items": ["PUT", "DELETE", "POST"] }],
      "blocks": root.blocks(),
    }, true, null)
  }

  TileView {
    anchors.fill: parent
    win: win
    service: service
  }
}
