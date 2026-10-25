import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui

// Properties: text, iconText, compact (only beside a field), enabled; signal clicked().
// Fills a card, or an equal share of SettingsButtonsRow; compact uses natural width.
Ui.Button {
  property bool compact: false
  width: compact ? implicitWidth : parent ? parent.width : implicitWidth
  height: Math.max(Style.spacing.controlHeight, implicitHeight)
  Layout.fillWidth: !compact
  Layout.preferredWidth: compact ? implicitWidth : 1
  Layout.minimumWidth: 0
  Layout.minimumHeight: Style.spacing.controlHeight
  Layout.fillHeight: true
  focusable: true
  bordered: true
  opacity: enabled ? 1 : 0.5
  Accessible.role: Accessible.Button
  Accessible.name: text
}
