pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons as Commons
import "../../tile" as Tile

Rectangle {
  id: root
  required property var view
  required property var line
  required property int lineIndex
  width: ListView.view ? ListView.view.width : 0
  height: Math.min(Commons.Style.space(120), content.implicitHeight + Commons.Style.spacing.sm * 2)
  clip: true
  radius: Commons.Style.cornerRadius
  color: lineIndex === view.p("active-line", -1) ? Commons.Style.selectedAccentFill
    : line.matched ? Commons.Util.alpha(Commons.Color.urgent, 0.12) : "transparent"
  MouseArea {
    anchors.fill: parent
    onClicked: root.view.call("play-line", [root.lineIndex])
  }
  RowLayout {
    id: content
    anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: Commons.Style.spacing.sm; rightMargin: Commons.Style.spacing.xxl; topMargin: Commons.Style.spacing.sm }
    spacing: Commons.Style.spacing.sm
    Tile.ActionButton {
      Layout.alignment: Qt.AlignTop
      implicitWidth: Commons.Style.space(24)
      implicitHeight: Commons.Style.space(24)
      iconText: root.line.checked ? "\uf046" : "\uf096"
      selected: !!root.line.checked
      label: root.view.service ? root.view.service.tr("Copy selected", []) : ""
      onClicked: root.view.call("toggle-line", [root.lineIndex])
    }
    Tile.Label {
      visible: root.line["offset-label"] !== ""
      Layout.alignment: Qt.AlignTop
      Layout.preferredWidth: Commons.Style.space(52)
      text: root.line["offset-label"]
      font.pixelSize: Commons.Style.font.caption
      color: Commons.Color.muted
    }
    Tile.Label {
      Layout.alignment: Qt.AlignTop
      Layout.preferredWidth: Commons.Style.space(root.view.p("by-voice", false) ? 120 : 84)
      text: root.line.speaker
      font.pixelSize: Commons.Style.font.caption
      font.bold: true
      color: root.view.p("by-voice", false) ? root.view.wireColor(root.line["speaker-color"]) : Commons.Color.accent
    }
    Tile.SelectableText {
      Layout.alignment: Qt.AlignTop
      Layout.fillWidth: true
      Layout.minimumWidth: 0
      service: root.view.service
      text: root.line["display-text"]
      sourceText: root.line.text
      onCapture: function(anchor, cursor) { root.view.call("capture-line-selection", [root.lineIndex, anchor, cursor]) }
    }
    Rectangle {
      id: star
      Layout.alignment: Qt.AlignTop
      implicitWidth: Commons.Style.space(26)
      implicitHeight: Commons.Style.space(24)
      radius: Commons.Style.cornerRadius
      color: activeFocus ? Commons.Style.focusFillColor : mouse.containsMouse ? Commons.Style.hoverFill : "transparent"
      border.color: activeFocus ? Commons.Color.accent : "transparent"
      activeFocusOnTab: true
      Accessible.role: Accessible.Button
      Accessible.name: root.view.service ? root.view.service.tr("To memory", []) : ""
      Accessible.checkable: true
      Accessible.checked: !!root.line.marked
      Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
          root.view.call("toggle-line-marked", [root.lineIndex, !!(event.modifiers & Qt.ShiftModifier)])
          event.accepted = true
        }
      }
      Tile.Label {
        anchors.centerIn: parent
        text: root.line.marked ? "\uf005" : "\uf006"
        color: root.line.marked || mouse.containsMouse ? Commons.Color.accent : Commons.Color.muted
        opacity: root.line.marked || mouse.containsMouse ? 1 : 0.5
      }
      MouseArea {
        id: mouse
        property bool shiftLatched: false
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onPressed: function(event) { shiftLatched = !!(event.modifiers & Qt.ShiftModifier) }
        onClicked: {
          star.forceActiveFocus()
          root.view.call("toggle-line-marked", [root.lineIndex, shiftLatched])
        }
      }
    }
  }
}
