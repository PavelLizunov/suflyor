import QtQuick
import qs.Commons
import qs.Commons as Commons

// Properties: icon (Nerd Font glyph), tint, glyphSize, slotWidth, slotHeight. No signals.
Item {
  id: root
  property string icon: ""
  property color tint: Commons.Color.muted
  property real glyphSize: Style.font.icon
  property real slotWidth: Style.space(20)
  property real slotHeight: Style.spacing.controlHeight
  implicitWidth: slotWidth
  implicitHeight: slotHeight
  width: implicitWidth
  height: implicitHeight
  Text {
    anchors.centerIn: parent
    textFormat: Text.PlainText
    text: root.icon
    color: root.tint
    font.family: Style.font.family
    font.pixelSize: root.glyphSize
  }
}
