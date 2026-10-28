import QtQuick
import QtQuick.Layouts
import qs.Commons

// Default children: SettingsButton/SettingsDangerButton; all visible buttons split width equally.
// Properties: spacing (RowLayout). Buttons have zero minimum width for translated labels.
RowLayout {
  width: parent ? parent.width : implicitWidth
  spacing: Style.space(8)
  height: implicitHeight
}
