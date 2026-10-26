pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons as Commons
import qs.Ui as Ui
import "../icons" as Icons

// Compact native switch row; daemon state remains authoritative.
Ui.BorderSurface {
  id: root
  property string label: ""
  property string accessibleLabel: label
  property string iconName: ""
  property bool checked: false
  property bool forceArrow: false
  property color foreground: Commons.Color.popups.text
  property color accent: Commons.Color.accent
  signal clicked()

  width: parent ? parent.width : implicitWidth
  implicitHeight: Math.max(Commons.Style.spacing.popupRowHeight, Commons.Style.space(32))
  radius: Commons.Style.cornerRadius
  color: Commons.Style.controlFill(activeFocus, mouse.containsMouse, foreground, accent)
  borderSpec: activeFocus || mouse.containsMouse
    ? Commons.Border.controlSpec(activeFocus ? "focus" : "hover-cursor", foreground, accent)
    : Commons.Border.none()
  opacity: enabled ? 1 : 0.5
  activeFocusOnTab: true
  Keys.onReturnPressed: root.clicked()
  Keys.onEnterPressed: root.clicked()
  Keys.onSpacePressed: root.clicked()
  Accessible.role: Accessible.CheckBox
  Accessible.name: accessibleLabel
  Accessible.checkable: true
  Accessible.checked: checked
  Accessible.onToggleAction: if (enabled) clicked()
  Accessible.onPressAction: if (enabled) clicked()

  Icons.ProjectIcon {
    id: icon
    name: root.iconName
    width: Commons.Style.space(16)
    height: width
    x: Commons.Style.spacing.lg
    color: root.foreground
    anchors.verticalCenter: parent.verticalCenter
  }
  Text {
    objectName: root.objectName ? root.objectName + "-label" : ""
    anchors.left: icon.right
    anchors.leftMargin: Commons.Style.spacing.controlGap
    anchors.right: track.left
    anchors.rightMargin: Commons.Style.spacing.controlGap
    anchors.verticalCenter: parent.verticalCenter
    text: root.label
    textFormat: Text.PlainText
    font.family: Commons.Style.font.family
    font.pixelSize: Commons.Style.font.body
    color: root.foreground
    elide: Text.ElideRight
  }
  Ui.ToggleSwitch {
    id: track
    objectName: root.objectName ? root.objectName + "-switch" : ""
    anchors.right: parent.right
    anchors.rightMargin: Commons.Style.spacing.lg
    anchors.verticalCenter: parent.verticalCenter
    trackHeight: Commons.Style.space(16)
    checked: root.checked
    foreground: root.foreground
    accent: root.accent
    interactive: false
  }
  MouseArea {
    id: mouse
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: root.forceArrow ? Qt.ArrowCursor : Qt.PointingHandCursor
    onClicked: { root.forceActiveFocus(); root.clicked() }
  }
  onActiveFocusChanged: {
    if (!activeFocus) return
    var view = parent
    while (view && !(view instanceof Flickable)) view = view.parent
    if (!view) return
    var point = root.mapToItem(view.contentItem, 0, 0)
    if (point.y < view.contentY) view.contentY = point.y
    else if (point.y + height > view.contentY + view.height)
      view.contentY = Math.min(view.contentHeight - view.height, point.y + height - view.height)
  }
}
