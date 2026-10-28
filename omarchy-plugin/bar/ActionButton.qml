import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import "../icons" as Icons

Icons.ActionButton {
  id: root

  property bool checkable: false
  property bool forceArrow: false
  property string accessibleLabel: text

  Layout.fillWidth: true
  Layout.preferredWidth: 1
  Layout.minimumWidth: 0
  Layout.fillHeight: true
  focusable: true
  bordered: false
  horizontalPadding: Style.spacing.lg
  verticalPadding: Style.spacing.sm
  fontSize: Style.font.body
  foreground: Commons.Color.popups.text
  opacity: enabled ? 1 : 0.5
  tooltipText: accessibleLabel
  Accessible.role: Accessible.Button
  Accessible.name: accessibleLabel
  Accessible.checkable: checkable
  Accessible.checked: checkable && selected
  Accessible.onPressAction: if (enabled) clicked()

  // Keep Tab navigation inside the panel viewport, including short screens.
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
