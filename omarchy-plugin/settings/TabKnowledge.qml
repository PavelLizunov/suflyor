pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import "controls" as C

// SettingsWindow tab 9: the search callback opens the host's knowledge window.
Column {
  id: root
  property var win: null
  property var service: null
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(12)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Entries: {}. Glossary, commands and patterns ship with the app.", [root.win ? root.win.prop("kb-entry-count", 0) : 0]) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Open search (F4)", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "kb-open-clicked", [])
    }
  }
}
