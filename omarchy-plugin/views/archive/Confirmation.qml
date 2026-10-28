pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons
import "../../tile" as Tile

Rectangle {
  id: root
  required property var view
  property string title: ""
  property string subject: ""
  property string explanation: ""
  property string acceptText: ""
  property bool destructive: false
  signal accepted()
  signal cancelled()
  color: Commons.Color.menu.scrim
  onVisibleChanged: if (visible) Qt.callLater(function() { cancel.forceActiveFocus() })
  MouseArea { anchors.fill: parent; onClicked: root.cancelled() }
  Rectangle {
    anchors.centerIn: parent
    width: Math.min(Commons.Style.space(420), root.width - Commons.Style.spacing.xxl * 2)
    height: content.implicitHeight + Commons.Style.spacing.xxl * 2
    color: Commons.Color.background
    radius: Commons.Style.cornerRadius
    border.color: Commons.Style.normalBorderColor
    MouseArea { anchors.fill: parent }
    ColumnLayout {
      id: content
      anchors { left: parent.left; right: parent.right; top: parent.top; margins: Commons.Style.spacing.xxl }
      spacing: Commons.Style.spacing.lg
      Tile.Label {
        Layout.fillWidth: true
        text: root.title
        horizontalAlignment: Text.AlignHCenter
        font.bold: true
        font.pixelSize: Commons.Style.font.subtitle
      }
      Tile.Label {
        visible: root.subject !== ""
        Layout.fillWidth: true
        text: root.subject
        color: Commons.Color.muted
        horizontalAlignment: Text.AlignHCenter
      }
      Tile.Label {
        Layout.fillWidth: true
        text: root.explanation
        color: root.destructive ? Commons.Color.urgent : Commons.Color.muted
        horizontalAlignment: Text.AlignHCenter
      }
      RowLayout {
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.sm
        Action {
          id: cancel
          text: root.view.service ? root.view.service.tr("Cancel", []) : ""
          onClicked: root.cancelled()
        }
        Action {
          text: root.acceptText
          iconText: root.destructive ? "\uf1f8" : "\uf021"
          foreground: root.destructive ? Commons.Color.urgent : Commons.Color.accent
          onClicked: root.accepted()
        }
      }
    }
  }
}
