pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons

Column {
  id: root
  required property var view
  spacing: Commons.Style.spacing.sm

  component MarkButton: Item {
    id: mark
    required property int blockIndex
    property bool marked: false
    implicitWidth: Commons.Style.space(24)
    implicitHeight: Commons.Style.space(22)
    Accessible.role: Accessible.Button
    Accessible.name: root.view.service.tr("To memory", [])
    Accessible.checkable: true
    Accessible.checked: marked
    activeFocusOnTab: true
    Keys.onSpacePressed: function(event) { root.view.call("toggle-block-marked", [mark.blockIndex, !!(event.modifiers & Qt.ShiftModifier)]) }
    Label {
      anchors.centerIn: parent
      text: mark.marked ? "\uf005" : "\uf006"
      color: mark.marked ? Commons.Color.accent : Commons.Color.muted
      opacity: mark.marked || touch.containsMouse || mark.activeFocus ? 1 : 0.55
    }
    MouseArea {
      id: touch
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      property bool shiftLatched: false
      onPressed: function(mouse) { shiftLatched = !!(mouse.modifiers & Qt.ShiftModifier) }
      onClicked: root.view.call("toggle-block-marked", [mark.blockIndex, shiftLatched])
    }
  }

  Repeater {
    model: root.view.p("blocks", [])
    delegate: Column {
      id: block
      required property var modelData
      required property int index
      readonly property int kind: modelData.kind
      readonly property bool code: kind === 5 && modelData.lang !== "user"
      readonly property bool userTurn: kind === 5 && modelData.lang === "user"
      readonly property bool boxed: kind === 5 || kind === 7
      width: root.width
      spacing: Commons.Style.spacing.sm
      visible: kind >= 0 && kind <= 7

      Ui.PanelSeparator { visible: block.kind === 6; width: block.width }
      Rectangle {
        visible: block.kind !== 6 && block.kind >= 0 && block.kind <= 7
        width: block.width
        height: body.implicitHeight + (block.boxed ? Commons.Style.spacing.lg * 2 : 0)
        color: block.userTurn ? Commons.Style.selectedAccentFill : block.boxed ? Commons.Color.menu.background : "transparent"
        border.width: block.boxed ? 1 : 0
        border.color: block.userTurn ? Commons.Color.accent : Commons.Style.normalBorderColor
        radius: Commons.Style.cornerRadius
        clip: true
        ColumnLayout {
          id: body
          x: block.boxed ? Commons.Style.spacing.lg : 0
          y: block.boxed ? Commons.Style.spacing.lg : 0
          width: parent.width - x * 2
          spacing: Commons.Style.spacing.sm
          Label {
            visible: block.userTurn
            text: root.view.service.tr("You", [])
            color: Commons.Color.accent
            font.pixelSize: Commons.Style.font.caption
            font.bold: true
          }
          RowLayout {
            visible: block.code
            Layout.fillWidth: true
            Label {
              Layout.fillWidth: true
              text: block.modelData.lang
              color: Commons.Color.muted
              font.pixelSize: Commons.Style.font.caption
              elide: Text.ElideRight
              wrapMode: Text.NoWrap
            }
            ActionButton {
              implicitWidth: Commons.Style.space(24)
              implicitHeight: Commons.Style.space(22)
              readonly property bool copied: root.view.p("copied-block-index", -1) === block.index
              iconText: copied ? "\uf00c" : "\uf0c5"
              label: root.view.service.tr(copied ? "copied" : "Copy", [])
              foreground: copied ? Commons.Color.accent : Commons.Color.muted
              onClicked: root.view.call("copy-block-clicked", [block.index, block.modelData.text])
            }
            MarkButton { blockIndex: block.index; marked: !!block.modelData.marked }
          }
          RowLayout {
            Layout.fillWidth: true
            spacing: Commons.Style.spacing.sm
            Layout.leftMargin: block.kind === 4 ? Commons.Style.spacing.xxl : 0
            Label {
              visible: block.kind === 4
              text: "•"
              color: Commons.Color.accent
              Layout.alignment: Qt.AlignTop
            }
            SelectableText {
              Layout.fillWidth: true
              Layout.minimumWidth: 0
              service: root.view.service
              text: block.modelData["display-text"]
              sourceText: block.modelData.text
              noWrap: block.code || block.kind === 7
              font.pixelSize: block.kind === 1 ? Commons.Style.font.heading
                : block.kind === 2 ? Commons.Style.font.title
                : block.kind === 3 ? Commons.Style.font.subtitle
                : block.code || block.kind === 7 ? Commons.Style.font.bodySmall : Commons.Style.font.body
              font.bold: block.kind >= 1 && block.kind <= 3
              onCapture: function(anchor, cursor) { root.view.call("capture-selection", [block.index, anchor, cursor]) }
            }
            MarkButton {
              visible: (block.kind === 0 || block.kind === 4) && !root.view.p("plain-notice", false)
              Layout.alignment: Qt.AlignTop
              blockIndex: block.index
              marked: !!block.modelData.marked
            }
          }
        }
      }
    }
  }

  Column {
    width: root.width
    visible: root.view.p("answer-terms", []).length > 0
    spacing: Commons.Style.spacing.sm
    topPadding: Commons.Style.spacing.lg
    Ui.PanelSectionHeader { text: root.view.service.tr("Terms", []); foreground: Commons.Color.muted }
    TermRows {
      width: parent.width
      rows: root.view.p("answer-terms", [])
      current: root.view.p("term-path", []).length ? root.view.p("term-path", [])[0] : ""
      onPicked: function(term) { root.view.call("term-opened", [term, false]) }
    }
  }
}
