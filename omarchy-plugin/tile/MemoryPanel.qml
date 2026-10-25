pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons

ColumnLayout {
  id: root
  required property var view
  readonly property bool pending: view.p("capture-pending", false)
  readonly property int markedCount: view.p("marked-count", 0)
  spacing: Commons.Style.spacing.sm

  Controls.ScrollView {
    id: viewport
    visible: root.markedCount === 1 || root.pending
    Layout.fillWidth: true
    Layout.fillHeight: true
    Layout.minimumHeight: Commons.Style.space(44)
    Layout.preferredHeight: Commons.Style.space(88)
    Layout.maximumHeight: Commons.Style.space(160)
    contentWidth: availableWidth
    Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
    Controls.TextArea {
      width: viewport.availableWidth
      text: root.view.p("capture-text", "")
      textFormat: TextEdit.PlainText
      wrapMode: TextEdit.Wrap
      selectByMouse: true
      font.family: Commons.Style.font.family
      font.pixelSize: Commons.Style.font.body
      color: Commons.Color.foreground
      selectionColor: Commons.Style.selectionFill
      selectedTextColor: Commons.Color.foreground
      background: Rectangle {
        color: Commons.Style.normalFill
        border.color: Commons.Style.normalBorderColor
        radius: Commons.Style.cornerRadius
      }
      onTextChanged: if (text !== root.view.p("capture-text", "")) root.view.set("capture-text", text)
    }
  }
  RowLayout {
    Layout.fillWidth: true
    spacing: Commons.Style.spacing.lg
    Item { Layout.fillWidth: true }
    Ui.Button {
      focusable: true
      bordered: true
      text: root.pending ? root.view.service.tr("To memory", []) : root.view.service.tr("To memory ({})", [root.markedCount])
      onClicked: root.view.call(root.pending ? "save-capture" : "save-marked")
    }
    Ui.Button {
      visible: root.markedCount >= 1
      focusable: true
      bordered: true
      text: root.view.service.tr("Copy", [])
      onClicked: root.view.call("copy-marked")
    }
    Ui.Button {
      focusable: true
      bordered: true
      text: root.view.service.tr("Cancel", [])
      onClicked: root.view.call(root.pending ? "cancel-capture" : "clear-marks")
    }
  }
}
