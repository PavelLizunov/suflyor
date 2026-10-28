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
  function call(name, args) { if (win && service) service.client.call(win.winId, name, args || []) }
  function tr(text, args) { return service ? service.tr(text, args || []) : "" }
  readonly property bool mac: {
    if (!service) return false
    var revision = service.client.globalsRev
    return service.client.globals["Platform.is-macos"] === true
  }
  function keyLabel(key) {
    if (!mac) return tr(key)
    var names = { "Ctrl+F8": "Control+F8", "Shift+Alt+1": "Shift+Option+1",
      "Shift+Alt+2": "Shift+Option+2", "Shift+Alt+3": "Shift+Option+3" }
    return tr(names[key] || key)
  }
  readonly property var icons: [
    { glyph: "\uf130", description: root.tr("Microphone: turn recording of your voice on or off") },
    { glyph: "\uf028", description: root.tr("System audio: turn recording of the other side on or off") },
    { glyph: root.tr("ask"), description: root.tr("Hold to ask by voice; swipe up so you do not have to hold") },
    { glyph: root.tr("grab"), description: root.tr("Hold to send the other side's last words to AI; swipe up so you do not have to hold") },
    { glyph: "\uf04b", description: root.tr("Start or end the session: transcript and auto-tips") },
    { glyph: root.tr("$0.000"), description: root.tr("Session cost (cloud AI only)") },
    { glyph: "\uf070", description: root.tr("Stealth: hide Suflyor windows from screen capture") },
    { glyph: "\uf06d", description: root.tr("Frequent auto-tiles: answer more phrases") },
    { glyph: "\uf09c", description: root.tr("Tile lock: keep listening without opening new tiles") },
    { glyph: "\uf187", description: root.tr("Session archive (also F7)") },
    { glyph: "\uf15c", description: root.tr("Meeting summary: participants, topics, decisions, tasks") },
    { glyph: "\uf040", description: root.tr("Write a question as text") },
    { glyph: "\uf067", description: root.tr("New tile from the last phrase (also F6)") },
    { glyph: "\uf030", description: root.tr("Screenshot a region, AI answers (also F8)") },
    { glyph: "\uf013", description: root.tr("Settings") },
    { glyph: "\uf059", description: root.tr("This help (also F1)") },
    { glyph: "\uf021", description: root.tr("Restart Suflyor (two clicks; settings are kept)") },
    { glyph: "\uf068", description: root.tr("Compact mode: only the main buttons") },
    { glyph: "\uf00d", description: root.tr("Quit (two clicks)") }
  ]
  readonly property var hotkeys: [
    { key: "Super+F9", description: root.tr("Hold to record a microphone question; release to send it to AI") },
    { key: "Super+Shift+F9", description: root.tr("Hold to record system audio; release to send it to AI") },
    { key: "F9", description: root.tr("Ask AI now (streams the answer into a tile)") },
    { key: "Shift+F9", description: root.tr("Ask AI escalated to the smart cloud model (one-shot; tile shows a cloud badge)") },
    { key: "F3", description: root.tr("Re-ask the last question with fresh context") },
    { key: "F6", description: root.tr("Manual tile from the last transcript line") },
    { key: "F8", description: root.tr("Screenshot → AI explains it") },
    { key: "Shift+F8", description: root.tr("Screenshot → translate only (no screen description)") },
    { key: "Ctrl+F8", description: root.tr("Screenshot → OCR the text, then read it aloud") },
    { key: "Shift+Alt+1", description: root.tr("Read the selected text aloud") },
    { key: "Shift+Alt+2", description: root.tr("OCR a screen region and read it aloud") },
    { key: "Shift+Alt+3", description: root.tr("Pause / resume reading aloud") },
    { key: "F4", description: root.tr("Knowledge-base palette (open / close)") },
    { key: "F1", description: root.tr("Help (open / close)") },
    { key: "F7", description: root.tr("Session archive (open / close)") }
  ]
  // No window properties. drag-start-requested / drag-moved belong to AuxLayer.
  Small.Surface {
    anchors.fill: parent
    view: root
    closeCallback: "cancelled"
    title: root.tr("Help")
    Controls.ScrollView {
      id: viewport
      anchors.fill: parent
      contentWidth: availableWidth
      Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
      ColumnLayout {
        width: Math.max(0, viewport.availableWidth - Commons.Style.spacing.lg)
        spacing: Commons.Style.spacing.sm
        // On Omarchy Suflyor lives in the bar: the widget replaces the floating bar,
        // so its grip and hide-to-tray rows of help.slint do not apply here.
        Tile.Label { Layout.fillWidth: true; text: root.tr("Suflyor in the Omarchy bar"); font.bold: true; color: Commons.Color.accent; }
        Repeater {
          model: [
            { description: root.tr("Click: open the Suflyor panel") },
            { description: root.tr("Right click: start or stop the session") },
            { description: root.tr("Middle click: write a question") },
          ]
          delegate: Small.ReferenceRow {
            required property var modelData
            Layout.fillWidth: true
            brandMark: true
            description: modelData.description
          }
        }
        Item { implicitHeight: Commons.Style.spacing.lg }
        Tile.Label { Layout.fillWidth: true; text: root.tr("How it works"); font.bold: true; color: Commons.Color.accent; }
        Tile.Label { Layout.fillWidth: true; text: root.tr("1) Start capture — press start on the bar. The mic records you; system audio records the other side."); }
        Tile.Label { Layout.fillWidth: true; text: root.tr("2) Get help — hold ask to ask by voice, hold grab to send the other side's last words to AI, or press F9. Answers appear in tiles."); }
        Tile.Label { Layout.fillWidth: true; text: root.tr("3) Stop and review — click the timer with the stop icon ('Stop session'), then open the session archive (F7) for transcripts and summaries."); }
        Item { implicitHeight: Commons.Style.spacing.lg }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Bar icons"); font.bold: true; color: Commons.Color.accent; }
        Small.ReferenceRow { Layout.fillWidth: true; header: true; glyph: root.tr("Icon"); description: root.tr("What it does") }
        Repeater {
          model: root.icons
          delegate: Small.ReferenceRow {
            required property var modelData
            Layout.fillWidth: true
            glyph: modelData.glyph
            description: modelData.description
          }
        }
        Item { implicitHeight: Commons.Style.spacing.lg }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Hotkeys"); color: Commons.Color.accent; font.bold: true }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Super is the Windows key. Hold the shortcut to record, then release it to ask. If another application uses a shortcut, Suflyor leaves it unchanged; see Diagnostics.") }
        Small.ReferenceRow { Layout.fillWidth: true; header: true; glyph: root.tr("Key"); description: root.tr("Action") }
        Repeater {
          model: root.hotkeys
          delegate: Small.ReferenceRow {
            required property var modelData
            Layout.fillWidth: true
            keyRow: true
            glyph: root.keyLabel(modelData.key)
            description: modelData.description
          }
        }
        Item { implicitHeight: Commons.Style.spacing.lg }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Record gestures"); color: Commons.Color.accent; font.bold: true; }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Hold a record button to record, release to send. While holding, swipe up to lock; releasing keeps recording. Click stop to send."); }
        Item { implicitHeight: Commons.Style.spacing.lg }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Local model and vision"); color: Commons.Color.accent; font.bold: true; }
        Tile.Label { Layout.fillWidth: true; text: root.tr("The answer model is chosen in Settings → AI providers, the screenshot model in Settings → Screenshots (F8). Speech recognition is set up in Settings → STT."); }
        Item { implicitHeight: Commons.Style.spacing.lg }
        Tile.Label { Layout.fillWidth: true; text: root.tr("Read-aloud · recognition · compact mode"); color: Commons.Color.accent; font.bold: true; }
        Tile.Label {
          Layout.fillWidth: true
          text: root.mac ? root.tr("Shift+Option+1 — read the selected text aloud; Shift+Option+2 — recognize a screen region with Apple Vision and read it; Shift+Option+3 — pause/resume. Voices are configured in Settings → TTS; Apple Vision is built into macOS. Compact reader mode uses the collapse chip on the bar. Listening mode in Settings → Auto-tiles mutes automatic tiles.")
            : root.tr("Shift+Alt+1 — read the selected text aloud; Shift+Alt+2 — recognize text in a screen region and read it; Shift+Alt+3 — pause or resume. Voices are set up in Settings → TTS, text recognition is installed in Settings → Components. Compact mode keeps only the main buttons on the bar. Listening mode (the lock on the bar or Settings → Auto-tiles) keeps recording without opening tiles.")
        }
      }
    }
  }
}
