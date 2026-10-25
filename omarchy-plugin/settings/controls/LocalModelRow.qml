pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons

// Properties: title, detail, chip, chipTone (0 neutral/1 good/2 warning), action,
// active, actionEnabled, busy, progressVisible, progress, service (for Cancel).
// Signals: actionClicked(), cancelClicked(). Actions fill width, including Cancel.
Rectangle {
  id: root
  property string title: ""
  property string detail: ""
  property string chip: ""
  property int chipTone: 0
  property string action: ""
  property bool active: false
  property bool actionEnabled: true
  property bool busy: false
  property bool progressVisible: false
  property real progress: 0
  property var service: null
  signal actionClicked()
  signal cancelClicked()
  width: parent ? parent.width : implicitWidth
  implicitHeight: content.implicitHeight + Style.space(8)
  height: implicitHeight
  radius: Style.cornerRadius
  color: active ? Qt.alpha(Commons.Color.accent, 0.12) : "transparent"

  Column {
    id: content
    x: Style.space(4)
    y: Style.space(4)
    width: Math.max(0, root.width - Style.space(8))
    spacing: Style.space(4)
    SettingsCaption {
      text: root.title
      color: Commons.Color.foreground
      font.pixelSize: Style.font.body
      font.bold: root.active
    }
    SettingsCaption { text: root.detail; visible: text !== "" }
    StatusLine {
      text: root.chip
      tone: root.chipTone === 1 ? "good" : root.chipTone === 2 ? "warning" : "muted"
    }
    SettingsButton {
      text: root.action
      enabled: root.actionEnabled
      onClicked: root.actionClicked()
    }
    WrapBlock {
      visible: root.busy
      Rectangle {
        visible: root.progressVisible
        width: parent.width
        height: Style.space(6)
        radius: height / 2
        color: Qt.alpha(Commons.Color.foreground, 0.15)
        Rectangle {
          width: parent.width * Math.max(0, Math.min(1, root.progress))
          height: parent.height
          radius: parent.radius
          color: Commons.Color.accent
        }
      }
      SettingsButton {
        text: root.service ? root.service.tr("Cancel", []) : ""
        onClicked: root.cancelClicked()
      }
    }
  }
}
