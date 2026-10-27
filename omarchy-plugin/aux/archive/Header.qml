pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons
import "../../tile" as Tile
import "../.." as Plugin

RowLayout {
  id: root
  required property var view
  property string title: ""
  property string note: ""
  property string icon: ""
  spacing: Commons.Style.spacing.lg
  Tile.Label { text: root.icon; color: Commons.Color.accent }
  Tile.Label {
    Layout.fillWidth: true
    Layout.minimumWidth: 0
    text: root.title
    font.bold: true
    font.pixelSize: Commons.Style.font.subtitle
    wrapMode: Text.NoWrap
    elide: Text.ElideRight
    Plugin.WindowDrag { anchors.fill: parent; win: root.view.win; service: root.view.service }
  }
  Tile.Label {
    visible: root.note !== ""
    Layout.maximumWidth: root.width * 0.4
    text: root.note
    color: Commons.Color.muted
    wrapMode: Text.NoWrap
    elide: Text.ElideRight
  }
  Tile.ActionButton {
    iconText: "\uf00d"
    label: root.view.service ? root.view.service.tr("Close", []) : ""
    foreground: hot ? Commons.Color.urgent : Commons.Color.foreground
    onClicked: root.view.call("close-requested")
  }
}
