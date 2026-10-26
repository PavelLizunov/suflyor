import QtQuick
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui
import "icons" as Icons

// Suflyor in the Omarchy bar: the session state of suflyord's OverlayBarWindow.
// Left click opens the panel, right click starts or stops the session, middle click asks.
Ui.BarWidget {
  id: root

  moduleName: "io.github.pavellizunov.suflyor"

  property var shell: null
  readonly property var effectiveShell: shell ? shell : (bar && bar.shell ? bar.shell : null)
  property var service: null
  readonly property var effectiveService: {
    if (service) return service
    var sh = effectiveShell
    return sh && typeof sh.serviceFor === "function" ? sh.serviceFor(moduleName) : null
  }

  readonly property var barWin: effectiveService ? effectiveService.client.first("OverlayBarWindow") : null
  function p(name, fallback) { return root.barWin ? root.barWin.prop(name, fallback) : fallback }
  function tr(msgid, args) { return root.effectiveService ? root.effectiveService.tr(msgid, args) : msgid }

  readonly property bool running: !!effectiveService && effectiveService.running && !!barWin
  readonly property bool sessionActive: running && p("timer-active", false)
  readonly property bool recording: running && (p("mic-recording", false) || p("sys-recording", false))
  readonly property string tone: running ? p("status-tone", "neutral") : "neutral"

  readonly property color ink: !running ? Commons.Color.muted
    : tone === "danger" ? Commons.Color.urgent
    : sessionActive || recording ? Commons.Color.accent
    : Commons.Color.bar.text

  function localizedStatus(status) {
    // Source toggles can refresh the host status while no session is running.
    if (!root.recording && (status === "recording" || status === "mic only")) {
      if (!root.sessionActive) status = "idle"
      else if (root.p("session-paused", false)) status = "paused"
    }
    switch (status) {
    case "idle": return tr("idle")
    case "recording": return tr("recording")
    case "mic only": return tr("mic only")
    case "mic muted": return tr("mic muted")
    case "paused": return tr("paused")
    case "wrapping up": return tr("wrapping up")
    case "start failed": return tr("start failed")
    case "sys ok": return tr("sys ok")
    case "sys quiet": return tr("sys quiet")
    case "sys silent": return tr("sys silent")
    case "sys test failed": return tr("sys test failed")
    case "AI unavailable": return tr("AI unavailable")
    default: return status
    }
  }

  readonly property string tooltip: {
    if (!effectiveService) return "Suflyor"
    if (effectiveService.daemonFailed) return root.tr("Suflyor could not start. Reinstall the plugin, then restart the shell.")
    if (!running) return root.tr("Suflyor is starting…")
    var lines = [sessionActive && p("session-name", "") ? p("session-name", "") : "Suflyor"]
    if (sessionActive) lines.push(root.tr("Stop session") + " · " + p("timer-label", ""))
    var status = p("stealth-fault", false) ? root.tr("[!] stealth unavailable") : localizedStatus(p("status-text", ""))
    if (status) lines.push(status)
    return lines.join("\n")
  }

  function fire(name) {
    if (!root.running) return
    if (name === "text-ask-clicked") root.close()
    root.effectiveService.client.call(root.barWin.winId, name, [])
  }

  readonly property var panelItem: panelLoader.item
  readonly property bool opened: panelItem ? panelItem.opened : false
  readonly property bool popoutSwitchClosing: panelItem ? panelItem.popoutSwitchClosing : false

  function syncPanel() {
    if (!panelLoader.item) return
    panelLoader.item.bar = root.bar
    panelLoader.item.settings = root.settings
    panelLoader.item.hostWidget = root
    panelLoader.item.anchorItem = button
    panelLoader.item.service = root.effectiveService
  }

  function open() {
    panelLoader.active = true
    Qt.callLater(function () {
      root.syncPanel()
      if (root.panelItem) root.panelItem.open()
    })
  }

  function close() {
    if (root.panelItem) root.panelItem.close()
  }

  function toggle() {
    if (opened) close()
    else open()
  }

  function closeForPopoutSwitch() {
    if (root.panelItem) root.panelItem.closeForPopoutSwitch()
  }

  onBarChanged: syncPanel()
  onSettingsChanged: syncPanel()
  onEffectiveServiceChanged: syncPanel()

  Loader {
    id: panelLoader
    active: false
    visible: false
    source: Qt.resolvedUrl("BarPanel.qml")
    onLoaded: root.syncPanel()
  }

  // Native capture concealment applies to the bar as well as the window layers.
  visible: !(effectiveService && effectiveService.client.concealed)
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  Ui.WidgetButton {
    id: button
    bar: root.bar
    hasVisualContent: true
    labelVisible: false
    fixedWidth: root.vertical ? -1 : row.implicitWidth
    fixedHeight: root.vertical ? (row.implicitHeight + Style.space(12)) : -1
    tooltipText: root.tooltip

    Accessible.role: Accessible.Button
    Accessible.name: root.tooltip
    Accessible.onPressAction: root.toggle()

    onPressed: function (buttonCode) {
      if (buttonCode === Qt.RightButton) { if (root.running) root.fire("timer-toggle-clicked") }
      else if (buttonCode === Qt.MiddleButton) { if (root.running) root.fire("text-ask-clicked") }
      else root.toggle()
    }

    Row {
      id: row
      anchors.centerIn: parent
      spacing: Style.space(5)

      Ui.BarIconButton {
        bar: root.bar
        iconComponent: Component {
          Icons.SuflyorMark { ink: root.ink; assistantActive: root.p("ai-streaming", false) }
        }
        foreground: root.ink
        interactive: false
        pressable: false
        anchors.verticalCenter: parent.verticalCenter
      }

      Text {
        visible: !root.vertical && root.sessionActive
        textFormat: Text.PlainText
        text: root.p("timer-label", "")
        color: root.ink
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        font.features: { "tnum": 1 }
        anchors.verticalCenter: parent.verticalCenter
      }
    }
  }
}
