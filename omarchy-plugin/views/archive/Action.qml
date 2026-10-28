import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons

// Action rows share their available width, independent of translated label lengths.
Ui.Button {
  Layout.fillWidth: true
  Layout.preferredWidth: 1
  Layout.minimumWidth: 0
  focusable: true
  bordered: true
  opacity: enabled ? 1 : 0.45
  Accessible.role: Accessible.Button
  Accessible.name: text !== "" ? text : tooltipText
}
