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
  readonly property bool shown: win.visible && !parked && !!service && service.captureReady && !(service.client.concealed || service.client.nativeDialogOpen)
  readonly property bool placed: typeof g.x === "number" && typeof g.y === "number" && !parked
  // suflyord places tiles in Hyprland's global layout coordinates: the tile goes on the
  // screen that holds its centre, its margins are relative to that screen.
  readonly property var placedScreen: placed ? screenAt(g.x + tileWidth / 2, g.y + tileHeight / 2) : null
  property var targetScreen: null
  visible: shown
  color: "transparent"
  screen: placedScreen || targetScreen
  onShownChanged: if (shown) targetScreen = service.focusedScreen()

  function screenAt(x, y) {
    for (var i = 0; i < Quickshell.screens.length; i++) {
      var s = Quickshell.screens[i]
      if (x >= s.x && x < s.x + s.width && y >= s.y && y < s.y + s.height) return s
    }
    return null
  }

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
  readonly property int originX: surface.screen ? surface.screen.x : 0
  readonly property int originY: surface.screen ? surface.screen.y : 0
  margins.left: placed ? g.x - originX : Style.space(80) + fallbackOffset
  margins.top: placed ? g.y - originY : Style.space(80) + fallbackOffset
  implicitWidth: tileWidth
  implicitHeight: tileHeight

  TileView {
    anchors.fill: parent
    win: surface.win
    service: surface.service
    onMoveBy: function (dx, dy) {
      var x = Math.round(surface.originX + surface.margins.left + dx)
      var y = Math.round(surface.originY + surface.margins.top + dy)
      surface.service.client.reportGeometry(surface.win.winId, { x: x, y: y, width: surface.tileWidth, height: surface.tileHeight })
      surface.win.geometry = Object.assign({}, surface.g, { x: x, y: y })
    }
  }

  Component.onCompleted: targetScreen = service.focusedScreen()
}
