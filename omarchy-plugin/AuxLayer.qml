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
  visible: win.visible && !parked && !(service && service.client.concealed)
  color: "transparent"
  screen: Quickshell.screens.length ? Quickshell.screens[0] : null

  WlrLayershell.namespace: "suflyor-" + viewName.toLowerCase()
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
  exclusionMode: ExclusionMode.Ignore

  implicitWidth: Style.space(size[0])
  implicitHeight: Style.space(size[1])

  Loader {
    anchors.fill: parent
    source: Qt.resolvedUrl("aux/" + surface.viewName + ".qml")
    onLoaded: {
      item.win = Qt.binding(function () { return surface.win })
      item.service = Qt.binding(function () { return surface.service })
    }
  }

  Component.onCompleted: service.client.reportGeometry(win.winId, { width: implicitWidth, height: implicitHeight })
}
