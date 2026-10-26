import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons

// Properties: text, color, font; already-translated plain text, width-aware height.
Text {
  id: root
  readonly property bool layoutManaged: parent instanceof RowLayout || parent instanceof ColumnLayout || parent instanceof GridLayout
  width: layoutManaged ? implicitWidth : parent ? parent.width : implicitWidth
  textFormat: Text.PlainText
  wrapMode: Text.Wrap
  color: Commons.Util.alpha(Commons.Color.foreground, 0.8)
  font.family: Style.font.family
  font.pixelSize: Style.font.bodySmall
}
