import QtQuick
import QtQuick.Layouts
import qs.Commons

// Default children: a field with Layout.fillWidth and a SettingsButton { compact: true }.
// Properties: spacing (RowLayout). Only actions next to an input use compact width.
RowLayout {
  width: parent ? parent.width : implicitWidth
  spacing: Style.space(8)
  height: implicitHeight
}
