pragma ComponentBehavior: Bound
import QtQuick

// Compositor cursor deltas stay stable when a bounded layer moves; Qt cannot
// report its changing global layer-shell origin through mapToGlobal.
MouseArea {
  id: root
  property var win: null
  property var service: null
  acceptedButtons: Qt.LeftButton
  cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
  preventStealing: true
  function drag(action) {
    if (root.win && root.service)
      root.service.client.send({ t: "drag", win: root.win.winId, action: action })
  }
  onPressed: function(mouse) { drag("begin"); mouse.accepted = true }
  onPositionChanged: if (pressed) drag("move")
  onReleased: drag("end")
  onCanceled: drag("end")
}
