pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons
import "controls" as C

// SettingsWindow tab 2: read-only global-hotkey table, including platform chord labels.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property bool macos: {
    if (!service || !service.client) return false
    var revision = service.client.globalsRev
    return (service.client.globals || {})["Platform.is-macos"] === true
  }
  readonly property var entries: [
    ["Super+F9", "Hold to record a microphone question; release to send it to AI"],
    ["Super+Shift+F9", "Hold to record system audio; release to send it to AI"],
    ["F9", "Ask AI now (streams the answer into a tile)"],
    ["Shift+F9", "Ask AI escalated to the smart cloud model (one-shot; tile shows a cloud badge)"],
    ["F3", "Re-ask the last question with fresh context"],
    ["F6", "Manual tile from the last transcript line"],
    ["F8", "Screenshot → AI explains it"],
    ["Shift+F8", "Screenshot → translate only (no screen description)"],
    ["Ctrl+F8", "Screenshot → OCR the text, then read it aloud"],
    ["Shift+Alt+1", "Read the selected text aloud"],
    ["Shift+Alt+2", "OCR a screen region and read it aloud"],
    ["Shift+Alt+3", "Pause / resume reading aloud"],
    ["F4", "Knowledge-base search (open / close)"],
    ["F1", "Help (open / close)"],
    ["F7", "Session archive (open / close)"]
  ]
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  function chord(key) {
    if (!root.service) return ""
    if (root.macos) {
      if (key === "Ctrl+F8") return root.service.tr("Control+F8", [])
      if (key === "Shift+Alt+1") return root.service.tr("Shift+Option+1", [])
      if (key === "Shift+Alt+2") return root.service.tr("Shift+Option+2", [])
      if (key === "Shift+Alt+3") return root.service.tr("Shift+Option+3", [])
    }
    // Machine key names are literal Slint model data; still route visible text through tr.
    return root.service.tr(key, [])
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Global hotkeys (work anywhere, even when the app isn't focused):", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Super is the Windows key. Hold the shortcut to record, then release it to ask. If another application uses a shortcut, Suflyor leaves it unchanged; see Diagnostics.", []) : ""
    }
    C.SettingsSeparator {}
    Repeater {
      model: root.entries
      delegate: Item {
        id: entry
        required property var modelData
        width: parent.width
        implicitHeight: Math.max(chordText.implicitHeight, description.implicitHeight) + Style.space(8)
        height: implicitHeight
        C.SettingsCaption {
          id: chordText
          x: Style.space(4)
          y: Style.space(4)
          width: Style.space(root.macos ? 112 : 140)
          text: root.chord(entry.modelData[0])
          color: Commons.Color.foreground
          font.pixelSize: Style.font.body
          font.bold: true
        }
        C.SettingsCaption {
          id: description
          x: chordText.x + chordText.width + Style.space(12)
          y: Style.space(4)
          width: Math.max(0, entry.width - x - Style.space(4))
          text: root.service ? root.service.tr(entry.modelData[1], []) : ""
          color: Commons.Color.foreground
          font.pixelSize: Style.font.body
        }
      }
    }
  }
}
