pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons as Commons
import "icons" as Icons

// The same grip.svg used by Slint's DragGrip and the tile header.
Item {
  id: root
  property var win: null
  property var service: null
  implicitWidth: Commons.Style.space(16)
  implicitHeight: Commons.Style.space(30)
  Accessible.name: service ? service.tr("Drag to move the window", []) : ""
  Icons.ProjectIcon {
    anchors.centerIn: parent
    name: "grip"
    color: Commons.Color.accent
    width: Commons.Style.space(13)
    height: width
  }
  WindowDrag { anchors.fill: parent; win: root.win; service: root.service }
}
