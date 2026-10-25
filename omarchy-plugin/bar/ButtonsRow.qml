import QtQuick
import QtQuick.Layouts
import qs.Commons

// Visible buttons divide the entire row equally, including conditional actions.
RowLayout {
  width: parent ? parent.width : implicitWidth
  height: implicitHeight
  spacing: Style.spacing.controlGap
}
