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
  function set(name, value) { if (win && service) service.client.set(win.winId, name, value) }
  function call(name, args) { if (win && service) service.client.call(win.winId, name, args || []) }
  function tr(text, args) { return service ? service.tr(text, args || []) : "" }
  // Local equivalent of source-label. Remote invoke_* currently has no return channel.
  function sourceLabel(source) {
    return ["glossary", "commands", "patterns"].indexOf(source) >= 0 ? tr(source) : tr("{}", [source])
  }
  function activate() {
    if (p("results", []).length > 0) call("result-activated", [p("selected-index", 0)])
  }
  Accessible.name: root.tr("Suflyor — knowledge base")
  // drag-start-requested / drag-moved are native-window chrome, owned by AuxLayer.
  // PaletteResult.title is not displayed by palette.slint either (key + preview are shown).
  Small.Surface {
    anchors.fill: parent
    view: root
    title: root.tr("Knowledge palette")
    closeCallback: "close-requested"
    focusTarget: search
    ColumnLayout {
      anchors.fill: parent
      spacing: Commons.Style.spacing.sm
      Small.Input {
        id: search
        Layout.fillWidth: true
        placeholderText: root.tr("Search knowledge or snippets…")
        text: root.p("query", "")
        onTextEdited: {
          root.set("query", text)
          root.call("query-changed", [text])
          root.set("selected-index", 0)
        }
        onAccepted: root.activate()
      }
      Tile.Label {
        Layout.fillWidth: true
        text: root.tr("Esc closes · Enter opens a tile")
        color: Commons.Color.muted
        font.pixelSize: Commons.Style.font.caption
      }
      ColumnLayout {
        visible: root.p("recent-chips", []).length > 0
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.xs
        Tile.Label {
          text: root.tr("recent:")
          color: Commons.Color.muted
          font.pixelSize: Commons.Style.font.caption
        }
        RowLayout {
          Layout.fillWidth: true
          spacing: Commons.Style.spacing.xs
          Repeater {
            model: root.p("recent-chips", [])
            delegate: Small.Action {
              id: chip
              required property string modelData
              text: root.tr("{}", [modelData])
              onClicked: root.call("chip-clicked", [chip.modelData])
            }
          }
        }
      }
      Controls.ScrollView {
        id: viewport
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
        contentWidth: availableWidth
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        ColumnLayout {
          width: Math.max(0, viewport.availableWidth - Commons.Style.spacing.lg)
          spacing: Commons.Style.spacing.xs
          Tile.Label {
            visible: root.p("results", []).length === 0
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            color: Commons.Color.muted
            text: root.p("query", "") === ""
              ? root.tr("Start typing to search the embedded knowledge base…")
              : root.tr("No results for \"{}\".", [root.p("query", "")])
          }
          Repeater {
            model: root.p("results", [])
            delegate: Rectangle {
              id: result
              required property var modelData
              required property int index
              readonly property bool selected: index === root.p("selected-index", 0)
              Layout.fillWidth: true
              implicitHeight: row.implicitHeight + Commons.Style.spacing.lg * 2
              radius: Commons.Style.cornerRadius
              color: selected ? Commons.Style.selectedAccentFill : "transparent"
              border.color: activeFocus || selected ? Commons.Color.accent : "transparent"
              activeFocusOnTab: true
              Accessible.role: Accessible.Button
              Accessible.name: root.tr("{}", [modelData.key])
              Keys.onReturnPressed: root.call("result-activated", [index])
              Keys.onEnterPressed: root.call("result-activated", [index])
              Keys.onSpacePressed: root.call("result-activated", [index])
              ColumnLayout {
                id: row
                anchors.fill: parent
                anchors.margins: Commons.Style.spacing.lg
                spacing: Commons.Style.spacing.xs
                RowLayout {
                  Layout.fillWidth: true
                  Tile.Label {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    text: root.tr("{}", [result.modelData.key])
                    font.bold: true
                    color: result.selected ? Commons.Color.accent : Commons.Color.foreground
                    elide: Text.ElideRight
                    wrapMode: Text.NoWrap
                  }
                  Tile.Label {
                    text: root.sourceLabel(result.modelData.source)
                    color: Commons.Color.muted
                    font.pixelSize: Commons.Style.font.caption
                  }
                }
                Tile.Label {
                  Layout.fillWidth: true
                  text: root.tr("{}", [result.modelData.preview])
                  font.pixelSize: Commons.Style.font.bodySmall
                  elide: Text.ElideRight
                  wrapMode: Text.NoWrap
                }
              }
              MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.call("result-activated", [result.index])
              }
            }
          }
        }
      }
    }
  }
}
