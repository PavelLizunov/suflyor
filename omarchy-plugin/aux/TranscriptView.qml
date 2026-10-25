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
  // mark-anchor and capture-line-index are host-owned bookkeeping: Rust owns
  // range marking and capture reseeding. start-ms is likewise consumed by
  // play-line in Rust, not by this view. Drag callbacks are replaced by AuxLayer.
  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function set(name, value) { if (root.win && root.service) root.service.client.set(root.win.winId, name, value) }
  function call(name, args) { if (root.win && root.service) root.service.client.call(root.win.winId, name, args || []) }
  // The wire uses Slint #rrggbbaa; Qt's eight-digit color string is #aarrggbb.
  function wireColor(value) {
    return typeof value === "string" && /^#[0-9a-fA-F]{8}$/.test(value)
      ? "#" + value.slice(7, 9) + value.slice(1, 7) : (value || Commons.Color.accent)
  }
  function scrollToLine(index) {
    if (root.p("lines", []).length === 0) return
    lines.positionViewAtIndex(Math.max(0, Math.min(index, root.p("lines", []).length - 1)), ListView.Center)
  }
  function dismiss() {
    if (root.p("confirm-rediar", false)) root.set("confirm-rediar", false)
    else root.call("close-requested")
  }
  function focusWindow() { root.forceActiveFocus() }
  readonly property bool confirming: p("confirm-rediar", false)
  onConfirmingChanged: if (!confirming) Qt.callLater(root.focusWindow)
  implicitWidth: Commons.Style.space(900)
  implicitHeight: Commons.Style.space(640)
  focus: true
  Keys.onEscapePressed: function(event) { root.dismiss(); event.accepted = true }
  onWinChanged: if (win) Qt.callLater(root.focusWindow)
  Connections {
    target: root.win || null
    function onFocusRequested() { root.focusWindow() }
    function onVisibleChanged() { if (root.win.visible) root.focusWindow() }
  }
  Connections {
    target: root.service ? root.service.client : null
    function onInvoked(w, name, args) {
      if (w !== root.win) return
      if (name === "scroll-to-line") root.scrollToLine(args[0])
      else if (name === "focus") root.focusWindow()
      else if (name === "clear-focus") { root.forceActiveFocus(); root.focus = false }
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
      spacing: Commons.Style.spacing.sm
      enabled: !root.p("confirm-rediar", false)
      Archive.Header {
        Layout.fillWidth: true
        view: root
        icon: "\uf15c"
        title: root.service ? root.service.tr("Transcript", []) : ""
        note: root.p("heading", "")
      }
      Ui.PanelSeparator { Layout.fillWidth: true }
      RowLayout {
        visible: !root.p("empty", false)
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.sm
        Archive.Action {
          iconText: "\uf0c5"
          text: root.service ? root.service.tr(root.p("copied", false) ? "Copied [ok]" : "Copy all", []) : ""
          onClicked: root.call("copy-all-requested")
        }
        Archive.Action {
          iconText: "\uf0c5"
          text: root.service ? root.service.tr(root.p("copied", false) ? "Copied [ok]" : "Copy selected", []) : ""
          onClicked: root.call("copy-selected-requested")
        }
        Ui.Toggle {
          Layout.preferredWidth: Commons.Style.space(260)
          implicitHeight: Commons.Style.space(36)
          titleSize: Commons.Style.font.body
          label: root.service ? root.service.tr("with timecodes [mm:ss]", []) : ""
          checked: root.p("with-timecodes", false)
          onClicked: root.set("with-timecodes", !root.p("with-timecodes", false))
        }
      }
      // Keep every diarization state reachable without letting an eight-speaker
      // toolbar or long backend status consume the transcript's viewport.
      Controls.ScrollView {
        id: diarization
        visible: !root.p("empty", false) && (root.p("can-diarize", false) || root.p("has-diarization", false) || root.p("needs-diar-models", false))
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(diarControls.implicitHeight, root.height * 0.36)
        Layout.minimumHeight: 0
        contentWidth: availableWidth
        clip: true
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        Archive.Diarization {
          id: diarControls
          width: Math.max(0, diarization.availableWidth - Commons.Style.spacing.xxl)
          view: root
        }
      }
      RowLayout {
        visible: !root.p("empty", false)
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.sm
        Ui.TextField {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          text: root.p("search-query", "")
          placeholderText: root.service ? root.service.tr("Search in transcript", []) : ""
          onTextEdited: {
            root.set("search-query", text)
            root.call("search-edited", [text])
          }
          onAccepted: root.call("search-jump", [1])
        }
        Tile.Label {
          visible: root.p("search-query", "") !== ""
          // The source formats this language-neutral counter without @tr.
          text: root.p("search-pos", 0) + " / " + root.p("search-count", 0)
          color: Commons.Color.muted
          font.pixelSize: Commons.Style.font.caption
        }
        Tile.ActionButton {
          iconText: "\uf104"
          label: root.service ? root.service.tr("Search in transcript", []) : ""
          onClicked: root.call("search-jump", [-1])
        }
        Tile.ActionButton {
          iconText: "\uf105"
          label: root.service ? root.service.tr("Search in transcript", []) : ""
          onClicked: root.call("search-jump", [1])
        }
      }
      Ui.Button {
        visible: !root.p("empty", false)
        Layout.fillWidth: true
        focusable: true
        leftAlign: true
        iconText: root.p("all-selected", false) ? "\uf046" : "\uf096"
        text: root.service ? root.service.tr("select all", []) : ""
        Accessible.role: Accessible.CheckBox
        Accessible.name: text
        Accessible.checkable: true
        Accessible.checked: root.p("all-selected", false)
        onClicked: root.call("toggle-all", [!root.p("all-selected", false)])
      }
      Tile.Label {
        visible: root.p("empty", false)
        Layout.fillWidth: true
        Layout.fillHeight: true
        text: root.service ? root.service.tr("Transcript not saved", []) : ""
        color: Commons.Color.muted
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
      }
      ListView {
        id: lines
        Layout.fillWidth: true
        Layout.fillHeight: !root.p("empty", false)
        Layout.preferredHeight: root.p("empty", false) ? 0 : Commons.Style.space(200)
        Layout.minimumHeight: 0
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: root.p("lines", [])
        spacing: Commons.Style.spacing.xs
        Controls.ScrollBar.vertical: Controls.ScrollBar { policy: Controls.ScrollBar.AsNeeded }
        delegate: Archive.TranscriptRow {
          required property var modelData
          required property int index
          view: root
          line: modelData
          lineIndex: index
        }
      }
      Archive.TranscriptMemory {
        visible: root.p("marked-count", 0) >= 1 || root.p("capture-pending", false)
        Layout.fillWidth: true
        view: root
      }
      Tile.Label {
        visible: root.p("overflow-count", 0) > 0
        Layout.fillWidth: true
        text: root.service ? root.service.tr("{} more lines not shown - use 'Copy all' for the full transcript", [root.p("overflow-count", 0)]) : ""
        color: Commons.Color.muted
        font.pixelSize: Commons.Style.font.caption
        horizontalAlignment: Text.AlignHCenter
      }
      Archive.TranscriptPlayer {
        visible: root.p("has-audio", false)
        Layout.fillWidth: true
        view: root
      }
      Tile.Label {
        visible: !root.p("empty", false) && !root.p("has-audio", false)
        Layout.fillWidth: true
        text: root.service ? root.service.tr("Audio not saved", []) : ""
        color: Commons.Color.muted
        font.pixelSize: Commons.Style.font.caption
        horizontalAlignment: Text.AlignHCenter
      }
    }
  }
  Archive.Confirmation {
    anchors.fill: parent
    visible: root.p("confirm-rediar", false)
    view: root
    title: root.service ? root.service.tr("Re-detect speakers?", []) : ""
    explanation: root.service ? root.service.tr("Speaker names you set will be reset.", []) : ""
    acceptText: root.service ? root.service.tr("Re-detect", []) : ""
    onAccepted: {
      root.set("confirm-rediar", false)
      root.call("run-diarization")
    }
    onCancelled: root.set("confirm-rediar", false)
  }
}
