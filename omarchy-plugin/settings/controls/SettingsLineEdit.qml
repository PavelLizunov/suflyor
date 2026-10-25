import QtQuick
import qs.Commons
import qs.Ui as Ui

// Properties: text, placeholderText, password, enabled; native textEdited/accepted/editingFinished signals.
// Bind text to win.prop and write on textEdited (not onTextChanged).
Ui.TextField {
  width: parent ? parent.width : implicitWidth
  height: Math.max(Style.spacing.controlHeight, implicitHeight)
  selectByMouse: true
  opacity: enabled ? 1 : 0.5
  Accessible.name: placeholderText
}
