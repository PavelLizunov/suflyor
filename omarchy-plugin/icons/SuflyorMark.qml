import QtQuick
import qs.Commons as Commons

// The original head becomes the penguin while AI is streaming, as in Slint.
ProjectIcon {
  property color ink: Commons.Color.foreground
  property bool assistantActive: false
  name: assistantActive ? "brand-penguin" : "brand-head"
  color: ink
  width: Commons.Style.bar.iconCanvas
  height: width
}
