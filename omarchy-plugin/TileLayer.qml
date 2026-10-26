import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

// The shell surface of one suflyord TileWindow: a layer above windows in the
// `suflyor-tile` namespace, which the stealth layer rule hides from screen share.
// Position and size follow the tile's geometry; dragging writes it back.
PanelWindow {
  id: surface

  required property var modelData
  readonly property var win: modelData
  property var service: null

  readonly property var g: win.geometry || ({})
  readonly property int tileWidth: g.width > 0 ? g.width : 460
  readonly property int tileHeight: g.height > 0 ? g.height : 360
  // Until suflyord places the tile, cascade from the top-left by tile number.
  readonly property int fallbackOffset: Style.space(24) * (win.prop("sequence", 0) % 8)

  // The host parks a window far off screen while it prepares it.
  readonly property bool parked: typeof g.x === "number" && (g.x < -10000 || g.y < -10000)
  readonly property bool shown: win.visible && !parked && !(service && service.client.concealed)
  property var targetScreen: null
  visible: shown
  color: "transparent"
  screen: targetScreen
  onShownChanged: if (shown) targetScreen = service.focusedScreen()

  Connections {
    target: Quickshell
    function onScreensChanged() {
      if (Quickshell.screens.indexOf(surface.targetScreen) < 0) surface.targetScreen = surface.service.focusedScreen()
    }
  }

  WlrLayershell.namespace: "suflyor-tile"
  WlrLayershell.layer: WlrLayer.Overlay
  // Follow-up questions are typed into the tile; focus only when clicked.
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
  exclusionMode: ExclusionMode.Ignore

  anchors { top: true; left: true }
  margins.left: typeof g.x === "number" ? g.x : Style.space(80) + fallbackOffset
  margins.top: typeof g.y === "number" ? g.y : Style.space(80) + fallbackOffset
  implicitWidth: tileWidth
  implicitHeight: tileHeight

  TileView {
    anchors.fill: parent
    win: surface.win
    service: surface.service
    onMoveBy: function (dx, dy) {
      surface.service.client.reportGeometry(surface.win.winId, {
        x: Math.round(surface.margins.left + dx),
        y: Math.round(surface.margins.top + dy),
        width: surface.tileWidth,
        height: surface.tileHeight,
      })
      surface.win.geometry = Object.assign({}, surface.g, { x: Math.round(surface.margins.left + dx), y: Math.round(surface.margins.top + dy) })
    }
  }

  Component.onCompleted: targetScreen = service.focusedScreen()
}
