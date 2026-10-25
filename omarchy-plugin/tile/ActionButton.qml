import QtQuick
import qs.Ui as Ui
import qs.Commons as Commons

Ui.Button {
  id: root
  property string label: ""
  implicitWidth: Commons.Style.space(30)
  implicitHeight: Commons.Style.space(30)
  horizontalPadding: Commons.Style.space(3)
  verticalPadding: Commons.Style.space(3)
  bordered: true
  focusable: true
  tooltipText: label
  Accessible.role: Accessible.Button
  Accessible.name: label
  opacity: enabled ? 1 : 0.45
}
