import QtQuick
import qs.Commons
import qs.Commons as Commons

// Properties: text, color, font; already-translated plain text, width-aware height.
Text {
  width: parent ? parent.width : implicitWidth
  textFormat: Text.PlainText
  wrapMode: Text.Wrap
  color: Commons.Color.muted
  font.family: Style.font.family
  font.pixelSize: Style.font.caption
}
