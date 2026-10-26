pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Commons as Commons
import "../icons" as Icons

RowLayout {
  id: root
  objectName: "tile-header"
  required property var view
  signal moveBy(real dx, real dy)
  spacing: Commons.Style.spacing.sm

  Item {
    Layout.fillWidth: true
    Layout.minimumWidth: 0
    implicitHeight: Commons.Style.space(30)
    clip: true
    Icons.ProjectIcon {
      anchors.verticalCenter: parent.verticalCenter
      name: "grip"
      color: Commons.Color.muted
      Accessible.name: root.view.service.tr("Drag to move the window", [])
    }
    Label {
      anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: Commons.Style.space(17) }
      text: root.view.p("question-text", "") !== ""
        ? root.view.service.tr("#{}", [root.view.p("sequence", 0)])
        : root.view.service.tr("#{}  {}", [root.view.p("sequence", 0), root.view.p("tile-title", "Untitled tile")])
      font.bold: true
      elide: Text.ElideRight
      wrapMode: Text.NoWrap
    }
    MouseArea {
      anchors.fill: parent
      acceptedButtons: Qt.LeftButton
      cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
      preventStealing: true
      property point lastGlobal
      onPressed: function(mouse) { lastGlobal = mapToGlobal(mouse.x, mouse.y) }
      onPositionChanged: function(mouse) {
        if (!pressed) return
        var now = mapToGlobal(mouse.x, mouse.y)
        root.moveBy(now.x - lastGlobal.x, now.y - lastGlobal.y)
        lastGlobal = now
      }
    }
  }
  Rectangle {
    visible: root.view.p("source-label", "—") !== ""
    // Also reserve room for all actions on narrow conversational/error tiles.
    Layout.minimumWidth: 0
    Layout.preferredWidth: Math.max(0, Math.min(source.implicitWidth + Commons.Style.spacing.lg * 2, Commons.Style.space(150), root.width - buttons.implicitWidth - Commons.Style.space(72)))
    implicitHeight: Commons.Style.space(22)
    color: "transparent"
    border.color: Commons.Style.normalBorderColor
    radius: Commons.Style.cornerRadius
    Label {
      id: source
      anchors.fill: parent
      anchors.leftMargin: Commons.Style.spacing.lg
      anchors.rightMargin: Commons.Style.spacing.lg
      text: root.view.p("source-label", "—")
      color: Commons.Color.muted
      font.pixelSize: Commons.Style.font.caption
      elide: Text.ElideRight
      wrapMode: Text.NoWrap
      verticalAlignment: Text.AlignVCenter
    }
  }
  RowLayout {
    id: buttons
    objectName: "tile-header-actions"
    spacing: Commons.Style.spacing.sm
    ActionButton {
      visible: root.view.p("can-copy", false)
      iconText: root.view.p("copied", false) ? "\uf00c" : "\uf0c5"
      label: root.view.service.tr(root.view.p("copied", false) ? "copied" : "Copy", [])
      foreground: root.view.p("copied", false) ? Commons.Color.accent : Commons.Color.foreground
      onClicked: root.view.call("copy-clicked")
    }
    ActionButton {
      id: more
      iconText: "\uf141"
      label: root.view.service.tr("More actions", [])
      onClicked: actions.open()
      Controls.Menu {
        id: actions
        x: Math.min(0, more.width - width)
        y: more.height + Commons.Style.spacing.sm
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.body
        palette.text: Commons.Color.menu.text
        palette.windowText: Commons.Color.menu.text
        palette.highlight: Commons.Color.menu.selectedBackground
        palette.highlightedText: Commons.Color.menu.selectedText
        background: Rectangle {
          color: Commons.Color.menu.background
          border.color: Commons.Color.menu.border
          radius: Commons.Style.cornerRadius
        }
        Controls.MenuItem {
          text: root.view.service.tr(root.view.p("select-mode", false) ? "Rich view" : "Select text", [])
          onTriggered: root.view.call("toggle-select-mode")
        }
        Controls.MenuItem {
          visible: root.view.p("can-regenerate", false)
          height: visible ? implicitHeight : 0
          text: root.view.service.tr("Regenerate", [])
          enabled: !root.view.p("followup-busy", false)
          onTriggered: root.view.call("regenerate-clicked")
        }
        Controls.MenuItem {
          visible: root.view.p("can-escalate", false)
          height: visible ? implicitHeight : 0
          text: root.view.service.tr("Smart cloud", [])
          enabled: !root.view.p("followup-busy", false)
          onTriggered: root.view.call("escalate-clicked")
        }
        // No pin entry: tile.slint removed it (audit #135); `pinned` and
        // pin-clicked stay declared only because Rust still references them.
      }
    }
    ActionButton {
      visible: root.view.p("can-retry", false)
      iconText: "\uf021"
      foreground: Commons.Color.accent
      label: root.view.service.tr("Retry", [])
      onClicked: root.view.call("retry-clicked")
    }
    ActionButton {
      visible: root.view.p("can-speak", false)
      iconText: root.view.p("speak-active", false) ? "\uf04d" : "\uf028"
      foreground: root.view.p("speak-active", false) ? Commons.Color.urgent : Commons.Color.foreground
      label: root.view.service.tr(root.view.p("speak-active", false) ? "Stop reading aloud" : "Read aloud", [])
      onClicked: root.view.call("speak-clicked")
    }
    ActionButton {
      iconText: root.view.p("tile-maximized", false) ? "\uf066" : "\uf065"
      label: root.view.service.tr(root.view.p("tile-maximized", false) ? "Restore tile size" : "Maximize tile", [])
      onClicked: root.view.call("maximize-clicked")
    }
    ActionButton {
      iconText: "\uf00d"
      label: root.view.service.tr("Close", [])
      foreground: hot ? Commons.Color.urgent : Commons.Color.foreground
      onClicked: root.view.call("close-clicked")
    }
  }
}
