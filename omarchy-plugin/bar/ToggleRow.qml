import QtQuick
import QtQuick.Controls
import qs.Commons
import qs.Ui as Ui

// Native stateless switch row: the daemon remains the source of checked state.
Ui.Toggle {
  id: root

  property string accessibleLabel: label
  property bool forceArrow: false
  property bool tooltipHovered: false

  width: parent ? parent.width : implicitWidth
  opacity: enabled ? 1 : 0.5
  Accessible.role: Accessible.CheckBox
  Accessible.name: accessibleLabel
  Accessible.checkable: true
  Accessible.checked: checked
  Accessible.onToggleAction: if (enabled) clicked()
  Accessible.onPressAction: if (enabled) clicked()

  ToolTip.visible: tooltipHovered && accessibleLabel !== ""
  ToolTip.text: accessibleLabel
  ToolTip.delay: 400
  onHovered: function(isHovered) { tooltipHovered = isHovered }
  onClicked: forceActiveFocus()

  // Match action buttons: Tab must reveal controls below the viewport edge.
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

  HoverHandler { cursorShape: root.forceArrow ? Qt.ArrowCursor : Qt.PointingHandCursor }
}
