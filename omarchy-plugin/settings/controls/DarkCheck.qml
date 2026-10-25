pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui

// Properties: text, checked, enabled; signal toggled(bool checked). Caller owns checked.
// Native switch with a wrapped label, so long translations grow the row instead of eliding.
Item {
  id: root
  property string text: ""
  property bool checked: false
  signal toggled(bool checked)
  width: parent ? parent.width : implicitWidth
  implicitHeight: Math.max(label.implicitHeight + Style.space(8), toggle.implicitHeight)
  height: implicitHeight
  opacity: enabled ? 1 : 0.5
  activeFocusOnTab: enabled
  Accessible.role: Accessible.CheckBox
  Accessible.name: text
  Accessible.checked: checked
  Accessible.onToggleAction: if (root.enabled) root.toggled(!root.checked)
  Keys.onSpacePressed: root.toggled(!root.checked)
  Keys.onReturnPressed: root.toggled(!root.checked)

  Ui.BorderSurface {
    anchors.fill: parent
    color: Style.controlFill(root.activeFocus, hover.hovered, Commons.Color.foreground, Commons.Color.accent)
    borderSpec: Commons.Border.controlSpec(root.activeFocus ? "focus" : hover.hovered ? "hover-cursor" : "normal", Commons.Color.foreground, Commons.Color.accent)
    radius: Style.cornerRadius
  }
  SettingsCaption {
    id: label
    x: Style.space(8)
    width: Math.max(0, root.width - toggle.width - Style.space(24))
    anchors.verticalCenter: parent.verticalCenter
    text: root.text
    color: Commons.Color.foreground
    font.pixelSize: Style.font.body
  }
  Ui.ToggleSwitch {
    id: toggle
    anchors.right: parent.right
    anchors.rightMargin: Style.space(8)
    anchors.verticalCenter: parent.verticalCenter
    interactive: false
    checked: root.checked
  }
  HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
  TapHandler { onTapped: { root.forceActiveFocus(); root.toggled(!root.checked) } }
}
