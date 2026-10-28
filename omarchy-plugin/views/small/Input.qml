import QtQuick
import qs.Ui as Ui

Ui.TextField {
  id: root
  // Keep Slint's layout-independent clipboard shortcuts alongside Qt's native ones.
  Keys.onPressed: function(event) {
    if (!(event.modifiers & Qt.ControlModifier) || (event.modifiers & Qt.ShiftModifier)) return
    switch (event.text.toLowerCase()) {
    case "\u043c": root.paste(); event.accepted = true; break
    case "\u0441": root.copy(); event.accepted = true; break
    case "\u0447": root.cut(); event.accepted = true; break
    case "\u0444": root.selectAll(); event.accepted = true; break
    }
  }
}
