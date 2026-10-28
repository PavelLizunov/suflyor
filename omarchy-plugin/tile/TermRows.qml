pragma ComponentBehavior: Bound
import QtQuick
import qs.Ui as Ui
import qs.Commons as Commons

Column {
  id: root
  property var rows: []
  property string current: ""
  signal picked(string term)
  spacing: Commons.Style.spacing.sm

  Repeater {
    model: root.rows
    // Reflow the host's rows if shell typography needs more room than Slint.
    delegate: Flow {
      id: termRow
      required property var modelData
      width: root.width
      spacing: Commons.Style.spacing.sm
      Repeater {
        model: termRow.modelData.items || []
        delegate: Ui.Button {
          id: chip
          required property string modelData
          text: modelData
          width: Math.min(implicitWidth, termRow.width)
          fontSize: Commons.Style.font.bodySmall
          verticalPadding: Commons.Style.space(3)
          bordered: true
          focusable: true
          selected: modelData === root.current
          Accessible.role: Accessible.Button
          Accessible.name: modelData
          onClicked: root.picked(chip.modelData)
        }
      }
    }
  }
}
