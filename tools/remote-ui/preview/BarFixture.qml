import QtQuick
import qs.Commons as Commons
import "../../../omarchy-plugin"

// Offscreen preview of the bar panel: the real BarPanel content on a canvas, fed with
// suflyord's own bar snapshot from the headless smoke (idle) or a running session.
Item {
  id: root
  width: 440
  height: 660
  property string locale: "ru"
  property string scene: "idle"
  // Ready once the data is in and switch/hover animations have finished.
  property bool ready: settled.fired
  Timer { id: settled; property bool fired: false; interval: 700; running: service.catalog["Settings"] !== undefined || locale !== "ru"; onTriggered: fired = true }

  readonly property var idle: ({
    "active-stack": "STT: Groq · AI: haiku", "lock-a11y": "Блокировка тайлов — режим прослушивания (без всплывающих окон)",
    "mic-active": true, "status-text": "Linux: экспериментальный порт", "status-tone": "warning", "timer-label": "00:00",
  })
  readonly property var running: Object.assign({}, idle, {
    "timer-active": true, "timer-label": "12:47", "session-name": root.locale === "ru" ? "Собеседование, бэкенд" : "Backend interview",
    "mic-recording": true, "sys-active": true, "stealth-active": true, "open-tiles": 2, "can-restore-tile": true,
    "status-text": "recording", "status-tone": "ok", "ai-streaming": true, "tok-per-sec": "42 tok/s",
    "last-transcript-line": root.locale === "ru" ? "А как вы бы обеспечили идемпотентность платежей?" : "How would you make payments idempotent?",
    "last-transcript-source": "sys",
  })

  FakeService { id: service; locale: root.locale; windows: ({ "OverlayBarWindow": barWin }) }

  RemoteWindow {
    id: barWin
    winId: "OverlayBarWindow-1"
    type: "OverlayBarWindow"
    visible: true
    Component.onCompleted: replace(root.scene === "running" ? root.running : root.idle, true, null)
  }

  Rectangle { anchors.fill: parent; color: Commons.Color.popups.background }
  Item { id: canvas; anchors.fill: parent; anchors.margins: 12 }

  BarPanel { id: panel; service: service; visible: false }
  Component.onCompleted: panel.contentRoot.parent = canvas
}
