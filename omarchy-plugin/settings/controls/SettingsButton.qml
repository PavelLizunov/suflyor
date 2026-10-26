import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import "../../icons" as Icons

// Properties: text, iconText, compact (only beside a field), enabled; signal clicked().
// Fills a card, or an equal share of SettingsButtonsRow; compact uses natural width.
Icons.ActionButton {
  id: root
  Layout.fillWidth: !compact
  Layout.preferredWidth: compact ? implicitWidth : 1
  Layout.minimumWidth: 0
  Layout.minimumHeight: Style.spacing.controlHeight
  Layout.fillHeight: true
  wrapLabel: !compact
  focusable: true
  bordered: true
  horizontalPadding: Style.spacing.lg
  verticalPadding: Style.spacing.sm
  opacity: enabled ? 1 : 0.5
  Accessible.role: Accessible.Button
  Accessible.name: text
  Accessible.onPressAction: if (root.enabled) root.clicked()
}
