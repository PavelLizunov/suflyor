import QtQuick
import QtQuick.Effects
import qs.Commons
import qs.Commons as Commons

// Properties: markSize; uses the original Suflyor brand asset. No signals.
Item {
  property real markSize: Style.space(22)
  implicitWidth: markSize
  implicitHeight: markSize
  width: implicitWidth
  height: implicitHeight
  Image {
    id: mark
    anchors.fill: parent
    visible: false
    source: Qt.resolvedUrl("brand-head.svg")
    fillMode: Image.PreserveAspectFit
    asynchronous: true
  }
  MultiEffect {
    anchors.fill: mark
    source: mark
    colorization: 1
    colorizationColor: Commons.Color.foreground
  }
}
