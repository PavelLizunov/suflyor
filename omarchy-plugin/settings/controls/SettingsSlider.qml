import QtQuick
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui

// Properties: minimum, maximum, value, step, enabled; signals changed(real value), released(real value).
// Adds keyboard access to the native slider; only user interaction emits changed.
Ui.PanelSlider {
  id: root
  signal changed(real value)
  width: parent ? parent.width : implicitWidth
  height: Math.max(Style.spacing.controlHeight, implicitHeight)
  trackColor: Qt.alpha(Commons.Color.foreground, 0.15)
  fillColor: Commons.Color.accent
  knobColor: activeFocus ? Commons.Color.accent : Commons.Color.foreground
  tickColor: Commons.Color.background
  opacity: enabled ? 1 : 0.5
  activeFocusOnTab: enabled
  Accessible.role: Accessible.Slider
  onMoved: function(value) { root.changed(value) }
  function nudge(delta) {
    var next = Math.max(minimum, Math.min(maximum, value + delta))
    root.changed(next)
    root.released(next)
  }
  Keys.onLeftPressed: root.nudge(-step)
  Keys.onRightPressed: root.nudge(step)
  // Keys has no Home/End handlers of its own.
  Keys.onPressed: function(event) {
    if (event.key === Qt.Key_Home) { root.changed(minimum); root.released(minimum); event.accepted = true }
    else if (event.key === Qt.Key_End) { root.changed(maximum); root.released(maximum); event.accepted = true }
  }
}
