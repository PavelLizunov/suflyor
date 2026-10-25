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
  visible: win.visible && !parked && !(service && service.client.concealed)
  color: "transparent"
  screen: Quickshell.screens.length ? Quickshell.screens[0] : null

  WlrLayershell.namespace: "suflyor-settings"
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
  exclusionMode: ExclusionMode.Ignore

  implicitWidth: Style.space(900)
  implicitHeight: Style.space(640)

  SettingsView {
    anchors.fill: parent
    win: surface.win
    service: surface.service
    focus: true
    Keys.onEscapePressed: surface.service.client.call(surface.win.winId, "close-clicked", [])
  }

  Component.onCompleted: service.client.reportGeometry(win.winId, { width: implicitWidth, height: implicitHeight })
}
