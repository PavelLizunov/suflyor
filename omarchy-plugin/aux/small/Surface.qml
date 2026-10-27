pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "../../tile" as Tile
import "../.." as Plugin

// The layer owns positioning; views retain close and keyboard focus semantics.
FocusScope {
  id: root
  required property var view
  property string title: ""
  property string note: ""
  property string closeCallback: "cancelled"
  property Item focusTarget: root
  default property alias content: body.data

  focus: true
  clip: true
  Accessible.name: title
  Keys.onEscapePressed: function(event) {
    root.view.call(root.closeCallback)
    event.accepted = true
  }
  function takeFocus() {
    if (root.view.win && root.view.win.visible && root.focusTarget)
      root.focusTarget.forceActiveFocus()
  }
  Component.onCompleted: Qt.callLater(root.takeFocus)
  // Explicit property keeps the frame out of the forwarded default content list.
  property Item frame: Item {
    parent: root
    anchors.fill: parent
    Connections {
      target: root.view.win || null
      function onFocusRequested() { root.takeFocus() }
      function onVisibleChanged() { if (root.view.win.visible) Qt.callLater(root.takeFocus) }
    }
    Connections {
      target: root.view.service ? root.view.service.client : null
      function onInvoked(win, name, args) {
        if (win !== root.view.win) return
        if (name === "focus") root.takeFocus()
        else if (name === "clear-focus") {
          if (root.focusTarget) root.focusTarget.focus = false
          root.focus = false
        }
      }
    }
    // AuxLayer assigns win/service after Loader construction.
    Connections {
      target: root.view
      function onWinChanged() { Qt.callLater(root.takeFocus) }
    }
    Rectangle {
      anchors.fill: parent
      color: Commons.Color.background
      border.color: Commons.Style.normalBorderColor
      radius: Commons.Style.cornerRadius
    }
    ColumnLayout {
      anchors.fill: parent
      anchors.margins: Commons.Style.spacing.xxl
      spacing: Commons.Style.spacing.lg
      RowLayout {
        Layout.fillWidth: true
        spacing: Commons.Style.spacing.sm
        Tile.Label {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          text: root.title
          font.pixelSize: Commons.Style.font.title
          font.bold: true
          Plugin.WindowDrag { anchors.fill: parent; win: root.view.win; service: root.view.service }
        }
        Tile.Label {
          visible: root.note !== ""
          Layout.maximumWidth: root.width * 0.35
          text: root.note
          color: Commons.Color.muted
          font.pixelSize: Commons.Style.font.caption
          elide: Text.ElideRight
          wrapMode: Text.NoWrap
        }
        Tile.ActionButton {
          iconText: "\uf00d"
          label: root.view.service ? root.view.service.tr("Close", []) : ""
          foreground: hot ? Commons.Color.urgent : Commons.Color.foreground
          onClicked: root.view.call(root.closeCallback)
        }
      }
      Ui.PanelSeparator { Layout.fillWidth: true }
      Item {
        id: body
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0
      }
    }
  }
}
