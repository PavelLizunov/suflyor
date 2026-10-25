pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "../../tile" as Tile

Rectangle {
  id: root
  required property var view
  required property var row
  required property int rowIndex
  readonly property bool renaming: view.p("renaming-index", -1) === rowIndex
  readonly property bool selected: view.p("selected-index", 0) === rowIndex
  implicitHeight: content.implicitHeight + Commons.Style.spacing.lg * 2
  color: selected ? Commons.Style.selectedAccentFill : "transparent"
  border.color: selected ? Commons.Color.accent : Commons.Style.normalBorderColor
  radius: Commons.Style.cornerRadius
  MouseArea {
    anchors.fill: parent
    onClicked: root.view.call("result-activated", [root.rowIndex])
  }
  ColumnLayout {
    id: content
    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Commons.Style.spacing.lg }
    spacing: Commons.Style.spacing.sm
    RowLayout {
      Layout.fillWidth: true
      spacing: Commons.Style.spacing.sm
      Ui.TextField {
        id: rename
        visible: root.renaming
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        text: root.view.p("rename-text", "")
        placeholderText: root.view.service ? root.view.service.tr("Session name", []) : ""
        onTextEdited: root.view.set("rename-text", text)
        onAccepted: root.view.call("rename-confirmed", [root.rowIndex])
        onVisibleChanged: if (visible) Qt.callLater(function() { rename.forceActiveFocus() })
      }
      Tile.ActionButton {
        visible: root.renaming
        iconText: "\uf00c"
        label: root.view.service ? root.view.service.tr("Save name", []) : ""
        onClicked: root.view.call("rename-confirmed", [root.rowIndex])
      }
      Tile.ActionButton {
        visible: root.renaming
        iconText: "\uf00d"
        label: root.view.service ? root.view.service.tr("Cancel rename", []) : ""
        onClicked: root.view.call("rename-cancelled")
      }
      Tile.Label {
        visible: !root.renaming
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        text: root.row.title
        font.bold: true
        color: root.selected ? Commons.Color.accent : Commons.Color.foreground
        wrapMode: Text.NoWrap
        elide: Text.ElideRight
      }
      Tile.Label {
        visible: !root.renaming
        Layout.maximumWidth: content.width * 0.3
        text: root.row.meta
        color: Commons.Color.muted
        font.pixelSize: Commons.Style.font.caption
        wrapMode: Text.NoWrap
        elide: Text.ElideRight
      }
    }
    Tile.Label {
      Layout.fillWidth: true
      text: root.row.subtitle
      color: Commons.Color.muted
      font.pixelSize: Commons.Style.font.caption
      wrapMode: Text.NoWrap
      elide: Text.ElideRight
    }
    RowLayout {
      visible: !root.renaming
      Layout.fillWidth: true
      spacing: Commons.Style.spacing.sm
      Item { Layout.fillWidth: true }
      IconAction {
        iconText: "\uf075"
        label: root.view.service ? root.view.service.tr("View transcript", []) : ""
        onClicked: root.view.call("transcript-requested", [root.rowIndex])
      }
      IconAction {
        iconText: "\uf040"
        label: root.view.service ? root.view.service.tr("Rename session", []) : ""
        onClicked: root.view.call("rename-requested", [root.rowIndex, root.row.name])
      }
      IconAction {
        iconText: "\uf0d0"
        label: root.view.service ? root.view.service.tr("Regenerate name from transcript", []) : ""
        onClicked: root.view.call("regen-name-requested", [root.rowIndex])
      }
      // A gap before the destructive action, as in archive.slint.
      Item { implicitWidth: Commons.Style.spacing.sm }
      IconAction {
        iconText: "\uf1f8"
        foreground: Commons.Color.urgent
        label: root.view.service ? root.view.service.tr("Delete session", []) : ""
        onClicked: root.view.call("delete-requested", [root.rowIndex])
      }
    }
    RowLayout {
      visible: !root.view.p("retranscribe-busy", false) && !!root.row["has-data"]
      Layout.fillWidth: true
      spacing: Commons.Style.spacing.sm
      Action {
        iconText: root.row["has-summary"] ? "\uf15c" : "\uf021"
        text: root.view.service ? root.view.service.tr(root.row["has-summary"] ? "View" : "Generate", []) : ""
        foreground: Commons.Color.accent
        onClicked: root.view.call(root.row["has-summary"] ? "view-summary-requested" : "retranscribe-requested", [root.rowIndex])
      }
      Action {
        visible: !!root.row["has-summary"]
        iconText: "\uf021"
        text: root.view.service ? root.view.service.tr("Recreate", []) : ""
        onClicked: root.view.call("retranscribe-requested", [root.rowIndex])
      }
      Action {
        visible: !!root.row["has-debrief"]
        iconText: "\uf086"
        text: root.view.service ? root.view.service.tr("Coaching", []) : ""
        onClicked: root.view.call("view-debrief-requested", [root.rowIndex])
      }
    }
    Tile.Label {
      visible: !root.view.p("retranscribe-busy", false) && !root.row["has-data"]
      Layout.fillWidth: true
      text: root.view.service ? root.view.service.tr("Not enough data", []) : ""
      horizontalAlignment: Text.AlignHCenter
      color: Commons.Color.muted
    }
  }
}
