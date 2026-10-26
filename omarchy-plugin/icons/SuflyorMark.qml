import QtQuick
import QtQuick.Shapes
import qs.Commons as Commons

// One optical canvas for the bar and panel identity; no icon-font fallback.
Shape {
  id: root
  property color ink: Commons.Color.foreground
  property bool recording: false
  implicitWidth: 16
  implicitHeight: 16
  width: Commons.Style.bar.iconCanvas
  height: Commons.Style.bar.iconCanvas
  fillMode: Shape.PreserveAspectFit
  horizontalAlignment: Shape.AlignHCenter
  verticalAlignment: Shape.AlignVCenter

  ShapePath {
    strokeColor: root.ink
    strokeWidth: 1.6
    fillColor: "transparent"
    capStyle: ShapePath.RoundCap
    joinStyle: ShapePath.RoundJoin
    PathSvg {
      path: root.recording
        ? "M6 3.5a2 2 0 0 1 4 0v4a2 2 0 0 1-4 0z M4 7.5v.5a4 4 0 0 0 8 0v-.5 M8 12v2 M6 14h4"
        : "M2 8V7a6 6 0 0 1 12 0v1 M2 7h2v5H2z M12 7h2v5h-2z M14 12v1l-2 1H9"
    }
  }
}
