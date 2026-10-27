import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

// The shell surface of one of suflyord's secondary windows (archive, transcript,
// palette, help, wizard, recovery offer, text ask): a centred layer in the
// `suflyor-<type>` namespace that loads aux/<Type>View.qml. Escape is the view's to handle.
PanelWindow {
  id: surface

  required property var modelData
  readonly property var win: modelData
  property var service: null

  // Logical size per window, close to the Slint windows' preferred sizes.
  readonly property var sizes: ({
    "ArchiveWindow": [860, 600], "TranscriptWindow": [900, 640], "PaletteWindow": [640, 460],
    "HelpWindow": [680, 600], "WizardWindow": [760, 560], "RecoverOfferWindow": [460, 270],
    "TextAskWindow": [560, 150],
  })
  readonly property var size: sizes[win.type] || [600, 400]
  readonly property string viewName: win.type.replace(/Window$/, "") + "View"

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

  WlrLayershell.namespace: "suflyor-" + viewName.toLowerCase()
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
  exclusionMode: ExclusionMode.Ignore

  implicitWidth: Math.min(Style.space(size[0]), screen ? Math.max(1, screen.width - Style.gapsOut * 2) : Style.space(size[0]))
  implicitHeight: Math.min(Style.space(size[1]), screen ? Math.max(1, screen.height - Style.space(60)) : Style.space(size[1]))

  Loader {
    anchors.fill: parent
    source: Qt.resolvedUrl("aux/" + surface.viewName + ".qml")
    onLoaded: {
      item.win = Qt.binding(function () { return surface.win })
      item.service = Qt.binding(function () { return surface.service })
    }
  }

  Component.onCompleted: {
    targetScreen = service.focusedScreen()
    service.client.reportGeometry(win.winId, { width: implicitWidth, height: implicitHeight })
  }
}
