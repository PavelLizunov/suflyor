pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons

// Compact model list row; actions stack only at narrow widths.
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
  implicitHeight: content.implicitHeight + Style.space(16)
  height: implicitHeight
  radius: Style.cornerRadius
  color: active ? Qt.alpha(Commons.Color.accent, 0.12) : "transparent"

  ColumnLayout {
    id: content
    x: Style.space(8)
    y: Style.space(8)
    width: Math.max(0, root.width - Style.space(16))
    spacing: Style.space(6)
    GridLayout {
      Layout.fillWidth: true
      columns: root.width < Style.space(380) ? 1 : 2
      columnSpacing: Style.space(12)
      rowSpacing: Style.space(6)
      Column {
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        spacing: Style.space(2)
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
      }
      SettingsButton {
        compact: true
        Layout.preferredWidth: Math.min(content.width, Style.space(150))
        Layout.fillHeight: false
        wrapLabel: true
        text: root.busy ? (root.service ? root.service.tr("Cancel", []) : "") : root.action
        enabled: root.busy || root.actionEnabled
        onClicked: root.busy ? root.cancelClicked() : root.actionClicked()
      }
    }
    Rectangle {
      visible: root.busy && root.progressVisible
      Layout.fillWidth: true
      implicitHeight: Style.space(6)
      radius: height / 2
      color: Qt.alpha(Commons.Color.foreground, 0.15)
      Rectangle {
        width: parent.width * Math.max(0, Math.min(1, root.progress))
        height: parent.height
        radius: parent.radius
        color: Commons.Color.accent
      }
    }
  }
}
