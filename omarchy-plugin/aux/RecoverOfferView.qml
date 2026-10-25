pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Commons as Commons
import "../tile" as Tile
import "small" as Small

Item {
  id: root
  property var win
  property var service
  function p(name, fallback) { return win ? win.prop(name, fallback) : fallback }
  function call(name, args) { if (win && service) service.client.call(win.winId, name, args || []) }
  function tr(text, args) { return service ? service.tr(text, args || []) : "" }
  Accessible.name: root.tr("Suflyor — restore session")
  // drag-start-requested / drag-moved are native-window chrome, owned by AuxLayer.
  Small.Surface {
    anchors.fill: parent
    view: root
    title: root.tr("Restore the previous session?")
    closeCallback: "dismissed"
    ColumnLayout {
      anchors.fill: parent
      spacing: Commons.Style.spacing.lg
      Controls.ScrollView {
        id: viewport
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
        contentWidth: availableWidth
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        ColumnLayout {
          width: Math.max(0, viewport.availableWidth - Commons.Style.spacing.lg)
          spacing: Commons.Style.spacing.sm
          Tile.Label {
            Layout.fillWidth: true
            text: root.tr("The last session ended without closing. Carry its context into a new one?")
          }
          Tile.Label {
            visible: root.p("has-qa", false)
            Layout.fillWidth: true
            font.bold: true
            text: root.tr("Last question:")
          }
          Tile.Label {
            visible: root.p("has-qa", false)
            Layout.fillWidth: true
            text: root.tr("{}", [root.p("last-question", "")])
          }
          Tile.Label {
            visible: root.p("has-qa", false)
            Layout.fillWidth: true
            font.bold: true
            text: root.tr("Last answer:")
          }
          Tile.Label {
            visible: root.p("has-qa", false)
            Layout.fillWidth: true
            text: root.tr("{}", [root.p("last-answer", "")])
          }
          Tile.Label {
            visible: root.p("transcript-preview", "") !== ""
            Layout.fillWidth: true
            font.bold: true
            text: root.tr("Recent lines:")
          }
          Tile.Label {
            visible: root.p("transcript-preview", "") !== ""
            Layout.fillWidth: true
            text: root.tr("{}", [root.p("transcript-preview", "")])
          }
        }
      }
      RowLayout {
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.sm
        Small.Action {
          text: root.tr("Dismiss")
          iconText: "\uf00d"
          onClicked: root.call("dismissed")
        }
        Small.Action {
          text: root.tr("Recover")
          iconText: "\uf0e2"
          selected: true
          onClicked: root.call("recover-accepted")
        }
      }
    }
  }
}
