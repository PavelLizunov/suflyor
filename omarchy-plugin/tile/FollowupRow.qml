pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons

RowLayout {
  id: root
  required property var view
  readonly property bool recording: view.p("voice-recording", false)
  property bool locked: false
  onRecordingChanged: if (!recording) locked = false
  spacing: Commons.Style.spacing.lg

  Ui.TextField {
    Layout.fillWidth: true
    Layout.minimumWidth: 0
    placeholderText: root.view.service.tr("Continue the dialog…", [])
    text: root.view.p("followup-text", "")
    enabled: !root.view.p("followup-busy", false)
    onTextEdited: root.view.set("followup-text", text)
    onAccepted: root.view.submit()
  }
  Rectangle {
    Layout.preferredWidth: Commons.Style.space(36)
    Layout.preferredHeight: Commons.Style.space(32)
    color: voice.containsMouse ? Commons.Style.hoverFill : "transparent"
    radius: Commons.Style.cornerRadius
    border.width: 1
    border.color: root.locked ? Commons.Color.urgent : root.recording ? Commons.Color.accent : Commons.Style.normalBorderColor
    enabled: !root.view.p("followup-busy", false) || root.recording
    opacity: enabled ? 1 : 0.45
    Accessible.role: Accessible.Button
    Accessible.name: root.view.service.tr("Continue the dialog…", [])
    Label {
      anchors.centerIn: parent
      text: root.locked ? "\uf04d" : root.recording ? "\uf062" : "\uf130"
      color: root.locked ? Commons.Color.urgent : root.recording ? Commons.Color.accent : Commons.Color.foreground
    }
    MouseArea {
      id: voice
      anchors.fill: parent
      acceptedButtons: Qt.LeftButton
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      preventStealing: true
      property real pressY: 0
      property bool swallow: false
      property bool startPending: false
      property bool stopOnStart: false
      property bool lockOnStart: false
      function release() {
        if (swallow) swallow = false
        else if (!root.locked) {
          // The daemon may not have echoed voice-recording before mouse-up.
          // Wait for the start callback so a rejected start never toggles ON
          // from this release, and an accepted quick tap always toggles OFF.
          if (startPending) stopOnStart = true
          else if (root.recording) root.view.call("followup-voice-toggled")
        }
      }
      onPressed: function(mouse) {
        if (startPending) {
          swallow = true
          return
        }
        if (root.locked) {
          root.locked = false
          swallow = true
          root.view.call("followup-voice-toggled")
        } else {
          pressY = mouse.y
          swallow = false
          stopOnStart = false
          lockOnStart = false
          startPending = true
          root.view.call("followup-voice-toggled", [], function(value, error) {
            voice.startPending = false
            if (!error && root.recording) {
              if (voice.lockOnStart) root.locked = true
              else if (voice.stopOnStart && !root.locked) root.view.call("followup-voice-toggled")
            }
            voice.stopOnStart = false
            voice.lockOnStart = false
          })
        }
      }
      onPositionChanged: function(mouse) {
        if (pressed && !root.locked && pressY - mouse.y > Commons.Style.space(24)) {
          if (root.recording) root.locked = true
          else if (startPending) lockOnStart = true
        }
      }
      onReleased: release()
      onCanceled: release()
    }
  }
  ActionButton {
    text: root.view.p("followup-busy", false) ? "…" : "→"
    enabled: !root.view.p("followup-busy", false)
    // The Slint send button has only an arrow; reuse its input's msgid for a11y.
    label: root.view.service.tr("Continue the dialog…", [])
    onClicked: root.view.submit()
  }
}
