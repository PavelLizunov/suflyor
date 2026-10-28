import QtQuick
import qs.Commons
import qs.Ui as Ui

// Properties: text (already translated); wraps at the available width. No signals.
Ui.PanelSectionHeader {
  width: parent ? parent.width : implicitWidth
  wrapMode: Text.Wrap
  fontSize: Style.font.body
}
