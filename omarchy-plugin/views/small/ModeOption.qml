pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons
import "../../tile" as Tile

Rectangle {
  id: root
  property string title: ""
  property string description: ""
  property bool selected: false
  signal chosen()
  implicitHeight: content.implicitHeight + Commons.Style.spacing.lg * 2
  color: selected ? Commons.Style.selectedAccentFill : Commons.Style.normalFill
  border.color: selected || activeFocus ? Commons.Color.accent : Commons.Style.normalBorderColor
  radius: Commons.Style.cornerRadius
  activeFocusOnTab: true
  Accessible.role: Accessible.RadioButton
  Accessible.name: title
  Accessible.description: description
  Accessible.checked: selected
  Keys.onReturnPressed: chosen()
  Keys.onEnterPressed: chosen()
  Keys.onSpacePressed: chosen()
  Behavior on color { ColorAnimation { duration: Commons.Style.duration(120) } }
  Behavior on border.color { ColorAnimation { duration: Commons.Style.duration(120) } }
  ColumnLayout {
    id: content
    anchors.fill: parent
    anchors.margins: Commons.Style.spacing.lg
    spacing: Commons.Style.spacing.xs
    Tile.Label {
      Layout.fillWidth: true
      text: root.title
      font.bold: true
      color: root.selected ? Commons.Color.accent : Commons.Color.foreground
    }
    Tile.Label {
      Layout.fillWidth: true
      text: root.description
      font.pixelSize: Commons.Style.font.bodySmall
    }
  }
  MouseArea {
    anchors.fill: parent
    cursorShape: Qt.PointingHandCursor
    onClicked: { root.forceActiveFocus(); root.chosen() }
  }
}
