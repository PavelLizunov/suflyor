pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui

// Properties: text, placeholderText, minimumHeight, readOnly; signal edited(string text).
// Write only when text differs from win.prop; remote changes also emit edited.
// Multi-line plain-text editor grows with wrapped content; the Settings frame owns scrolling.
Controls.TextArea {
  id: root
  property real minimumHeight: Style.space(220)
  signal edited(string text)
  width: parent ? parent.width : implicitWidth
  height: implicitHeight
  implicitHeight: Math.max(minimumHeight, contentHeight + topPadding + bottomPadding)
  wrapMode: TextEdit.Wrap
  textFormat: TextEdit.PlainText
  selectByMouse: true
  persistentSelection: true
  activeFocusOnTab: true
  font.family: Style.font.family
  font.pixelSize: Style.font.body
  color: Commons.Color.foreground
  placeholderTextColor: Commons.Color.muted
  selectionColor: Style.selectionFillFor(Commons.Color.foreground, Commons.Color.accent)
  selectedTextColor: Commons.Color.foreground
  padding: Style.space(8)
  opacity: enabled ? 1 : 0.5
  Accessible.name: placeholderText
  // TextArea has no textEdited signal. Binding-driven changes arrive with their
  // remote mirror already updated; only a differing value is a local edit.
  onTextChanged: root.edited(text)
  background: Ui.BorderSurface {
    color: Style.controlFill(root.activeFocus, root.hovered, Commons.Color.foreground, Commons.Color.accent)
    radius: Style.cornerRadius
    borderSpec: Commons.Border.controlSpec(root.activeFocus ? "focus" : root.hovered ? "hover-cursor" : "normal", Commons.Color.foreground, Commons.Color.accent)
  }
}
