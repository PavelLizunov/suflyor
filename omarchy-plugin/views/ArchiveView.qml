pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "../tile" as Tile
import "archive" as Archive

Item {
  id: root
  property var win
  property var service
  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function set(name, value) { if (root.win && root.service) root.service.client.set(root.win.winId, name, value) }
  function call(name, args) { if (root.win && root.service) root.service.client.call(root.win.winId, name, args || []) }
  function focusSearch() { search.forceActiveFocus() }
  function dismiss() {
    if (p("confirm-resummary-index", -1) !== -1) call("resummary-cancelled")
    else if (p("confirm-delete-index", -1) !== -1) call("delete-cancelled")
    else if (p("renaming-index", -1) !== -1) call("rename-cancelled")
    else call("close-requested")
  }
  readonly property bool confirming: p("confirm-delete-index", -1) !== -1 || p("confirm-resummary-index", -1) !== -1
  readonly property int renamingIndex: p("renaming-index", -1)
  onConfirmingChanged: if (!confirming) Qt.callLater(root.focusSearch)
  onRenamingIndexChanged: if (renamingIndex === -1) Qt.callLater(root.focusSearch)
  implicitWidth: Commons.Style.space(860)
  implicitHeight: Commons.Style.space(600)
  focus: true
  Keys.onEscapePressed: function(event) { root.dismiss(); event.accepted = true }
  onWinChanged: if (win) Qt.callLater(root.focusSearch)
  Connections {
    target: root.win || null
    function onFocusRequested() { root.focusSearch() }
    function onVisibleChanged() { if (root.win.visible) root.focusSearch() }
  }
  Connections {
    target: root.service ? root.service.client : null
    function onInvoked(w, name, args) {
      if (w !== root.win) return
      if (name === "focus") root.focusSearch()
      else if (name === "clear-focus") { root.forceActiveFocus(); search.focus = false; root.focus = false }
    }
  }
  Rectangle {
    anchors.fill: parent
    color: Commons.Color.background
    border.color: Commons.Style.normalBorderColor
    radius: Commons.Style.cornerRadius
    ColumnLayout {
      anchors.fill: parent
      anchors.margins: Commons.Style.spacing.xxl
      spacing: Commons.Style.spacing.lg
      enabled: !root.confirming
      Archive.Header {
        Layout.fillWidth: true
        view: root
        icon: "\uf187"
        title: !root.service ? "" : root.p("summary", "") === ""
          ? root.service.tr("Session archive", []) : root.service.tr("Session archive ({})", [root.p("summary", "")])
      }
      Ui.TextField {
        id: search
        Layout.fillWidth: true
        placeholderText: root.service ? root.service.tr("Search transcripts and AI answers…", []) : ""
        text: root.p("query", "")
        onTextEdited: {
          root.set("query", text)
          root.set("notice", "")
          root.call("query-changed", [text])
          root.set("selected-index", 0)
        }
        onAccepted: if (root.p("results", []).length > 0) root.call("result-activated", [root.p("selected-index", 0)])
      }
      Ui.PanelSeparator { Layout.fillWidth: true }
      Tile.Label {
        visible: root.p("retranscribe-busy", false)
        Layout.fillWidth: true
        text: root.p("retranscribe-status", "")
        color: Commons.Color.accent
        horizontalAlignment: Text.AlignHCenter
      }
      Tile.Label {
        visible: !root.p("retranscribe-busy", false) && root.p("notice", "") !== ""
        Layout.fillWidth: true
        text: root.p("notice", "")
        color: Commons.Color.urgent
        horizontalAlignment: Text.AlignHCenter
      }
      Tile.Label {
        visible: root.p("stt-is-cloud", false) && !root.p("retranscribe-busy", false)
        Layout.fillWidth: true
        text: root.service ? root.service.tr("Generate sends the saved audio of a session to cloud speech recognition.", []) : ""
        color: Commons.Color.urgent
        horizontalAlignment: Text.AlignHCenter
      }
      Tile.Label {
        visible: root.p("unavailable", false)
        Layout.fillWidth: true
        Layout.fillHeight: true
        text: root.service ? root.service.tr("The session archive is unavailable.", []) + "\n"
          + root.service.tr("The session catalog could not be opened. Press Close, then reopen Session archive to try again.", []) : ""
        color: Commons.Color.muted
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
      }
      Controls.ScrollView {
        id: results
        visible: !root.p("unavailable", false)
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
        contentWidth: availableWidth
        clip: true
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        Column {
          width: Math.max(0, results.availableWidth - Commons.Style.spacing.xxl)
          spacing: Commons.Style.spacing.sm
          Tile.Label {
            visible: root.p("results", []).length === 0
            width: parent.width
            text: !root.service ? "" : root.p("query", "") === "" ? root.service.tr("No archived sessions yet.", [])
              : root.service.tr("No matches for \"{}\".", [root.p("query", "")])
            color: Commons.Color.muted
            horizontalAlignment: Text.AlignHCenter
          }
          Repeater {
            model: root.p("results", [])
            Archive.SessionRow {
              required property var modelData
              required property int index
              width: parent.width
              view: root
              row: modelData
              rowIndex: index
            }
          }
        }
      }
      Tile.Label {
        Layout.fillWidth: true
        text: root.service ? root.service.tr("Esc to close · Enter / click to open a session", []) : ""
        color: Commons.Color.muted
        font.pixelSize: Commons.Style.font.caption
        horizontalAlignment: Text.AlignHCenter
      }
    }
  }
  Archive.Confirmation {
    anchors.fill: parent
    visible: root.p("confirm-delete-index", -1) !== -1
    view: root
    destructive: true
    title: root.service ? root.service.tr("Delete session?", []) : ""
    subject: root.p("confirm-delete-title", "")
    explanation: root.service ? root.service.tr("Irreversible — transcript, audio and summary are erased.", []) : ""
    acceptText: root.service ? root.service.tr("Delete", []) : ""
    onAccepted: root.call("delete-confirmed", [root.p("confirm-delete-index", -1)])
    onCancelled: root.call("delete-cancelled")
  }
  Archive.Confirmation {
    anchors.fill: parent
    visible: root.p("confirm-resummary-index", -1) !== -1
    view: root
    title: root.service ? root.service.tr("Recreate summary?", []) : ""
    subject: root.p("confirm-resummary-title", "")
    explanation: root.service ? root.service.tr("A summary already exists — recreate it?", []) : ""
    acceptText: root.service ? root.service.tr("Recreate", []) : ""
    onAccepted: root.call("resummary-confirmed", [root.p("confirm-resummary-index", -1)])
    onCancelled: root.call("resummary-cancelled")
  }
}
