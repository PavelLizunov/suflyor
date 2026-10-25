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
      onPressed: function(mouse) {
        if (root.locked) {
          root.locked = false
          swallow = true
          root.view.call("followup-voice-toggled")
        } else {
          pressY = mouse.y
          swallow = false
          root.view.call("followup-voice-toggled")
        }
      }
      onPositionChanged: function(mouse) {
        if (pressed && root.recording && !root.locked && pressY - mouse.y > Commons.Style.space(24)) root.locked = true
      }
      onReleased: {
        if (swallow) swallow = false
        else if (!root.locked && root.recording) root.view.call("followup-voice-toggled")
      }
      onCanceled: {
        if (!swallow && !root.locked && root.recording) root.view.call("followup-voice-toggled")
        swallow = false
      }
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
