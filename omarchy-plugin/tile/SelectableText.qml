import QtQuick
import QtQuick.Controls as Controls
import qs.Commons as Commons

Controls.TextArea {
  id: root
  required property var service
  property string sourceText: text
  property bool noWrap: false
  readonly property bool sourceMatches: text === sourceText
  signal capture(int anchor, int cursor)
  signal interacted()
  property bool menuOpen: false
  onSelectionStartChanged: interacted()
  onSelectionEndChanged: interacted()
  Component.onDestruction: if (menuOpen) menuOpen = false

  // Qt positions are UTF-16 code units; suflyord slices UTF-8 byte offsets.
  function byteOffset(position) {
    var bytes = 0
    for (var i = 0; i < position; i++) {
      var c = root.sourceText.charCodeAt(i)
      if (c < 0x80) bytes++
      else if (c < 0x800) bytes += 2
      else if (c >= 0xd800 && c <= 0xdbff && i + 1 < position
               && root.sourceText.charCodeAt(i + 1) >= 0xdc00
               && root.sourceText.charCodeAt(i + 1) <= 0xdfff) { bytes += 4; i++ }
      else bytes += 3
    }
    return bytes
  }

  height: implicitHeight
  padding: 0
  background: null
  readOnly: true
  textFormat: TextEdit.PlainText
  wrapMode: noWrap ? TextEdit.NoWrap : TextEdit.Wrap
  selectByMouse: sourceMatches
  enabled: sourceMatches
  activeFocusOnPress: sourceMatches
  persistentSelection: true
  font.family: Commons.Style.font.family
  font.pixelSize: Commons.Style.font.body
  color: Commons.Color.foreground
  selectionColor: Commons.Style.selectionFill
  selectedTextColor: Commons.Color.foreground
  clip: true

  Keys.onPressed: function(event) {
    root.interacted()
    if ((event.modifiers & Qt.ControlModifier) && !(event.modifiers & Qt.ShiftModifier)) {
      if (event.text.toLowerCase() === "\u0441") { root.copy(); event.accepted = true }
      if (event.text.toLowerCase() === "\u0444") { root.selectAll(); event.accepted = true }
    }
  }

  MouseArea {
    anchors.fill: parent
    acceptedButtons: Qt.RightButton
    enabled: root.sourceMatches
    onClicked: function(mouse) { menu.popup(mouse.x, mouse.y) }
  }
  Controls.Menu {
    id: menu
    onOpened: { root.menuOpen = true; root.interacted() }
    onClosed: { root.menuOpen = false; root.interacted() }
    font.family: Commons.Style.font.family
    font.pixelSize: Commons.Style.font.body
    palette.text: Commons.Color.menu.text
    palette.windowText: Commons.Color.menu.text
    palette.highlight: Commons.Color.menu.selectedBackground
    palette.highlightedText: Commons.Color.menu.selectedText
    background: Rectangle {
      // Keep the themed menu as wide as its visible localized items.
      implicitWidth: {
        var width = Commons.Style.space(200)
        for (var i = 0; i < menu.count; i++) {
          var item = menu.itemAt(i)
          if (item && item.visible) width = Math.max(width, item.implicitWidth + menu.leftPadding + menu.rightPadding)
        }
        return width
      }
      color: Commons.Color.menu.background
      border.color: Commons.Color.menu.border
      radius: Commons.Style.cornerRadius
    }
    Controls.MenuItem {
      text: root.service.tr("Copy selection", [])
      enabled: root.selectionStart !== root.selectionEnd
      onTriggered: root.copy()
    }
    Controls.MenuItem {
      text: root.service.tr("Add selection to memory", [])
      enabled: root.selectionStart !== root.selectionEnd
      onTriggered: root.capture(root.byteOffset(root.selectionStart), root.byteOffset(root.selectionEnd))
    }
  }
}
