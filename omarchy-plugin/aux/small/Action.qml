import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons
import "../../icons" as Icons

Icons.ActionButton {
  Layout.fillWidth: true
  Layout.preferredWidth: 0
  Layout.minimumWidth: 0
  focusable: true
  bordered: true
  horizontalPadding: Commons.Style.spacing.lg
  verticalPadding: Commons.Style.spacing.sm
  opacity: enabled ? 1 : 0.45
  Accessible.role: Accessible.Button
  Accessible.name: text
}
