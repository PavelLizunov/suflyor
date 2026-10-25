pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons
import "../../tile" as Tile

Rectangle {
  id: root
  property string glyph: ""
  property string description: ""
  property bool header: false
  property bool keyRow: false
  implicitHeight: Math.max(Commons.Style.space(header ? 28 : 42), cells.implicitHeight + Commons.Style.spacing.sm * 2)
  color: header ? Commons.Style.normalFill : "transparent"
  border.color: Commons.Style.normalBorderColor
  RowLayout {
    id: cells
    anchors.fill: parent
    anchors.margins: Commons.Style.spacing.sm
    spacing: Commons.Style.spacing.lg
    Tile.Label {
      Layout.preferredWidth: Commons.Style.space(124)
      Layout.minimumWidth: 0
      text: root.glyph
      color: root.keyRow ? Commons.Color.accent : root.header ? Commons.Color.muted : Commons.Color.foreground
      font.pixelSize: root.header || root.keyRow ? Commons.Style.font.caption : Commons.Style.font.body
      font.bold: true
      horizontalAlignment: Text.AlignHCenter
    }
    Rectangle { Layout.fillHeight: true; implicitWidth: Commons.Style.space(1); color: Commons.Style.normalBorderColor }
    Tile.Label {
      Layout.fillWidth: true
      Layout.minimumWidth: 0
      text: root.description
      font.bold: root.header
      color: root.header ? Commons.Color.muted : Commons.Color.foreground
      font.pixelSize: root.header ? Commons.Style.font.caption : Commons.Style.font.body
    }
  }
}
