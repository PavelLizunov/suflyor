import QtQuick
import qs.Commons
import qs.Commons as Commons

// Properties: label, icon (Nerd Font glyph), tabIndex, active; signal chosen(int index).
SettingsButton {
  id: root
  property string label: ""
  property string icon: ""
  property int tabIndex: -1
  signal chosen(int index)
  text: label
  iconText: icon
  selected: active
  leftAlign: true
  bordered: false
  foreground: active ? Commons.Color.accent : Commons.Color.foreground
  onClicked: root.chosen(root.tabIndex)
  Accessible.role: Accessible.PageTab
  Accessible.name: label
  Accessible.checked: active
}
