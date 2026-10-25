pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import "controls" as C

// SettingsWindow tab 6. Linux uses index-based PipeWire device selection.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property bool loading: win ? win.prop("audio-devices-loading", false) : false
  readonly property bool failed: win ? win.prop("audio-devices-failed", false) : false
  readonly property bool devicesReady: !loading && !failed
  readonly property bool micMissing: win ? win.prop("mic-device-missing", false) : false
  readonly property bool micEmpty: win ? win.prop("mic-devices-empty", false) : false
  readonly property int saveState: win ? win.prop("audio-save-state", 0) : 0
  readonly property int retentionMode: win ? win.prop("retention-mode", 1) : 1
  readonly property string retentionValue: win ? win.prop("retention-value", "10") : "10"
  readonly property string journalKeep: win ? win.prop("journal-keep-value", "100") : "100"
  readonly property string journalMb: win ? win.prop("journal-mb-value", "500") : "500"
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Microphone", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Pick the microphone. The list comes from PipeWire.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsFieldRow {
      C.SettingsComboBox {
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        Accessible.name: root.service ? root.service.tr("Microphone device", []) : ""
        model: root.win ? root.win.prop("mic-devices", []) : []
        currentIndex: root.win ? root.win.prop("mic-device-index", 0) : 0
        enabled: root.devicesReady
        onSelected: function(index) {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "mic-device-index", index)
          root.service.client.call(root.win.winId, "mic-device-index-selected", [index])
        }
      }
      C.SettingsButton {
        compact: true
        text: root.service ? root.service.tr(root.loading ? "Loading audio devices..." : "Refresh devices", []) : ""
        enabled: !root.loading
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "audio-devices-refresh", [])
      }
    }
    C.WrapBlock {
      spacing: Style.space(6)
      C.StatusLine {
        visible: root.failed
        text: root.service ? root.service.tr("Could not load audio devices. Your saved choices are unchanged. Try Refresh devices.", []) : ""
        tone: "warning"
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Changes apply at the next capture start; restart an active session.", []) : ""
        font.pixelSize: Style.font.body
      }
      C.StatusLine {
        visible: root.saveState !== 0
        text: root.service ? root.service.tr(root.saveState === 1 ? "Saved for the next capture." : "Could not save the device. The previous choice is unchanged.", []) : ""
        tone: root.saveState === 1 ? "accent" : "warning"
        font.pixelSize: Style.font.body
      }
    }
    C.StatusLine {
      visible: root.devicesReady && root.micMissing
      text: root.service ? root.service.tr("Saved device is unavailable. Choose another device or system default.", []) : ""
      tone: "warning"
      font.pixelSize: Style.font.body
    }
    C.StatusLine {
      visible: root.devicesReady && root.micEmpty
      text: root.service ? root.service.tr("No microphone devices found.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Test microphone (3s record)", []) : ""
      enabled: root.devicesReady && !root.micMissing && !root.micEmpty
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "mic-test-clicked", [])
    }
    C.StatusLine {
      text: root.win ? root.win.prop("mic-test-result", "") : ""
      tone: "accent"
      font.pixelSize: Style.font.body
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("System audio - headphones / speakers", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Capture the selected PipeWire output monitor or input. This does not change system or read-aloud playback.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsComboBox {
      Accessible.name: root.service ? root.service.tr("System audio device", []) : ""
      model: root.win ? root.win.prop("system-devices", []) : []
      currentIndex: root.win ? root.win.prop("system-device-index", 0) : 0
      enabled: root.devicesReady
      onSelected: function(index) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "system-device-index", index)
        root.service.client.call(root.win.winId, "system-device-selected", [index])
      }
    }
    C.StatusLine {
      visible: root.devicesReady && (root.win ? root.win.prop("system-device-missing", false) : false)
      text: root.service ? root.service.tr("Saved device is unavailable. Choose another device or system default.", []) : ""
      tone: "warning"
      font.pixelSize: Style.font.body
    }
    C.StatusLine {
      visible: root.devicesReady && (root.win ? root.win.prop("system-devices-empty", false) : false)
      text: root.service ? root.service.tr("No system audio devices found.", []) : ""
      font.pixelSize: Style.font.body
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Storage", []) : ""
    C.DarkCheck {
      text: root.service ? root.service.tr("Record call audio (mic + system)", []) : ""
      checked: root.win ? root.win.prop("record-audio", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "record-audio", checked)
        root.service.client.call(root.win.winId, "record-audio-changed", [checked])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Saved locally as separate mic + system WAVs per session — nothing is uploaded. ~230 MB/hour. Used when you click 'Generate' or the regenerate icon in the session archive.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Keep recordings:", []) : ""
      RowLayout {
        width: parent.width
        spacing: Style.space(8)
        C.SettingsComboBox {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          model: root.service ? [root.service.tr("all (no limit)", []), root.service.tr("last N sessions", []), root.service.tr("last N days", [])] : []
          currentIndex: root.retentionMode
          onSelected: function(index) {
            if (!root.win || !root.service) return
            root.service.client.set(root.win.winId, "retention-mode", index)
            root.service.client.call(root.win.winId, "retention-changed", [index, root.retentionValue])
          }
        }
        C.SettingsLineEdit {
          visible: root.retentionMode !== 0
          Layout.preferredWidth: Style.space(64)
          Layout.minimumWidth: 0
          text: root.retentionValue
          onTextEdited: {
            if (!root.win || !root.service) return
            root.service.client.set(root.win.winId, "retention-value", text)
            root.service.client.call(root.win.winId, "retention-changed", [root.retentionMode, text])
          }
          onAccepted: if (root.win && root.service) root.service.client.call(root.win.winId, "retention-changed", [root.retentionMode, text])
        }
      }
    }
    C.StatusLine {
      visible: root.retentionMode === 0
      text: root.service ? root.service.tr("Unlimited recordings can take tens of GB per month with daily calls.", []) : ""
      tone: "warning"
      font.pixelSize: Style.font.body
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Open recordings folder", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "open-recordings-clicked", [])
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Logs store transcripts and AI answers for the archive (F7). The disk cap removes the oldest logs first.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Keep logs (0 = no limit):", []) : ""
      C.SettingsLineEdit {
        width: Math.min(parent.width, Style.space(64))
        text: root.journalKeep
        onTextEdited: {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "journal-keep-value", text)
          root.service.client.call(root.win.winId, "journal-retention-changed", [text, root.journalMb])
        }
        onAccepted: if (root.win && root.service) root.service.client.call(root.win.winId, "journal-retention-changed", [text, root.journalMb])
      }
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Logs disk cap, MB (0 = no limit):", []) : ""
      C.SettingsLineEdit {
        width: Math.min(parent.width, Style.space(64))
        text: root.journalMb
        onTextEdited: {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "journal-mb-value", text)
          root.service.client.call(root.win.winId, "journal-retention-changed", [root.journalKeep, text])
        }
        onAccepted: if (root.win && root.service) root.service.client.call(root.win.winId, "journal-retention-changed", [root.journalKeep, text])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Retention changes apply at the next session start.", []) : ""
      font.pixelSize: Style.font.body
    }
  }
}
