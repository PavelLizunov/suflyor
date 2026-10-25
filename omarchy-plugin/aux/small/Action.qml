import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui

Ui.Button {
  Layout.fillWidth: true
  Layout.preferredWidth: 0
  Layout.minimumWidth: 0
  focusable: true
  bordered: true
  opacity: enabled ? 1 : 0.45
  Accessible.role: Accessible.Button
  Accessible.name: text
}
