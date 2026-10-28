pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons

// Properties: progress (0..1). A passive, full-width download indicator; no signals.
Rectangle {
  id: root
  property real progress: 0
  width: parent ? parent.width : implicitWidth
  implicitHeight: Style.space(6)
  height: implicitHeight
  radius: height / 2
  color: Qt.alpha(Commons.Color.foreground, 0.16)
  Rectangle {
    width: parent.width * Math.max(0, Math.min(1, root.progress))
    height: parent.height
    radius: parent.radius
    color: Commons.Color.accent
  }
}
