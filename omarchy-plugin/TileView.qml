pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "tile" as Tile

Item {
  id: root
  property var win
  property var service
  signal moveBy(real dx, real dy)
  property int openMenus: 0
  property bool dragging: false
  property bool autoCloseEnabled: false
  property bool autoClosePaused: false
  property int idleSecondsRemaining: 0
  readonly property bool idleHeld: interactionPoint.active || dragging || openMenus > 0
    || answerScrollBar.pressed || selectionScrollBar.pressed
  function interact() { if (root.win) root.win.interacted() }

  // Passive handlers observe activity without stealing clicks, selection or scrolling.
  PointHandler {
    id: interactionPoint
    target: null
    acceptedButtons: Qt.AllButtons
    onActiveChanged: root.interact()
    onPointChanged: if (active) root.interact()
  }
  HoverHandler {
    blocking: false
    onPointChanged: if (hovered) root.interact()
  }
  WheelHandler {
    target: null
    blocking: false
    onWheel: function(event) { root.interact(); event.accepted = false }
  }
  Keys.onPressed: function(event) { root.interact(); event.accepted = false }

  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function set(name, value) { if (root.win && root.service) root.service.client.set(root.win.winId, name, value) }
  function call(name, args, done) { if (root.win && root.service) root.service.client.call(root.win.winId, name, args || [], done) }
  function submit() {
    if (root.p("followup-busy", false) || root.p("followup-text", "") === "") return
    root.call("followup-submitted", [root.p("followup-text", "")])
    root.set("followup-text", "")
  }

  readonly property bool selecting: p("select-mode", false)
  readonly property bool capturing: p("capture-pending", false)
  readonly property bool memoryVisible: p("marked-count", 0) >= 1 || capturing
  readonly property real gutter: Commons.Style.space(14)
  readonly property string triggerTone: p("trigger-tone", "accent")
  readonly property color triggerColor: {
    if (triggerTone === "accent") return Commons.Color.accent
    if (triggerTone === "red") return Commons.Color.urgent
    // Quattro exposes no categorical palette. Derive its named hues from the
    // current accent's saturation/lightness rather than importing Slint colors.
    var hues = { violet: 0.75, sky: 0.55, green: 0.39, cyan: 0.5, orange: 0.08 }
    return Qt.hsla(hues[triggerTone] === undefined ? Commons.Color.accent.hslHue : hues[triggerTone],
                   Math.max(0.45, Commons.Color.accent.hslSaturation),
                   Commons.Color.background.hslLightness > 0.5 ? Math.min(0.4, Commons.Color.accent.hslLightness)
                     : Math.max(0.65, Math.min(0.8, Commons.Color.accent.hslLightness)), 1)
  }

  // Polling sends mark-only edits too; only those must keep the editor/anchor.
  // An identical payload can still be an explicit model replacement (e.g. a
  // repeated answer), and must run Slint's `changed blocks` stale-state guard.
  readonly property var blocks: p("blocks", [])
  property var previousBlocks: null
  property string previousBlockContent: ""
  property string previousBlockPayload: ""
  property bool initialized: false
  // set() advances the window revision. Defer writes until the blocks binding
  // has settled, otherwise resetting an editor feeds back into that binding.
  onBlocksChanged: Qt.callLater(root.resetBlockState)
  function resetBlockState() {
    var nextBlocks = root.blocks
    if (nextBlocks === previousBlocks) return
    previousBlocks = nextBlocks
    var content = JSON.stringify(nextBlocks.map(function(block) {
      return { kind: block.kind, text: block.text, display: block["display-text"], lang: block.lang }
    }))
    var payload = JSON.stringify(nextBlocks)
    var markOnly = content === previousBlockContent && payload !== previousBlockPayload
    previousBlockContent = content
    previousBlockPayload = payload
    if (!initialized || markOnly) return
    root.set("copied-block-index", -1)
    root.set("marked-count", 0)
    root.set("mark-anchor", -1)
    root.set("capture-block-index", -1)
    root.set("capture-text", "")
    root.set("capture-pending", false)
    root.set("select-mode", false)
  }
  Component.onCompleted: {
    root.resetBlockState()
    initialized = true
  }

  implicitWidth: Commons.Style.space(460)
  implicitHeight: Commons.Style.space(360)
  Accessible.name: root.service ? root.service.tr("tile", []) : ""

  Rectangle {
    objectName: "tile-frame"
    anchors.fill: parent
    color: Commons.Color.popups.background
    border.color: Commons.Style.normalBorderColor
    radius: Commons.Style.cornerRadius
    // Slint fades the entire card, including its chrome and descendants.
    opacity: root.p("body-opacity", 1)
    clip: true
    ColumnLayout {
      anchors.fill: parent
      anchors.margins: Commons.Style.spacing.xxl
      spacing: Commons.Style.spacing.lg
      Tile.Header {
        Layout.fillWidth: true
        view: root
        onMoveBy: function(dx, dy) { root.moveBy(dx, dy) }
      }
      Ui.PanelSeparator { Layout.fillWidth: true }
      Tile.Label {
        Layout.fillWidth: true
        visible: root.p("speak-error", false)
        text: root.service.tr("Read-aloud is unavailable. Check the installed voice and try again.", [])
        color: Commons.Color.urgent
      }
      Rectangle {
        visible: root.p("question-text", "") !== ""
        Layout.fillWidth: true
        implicitHeight: question.implicitHeight + Commons.Style.spacing.sm * 2
        color: Commons.Style.selectedAccentFill
        radius: Commons.Style.cornerRadius
        Tile.Label {
          id: question
          x: Commons.Style.spacing.lg
          y: Commons.Style.spacing.sm
          width: parent.width - x * 2
          text: root.p("question-text", "")
          font.bold: true
          maximumLineCount: 5
          elide: Text.ElideRight
        }
      }
      Row {
        visible: root.p("trigger-label", "") !== "" && root.p("trigger-label", "").toLowerCase() !== root.p("tile-title", "Untitled tile").toLowerCase()
        Layout.fillWidth: true
        Rectangle {
          width: Math.min(trigger.implicitWidth + Commons.Style.spacing.lg * 2, parent.width)
          height: Commons.Style.space(22)
          radius: Commons.Style.cornerRadius
          color: Commons.Util.alpha(root.triggerColor, Commons.Style.selectedFillAlpha)
          border.color: root.triggerColor
          Tile.Label {
            id: trigger
            anchors.fill: parent
            anchors.leftMargin: Commons.Style.spacing.lg
            anchors.rightMargin: Commons.Style.spacing.lg
            text: root.p("trigger-label", "")
            font.pixelSize: Commons.Style.font.caption
            font.bold: true
            color: root.triggerColor
            elide: Text.ElideRight
            wrapMode: Text.NoWrap
            verticalAlignment: Text.AlignVCenter
          }
        }
      }
      Controls.ScrollView {
        id: answer
        visible: !root.selecting
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
        contentWidth: availableWidth
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        Controls.ScrollBar.vertical: Controls.ScrollBar { id: answerScrollBar }
        Tile.MarkdownBlocks {
          width: Math.max(0, answer.availableWidth - root.gutter)
          bottomPadding: root.p("speak-active", false) ? Commons.Style.space(44) : 0
          view: root
        }
      }
      Tile.TermPanel {
        visible: root.p("term-path", []).length > 0 && !root.selecting
        Layout.fillWidth: true
        Layout.maximumHeight: Math.min(Commons.Style.space(180), root.height * 0.35)
        view: root
      }
      Flickable {
        id: selection
        visible: root.selecting && !root.capturing
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        Controls.ScrollBar.vertical: Controls.ScrollBar { id: selectionScrollBar }
        // TextArea's native Flickable integration keeps drag selection scrolling
        // even while the pointer is held still beyond the viewport edge.
        Controls.TextArea.flickable: Tile.SelectableText {
          service: root.service
          onInteracted: root.interact()
          onMenuOpenChanged: root.openMenus += menuOpen ? 1 : -1
          text: root.p("select-text", "")
          onCapture: function(anchor, cursor) { root.call("capture-selection", [-1, anchor, cursor]) }
        }
      }
      Tile.MemoryPanel {
        visible: root.memoryVisible
        Layout.fillWidth: true
        Layout.fillHeight: root.selecting && root.capturing
        view: root
      }
      Tile.Player {
        visible: root.p("speak-active", false)
        Layout.fillWidth: true
        view: root
      }
      Tile.FollowupRow {
        visible: root.p("convo-id", -1) >= 0
        Layout.fillWidth: true
        Layout.rightMargin: root.gutter
        view: root
      }
      Tile.Label {
        objectName: "tile-close-countdown"
        visible: root.autoCloseEnabled
        Layout.fillWidth: true
        text: root.autoClosePaused ? root.service.tr("Auto-close paused", [])
          : root.service.tr("Closes in {} s", [root.idleSecondsRemaining])
        color: !root.autoClosePaused && root.idleSecondsRemaining <= 5 ? Commons.Color.urgent : Commons.Color.muted
        font.pixelSize: Commons.Style.font.caption
        horizontalAlignment: Text.AlignRight
        wrapMode: Text.NoWrap
        elide: Text.ElideRight
      }
    }
  }
}
