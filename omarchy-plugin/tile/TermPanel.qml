pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Commons as Commons

Rectangle {
  id: root
  required property var view
  readonly property var path: view.p("term-path", [])
  implicitHeight: Math.min(card.implicitHeight + Commons.Style.spacing.lg * 2, Commons.Style.space(180))
  color: Commons.Color.menu.background
  border.color: Commons.Style.normalBorderColor
  radius: Commons.Style.cornerRadius
  clip: true

  Controls.ScrollView {
    id: viewport
    anchors.fill: parent
    anchors.margins: Commons.Style.spacing.lg
    contentWidth: availableWidth
    Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
    ColumnLayout {
      id: card
      width: viewport.availableWidth
      spacing: Commons.Style.spacing.sm
      RowLayout {
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.sm
        Item {
          Layout.fillWidth: true
          implicitHeight: Commons.Style.space(24)
          clip: true
          Row {
            spacing: Commons.Style.spacing.sm
            Repeater {
              model: root.path
              delegate: Row {
                id: crumb
                required property string modelData
                required property int index
                spacing: Commons.Style.spacing.sm
                Label { visible: crumb.index > 0; text: "›"; color: Commons.Color.muted }
                Label {
                  text: crumb.modelData
                  width: Math.min(implicitWidth, Commons.Style.space(140))
                  elide: Text.ElideRight
                  wrapMode: Text.NoWrap
                  font.bold: crumb.index === root.path.length - 1
                  color: crumb.index === root.path.length - 1 ? Commons.Color.foreground : Commons.Color.accent
                  MouseArea {
                    anchors.fill: parent
                    enabled: crumb.index < root.path.length - 1
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.view.call("term-crumb", [crumb.index])
                  }
                }
              }
            }
          }
        }
        ActionButton {
          implicitWidth: Commons.Style.space(24)
          implicitHeight: Commons.Style.space(24)
          iconText: "\uf00d"
          label: root.view.service.tr("Close the definition", [])
          onClicked: root.view.call("term-closed")
        }
      }
      Label {
        Layout.fillWidth: true
        text: root.view.p("term-definition", "") || (root.view.p("term-busy", false) ? "…" : "")
      }
      TermRows {
        Layout.fillWidth: true
        rows: root.view.p("term-subterms", [])
        visible: rows.length > 0 && !root.view.p("term-busy", false)
        onPicked: function(term) { root.view.call("term-opened", [term, true]) }
      }
    }
  }
}
