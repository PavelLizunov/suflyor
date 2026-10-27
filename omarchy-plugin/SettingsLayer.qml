import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

// Keep the layer stationary: Wayland does not expose a moving layer's global
// origin to Qt. Move only the masked card so pointer deltas remain stable.
PanelWindow {
  id: surface
  required property var modelData
  readonly property var win: modelData
  property var service: null
  property var targetScreen: null
  property real offsetX: 0
  property real offsetY: 0
  readonly property var g: win.geometry || ({})
  readonly property bool parked: typeof g.x === "number" && (g.x < -10000 || g.y < -10000)
  readonly property bool shown: win.visible && !parked && !(service && (service.client.concealed || service.client.nativeDialogOpen))
  readonly property int cardWidth: Math.min(Style.space(960), screen ? Math.max(1, screen.width - Style.gapsOut * 2) : Style.space(960))
  readonly property int cardHeight: Math.min(Style.space(680), screen ? Math.max(1, screen.height - Style.space(60)) : Style.space(680))
  readonly property real centeredX: (width - cardWidth) / 2
  readonly property real centeredY: (height - cardHeight) / 2
  anchors { top: true; bottom: true; left: true; right: true }
  mask: Region { item: card }
  visible: shown
  color: "transparent"
  screen: targetScreen
  onShownChanged: if (shown) targetScreen = service.focusedScreen()
  WlrLayershell.namespace: "suflyor-settings"
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
      var x = Math.max(0, Math.min(surface.width - surface.cardWidth, card.x + dx))
      var y = Math.max(0, Math.min(surface.height - surface.cardHeight, card.y + dy))
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
    x: Math.round(surface.centeredX + surface.offsetX)
    y: Math.round(surface.centeredY + surface.offsetY)
    width: surface.cardWidth
    height: surface.cardHeight
    SettingsView {
      anchors.fill: parent
      win: surface.win
      service: surface.service
      focus: true
      Keys.onEscapePressed: surface.service.client.call(surface.win.winId, "close-clicked", [])
    }
  }
  Component.onCompleted: {
    targetScreen = service.focusedScreen()
    service.client.reportGeometry(win.winId, { width: cardWidth, height: cardHeight })
  }
}
