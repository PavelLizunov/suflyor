import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

// The shell surface of suflyord's SettingsWindow: a centred layer in the
// `suflyor-settings` namespace (hidden from screen share with stealth on),
// keyboard input on demand for its fields. Escape asks suflyord to close it.
PanelWindow {
  id: surface

  required property var modelData
  readonly property var win: modelData
  property var service: null

  readonly property var g: win.geometry || ({})
  readonly property bool parked: typeof g.x === "number" && (g.x < -10000 || g.y < -10000)
  readonly property bool shown: win.visible && !parked && !(service && (service.client.concealed || service.client.nativeDialogOpen))
  property var targetScreen: null
  property real offsetX: 0
  property real offsetY: 0
  anchors { top: true; left: true }
  readonly property real centeredX: screen ? (screen.width - implicitWidth) / 2 : 0
  readonly property real centeredY: screen ? (screen.height - implicitHeight) / 2 : 0
  margins.left: Math.round(centeredX + offsetX)
  margins.top: Math.round(centeredY + offsetY)
  visible: shown
  color: "transparent"
  screen: targetScreen
  onShownChanged: if (shown) targetScreen = service.focusedScreen()

  // Presented again (hotkey, bar panel): come to the monitor in use.
  Connections {
    target: surface.win
    function onFocusRequested() { surface.targetScreen = surface.service.focusedScreen() }
  }

  Connections {
    target: surface.service ? surface.service.client : null
    function onMoveRequested(winId, dx, dy) {
      if (winId !== surface.win.winId || !surface.screen) return
      var x = Math.max(0, Math.min(surface.screen.width - surface.implicitWidth, surface.margins.left + dx))
      var y = Math.max(0, Math.min(surface.screen.height - surface.implicitHeight, surface.margins.top + dy))
      surface.offsetX = x - surface.centeredX
      surface.offsetY = y - surface.centeredY
      surface.service.client.reportGeometry(winId, {
        x: Math.round(surface.screen.x + x), y: Math.round(surface.screen.y + y),
        width: surface.implicitWidth, height: surface.implicitHeight
      })
    }
  }

  Connections {
    target: Quickshell
    function onScreensChanged() {
      if (Quickshell.screens.indexOf(surface.targetScreen) < 0) surface.targetScreen = surface.service.focusedScreen()
    }
  }

  WlrLayershell.namespace: "suflyor-settings"
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
  exclusionMode: ExclusionMode.Ignore

  implicitWidth: Math.min(Style.space(960), screen ? Math.max(1, screen.width - Style.gapsOut * 2) : Style.space(960))
  implicitHeight: Math.min(Style.space(680), screen ? Math.max(1, screen.height - Style.space(60)) : Style.space(680))

  SettingsView {
    anchors.fill: parent
    win: surface.win
    service: surface.service
    focus: true
    Keys.onEscapePressed: surface.service.client.call(surface.win.winId, "close-clicked", [])
  }

  Component.onCompleted: {
    targetScreen = service.focusedScreen()
    service.client.reportGeometry(win.winId, { width: implicitWidth, height: implicitHeight })
  }
}
