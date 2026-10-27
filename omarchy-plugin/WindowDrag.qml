pragma ComponentBehavior: Bound
import QtQuick

// A layer-shell surface cannot use the window manager's normal window drag.
// Keep the title's pointer grab and let its owning layer update its margins.
MouseArea {
  id: root
  property var win: null
  property var service: null
  property point lastGlobal
  acceptedButtons: Qt.LeftButton
  cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
  preventStealing: true
  onPressed: function(mouse) { lastGlobal = mapToGlobal(mouse.x, mouse.y) }
  onPositionChanged: function(mouse) {
    if (!pressed || !root.win || !root.service) return
    var now = mapToGlobal(mouse.x, mouse.y)
    root.service.client.moveRequested(root.win.winId, now.x - lastGlobal.x, now.y - lastGlobal.y)
    lastGlobal = now
  }
}
