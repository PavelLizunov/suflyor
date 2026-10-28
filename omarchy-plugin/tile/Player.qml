pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons

Rectangle {
  id: root
  required property var view
  implicitHeight: content.implicitHeight + Commons.Style.spacing.lg * 2
  color: Commons.Color.menu.background
  border.color: Commons.Style.normalBorderColor
  radius: Commons.Style.cornerRadius
  ColumnLayout {
    id: content
    anchors.fill: parent
    anchors.margins: Commons.Style.spacing.lg
    spacing: Commons.Style.spacing.sm
    Label {
      Layout.fillWidth: true
      text: root.view.service.tr(root.view.p("speak-loading", false) ? "Preparing voice..." : "Reading aloud", [])
      color: root.view.p("speak-loading", false) ? Commons.Color.accent : Commons.Color.muted
      font.pixelSize: Commons.Style.font.caption
      elide: Text.ElideRight
      wrapMode: Text.NoWrap
    }
    RowLayout {
      Layout.fillWidth: true
      spacing: Commons.Style.spacing.sm
      ActionButton {
        text: "-10"
        label: root.view.service.tr("Back 10 seconds", [])
        onClicked: root.view.call("speak-seek-clicked", [-10])
      }
      ActionButton {
        iconText: root.view.p("speak-paused", false) ? "\uf04b" : "\uf04c"
        selected: true
        label: root.view.service.tr("Pause / resume reading aloud", [])
        onClicked: root.view.call("speak-pause-clicked")
      }
      ActionButton {
        text: "+15"
        label: root.view.service.tr("Forward 15 seconds", [])
        onClicked: root.view.call("speak-seek-clicked", [15])
      }
      Item { Layout.fillWidth: true }
      ActionButton {
        iconText: "\uf04d"
        foreground: Commons.Color.urgent
        label: root.view.service.tr("Stop reading aloud", [])
        onClicked: root.view.call("speak-clicked")
      }
      ActionButton {
        implicitWidth: Math.max(Commons.Style.space(46), implicitHeight)
        text: root.view.p("speak-speed-label", "1.0x")
        label: root.view.service.tr("Reading speed", []) + ": " + text
        onClicked: root.view.call("speak-speed-next-clicked")
      }
    }
  }
}
