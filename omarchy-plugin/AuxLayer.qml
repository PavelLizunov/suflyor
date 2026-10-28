import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

// A capture-excluded layer must fit its card, not a full-screen input mask.
// WindowDrag asks the host for compositor-space pointer deltas.
PanelWindow {
  id: surface
  required property var modelData
  readonly property var win: modelData
  property var service: null
  readonly property var sizes: ({
    "ArchiveWindow": [860, 600], "TranscriptWindow": [900, 640], "PaletteWindow": [640, 460],
    "HelpWindow": [680, 600], "WizardWindow": [760, 560], "RecoverOfferWindow": [460, 270],
    "TextAskWindow": [560, 150]
  })
  readonly property var size: sizes[win.type] || [600, 400]
  readonly property string viewName: win.type.replace(/Window$/, "") + "View"
  readonly property var g: win.geometry || ({})
  readonly property bool parked: typeof g.x === "number" && (g.x < -10000 || g.y < -10000)
  readonly property bool shown: win.visible && !parked && !!service && service.captureReady && !(service.client.concealed || service.client.nativeDialogOpen)
  property var targetScreen: null
  property real offsetX: 0
  property real offsetY: 0
  readonly property int cardWidth: Math.min(Style.space(size[0]), screen ? Math.max(1, screen.width - Style.gapsOut * 2) : Style.space(size[0]))
  readonly property int cardHeight: Math.min(Style.space(size[1]), screen ? Math.max(1, screen.height - Style.space(60)) : Style.space(size[1]))
  readonly property real centeredX: screen ? (screen.width - cardWidth) / 2 : 0
  readonly property real centeredY: screen ? (screen.height - cardHeight) / 2 : 0
  readonly property real cardX: Math.max(0, Math.min(screen ? screen.width - cardWidth : 0, centeredX + offsetX))
  readonly property real cardY: Math.max(0, Math.min(screen ? screen.height - cardHeight : 0, centeredY + offsetY))
  anchors { top: true; left: true }
  margins.left: Math.round(cardX)
  margins.top: Math.round(cardY)
  implicitWidth: cardWidth
  implicitHeight: cardHeight
  visible: shown
  color: "transparent"
  screen: targetScreen
  onShownChanged: if (shown) targetScreen = service.focusedScreen()
  WlrLayershell.namespace: "suflyor-" + viewName.toLowerCase()
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
  exclusionMode: ExclusionMode.Ignore
  Connections {
    target: surface.win
    function onFocusRequested() { surface.targetScreen = surface.service.focusedScreen() }
  }
  Connections {
    target: surface.service ? surface.service.client : null
    function onMoveRequested(winId, dx, dy) {
      if (winId !== surface.win.winId || !surface.screen) return
      var x = Math.max(0, Math.min(surface.screen.width - surface.cardWidth, surface.cardX + dx))
      var y = Math.max(0, Math.min(surface.screen.height - surface.cardHeight, surface.cardY + dy))
      surface.offsetX = x - surface.centeredX
      surface.offsetY = y - surface.centeredY
      surface.service.client.reportGeometry(winId, {
        x: Math.round(surface.screen.x + x), y: Math.round(surface.screen.y + y),
        width: surface.cardWidth, height: surface.cardHeight
      })
    }
  }
  Connections {
    target: Quickshell
    function onScreensChanged() {
      if (Quickshell.screens.indexOf(surface.targetScreen) < 0) surface.targetScreen = surface.service.focusedScreen()
    }
  }
  Item {
    id: card
    x: 0
    y: 0
    width: surface.cardWidth
    height: surface.cardHeight
    Loader {
      anchors.fill: parent
      source: Qt.resolvedUrl("views/" + surface.viewName + ".qml")
      onLoaded: {
        item.win = Qt.binding(function () { return surface.win })
        item.service = Qt.binding(function () { return surface.service })
      }
    }
  }
  Component.onCompleted: {
    targetScreen = service.focusedScreen()
    service.client.reportGeometry(win.winId, { width: cardWidth, height: cardHeight })
  }
}
