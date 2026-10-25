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

  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function set(name, value) { if (root.win && root.service) root.service.client.set(root.win.winId, name, value) }
  function call(name, args) { if (root.win && root.service) root.service.client.call(root.win.winId, name, args || []) }
  function submit() {
    if (root.p("followup-busy", false) || root.p("followup-text", "") === "") return
    root.call("followup-submitted", [root.p("followup-text", "")])
    root.set("followup-text", "")
  }

  readonly property bool selecting: p("select-mode", false)
  readonly property bool capturing: p("capture-pending", false)
  readonly property bool memoryVisible: p("marked-count", 0) >= 1 || capturing
  readonly property real gutter: Commons.Style.spacing.xxl + Commons.Style.space(2)
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

  // The wire mirror has no model identity: ignore mark-only in-place updates,
  // otherwise starring a block would immediately clear its own mark/editor.
  // Content replacements reproduce Slint's `changed blocks` stale-state guard.
  readonly property string blockContent: JSON.stringify(p("blocks", []).map(function(block) {
    return { kind: block.kind, text: block.text, display: block["display-text"], lang: block.lang }
  }))
  property bool initialized: false
  onBlockContentChanged: {
    if (!initialized) return
    root.set("copied-block-index", -1)
    root.set("marked-count", 0)
    root.set("mark-anchor", -1)
    root.set("capture-block-index", -1)
    root.set("capture-text", "")
    root.set("capture-pending", false)
    root.set("select-mode", false)
  }
  Component.onCompleted: initialized = true

  implicitWidth: Commons.Style.space(460)
  implicitHeight: Commons.Style.space(360)
  Accessible.name: root.service ? root.service.tr("tile", []) : ""

  Rectangle {
    anchors.fill: parent
    color: Commons.Color.background
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
      Controls.ScrollView {
        id: selection
        visible: root.selecting && !root.capturing
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
        contentWidth: availableWidth
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        Tile.SelectableText {
          id: selectionText
          width: selection.availableWidth
          service: root.service
          text: root.p("select-text", "")
          onCapture: function(anchor, cursor) { root.call("capture-selection", [-1, anchor, cursor]) }
          // Keep keyboard/drag selection visible across the entire answer.
          onCursorRectangleChanged: {
            if (!activeFocus || !selection.contentItem) return
            var viewport = selection.contentItem
            if (cursorRectangle.y < viewport.contentY) viewport.contentY = cursorRectangle.y
            else if (cursorRectangle.y + cursorRectangle.height > viewport.contentY + viewport.height)
              viewport.contentY = Math.min(Math.max(0, height - viewport.height), cursorRectangle.y + cursorRectangle.height - viewport.height)
          }
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
    }
  }
}
