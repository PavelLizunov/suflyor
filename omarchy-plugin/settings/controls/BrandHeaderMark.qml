import QtQuick
import qs.Commons
import qs.Commons as Commons
import "../../icons" as Icons

// Properties: markSize; shares the bar identity. No signals.
Item {
  property real markSize: Style.space(22)
  implicitWidth: markSize
  implicitHeight: markSize
  width: implicitWidth
  height: implicitHeight
  Icons.SuflyorMark {
    anchors.fill: parent
    ink: Commons.Color.foreground
  }
}
