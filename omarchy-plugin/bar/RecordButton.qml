import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui
import "../icons" as Icons

// Slint RecordBtn: hold, swipe up to lock, then click again to stop/send.
Item {
  id: root

  property string text: ""
  property string iconText: ""
  property bool recording: false
  property bool forceArrow: false
  property color accent: Commons.Color.accent
  property bool locked: false
  property bool held: false
  property bool swallowUp: false
  property real pressY: 0
  signal recordPressed()
  signal recordReleased()

  Layout.fillWidth: true
  Layout.preferredWidth: 1
  Layout.minimumWidth: 0
  Layout.fillHeight: true
  implicitWidth: face.implicitWidth
  implicitHeight: face.implicitHeight
  activeFocusOnTab: enabled
  Accessible.role: Accessible.Button
  Accessible.name: text
  Accessible.checkable: true
  Accessible.checked: recording
  Accessible.onPressAction: {
    if (!enabled) return
    if (held || locked) finish()
    else begin(0)
  }

  onRecordingChanged: if (!recording) locked = false
  onVisibleChanged: if (!visible) finish()
  onEnabledChanged: if (!enabled) finish()
  onActiveFocusChanged: {
    if (!activeFocus) {
      if (held && !mouse.pressed) finish()
      return
    }
    var view = parent
    while (view && !(view instanceof Flickable)) view = view.parent
    if (!view) return
    var point = root.mapToItem(view.contentItem, 0, 0)
    if (point.y < view.contentY) view.contentY = point.y
    else if (point.y + height > view.contentY + view.height)
      view.contentY = Math.min(view.contentHeight - view.height, point.y + height - view.height)
  }
  Component.onDestruction: finish()

  function begin(y) {
    forceActiveFocus()
    if (locked) {
      locked = false
      swallowUp = true
      held = false
      recordReleased()
    } else {
      pressY = y
      swallowUp = false
      held = true
      recordPressed()
    }
  }

  function release() {
    held = false
    if (swallowUp) swallowUp = false
    else if (!locked) recordReleased()
  }

  // Closing the popout must not leave a capture waiting for an unreachable up.
  function finish() {
    if (held || locked) recordReleased()
    held = false
    locked = false
    swallowUp = false
  }

  Keys.onPressed: function(event) {
    if (event.key !== Qt.Key_Space && event.key !== Qt.Key_Return && event.key !== Qt.Key_Enter) return
    event.accepted = true
    if (!event.isAutoRepeat) begin(0)
  }
  Keys.onReleased: function(event) {
    if (event.key !== Qt.Key_Space && event.key !== Qt.Key_Return && event.key !== Qt.Key_Enter) return
    event.accepted = true
    if (!event.isAutoRepeat) release()
  }

  Icons.ActionButton {
    id: face
    anchors.fill: parent
    // Presentation only: the MouseArea below owns press/release, not clicked.
    Accessible.ignored: true
    text: root.text
    iconText: root.locked ? "\u{f04db}" : root.iconText
    selected: root.recording
    hasCursor: mouse.containsMouse || root.activeFocus
    bordered: false
    foreground: root.recording ? Commons.Color.urgent : Commons.Color.popups.text
    accent: root.recording ? Commons.Color.urgent : root.accent
    horizontalPadding: Style.spacing.lg
    verticalPadding: Style.spacing.sm
    opacity: root.enabled ? 1 : 0.5
  }

  Rectangle {
    x: Style.space(7)
    anchors.verticalCenter: parent.verticalCenter
    width: Style.space(6)
    height: width
    radius: root.locked ? 0 : width / 2
    color: root.recording ? Commons.Color.urgent : "transparent"
    border.width: root.recording ? 0 : 1
    border.color: root.accent
  }
  Icons.ProjectIcon {
    anchors.right: parent.right
    anchors.rightMargin: Style.space(5)
    anchors.verticalCenter: parent.verticalCenter
    visible: root.recording && !root.locked
    name: "arrow-up"
    width: Style.font.iconSmall
    height: width
    color: Commons.Color.accent
  }
  MouseArea {
    id: mouse
    anchors.fill: parent
    acceptedButtons: Qt.LeftButton
    hoverEnabled: true
    preventStealing: true
    cursorShape: root.forceArrow ? Qt.ArrowCursor : Qt.PointingHandCursor
    onPressed: function(event) { root.begin(event.y) }
    onReleased: root.release()
    onCanceled: root.finish()
    onPositionChanged: function(event) {
      if (pressed && root.recording && !root.locked && root.pressY - event.y > Style.space(24)) root.locked = true
    }
  }
}
