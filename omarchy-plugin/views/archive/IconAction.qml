import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui

// A compact icon button for per-row actions, as archive.slint draws them:
// the label is the tooltip and the accessible name, not visible text.
Ui.Button {
  property string label: ""
  Layout.fillWidth: false
  focusable: true
  bordered: true
  tooltipText: label
  opacity: enabled ? 1 : 0.45
  Accessible.role: Accessible.Button
  Accessible.name: label
}
