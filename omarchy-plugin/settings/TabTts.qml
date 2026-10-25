pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import "controls" as C

// SettingsWindow tab 5. Installation and test results remain host-owned.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property int engine: win ? win.prop("tts-engine-index", 0) : 0
  readonly property bool available: win ? win.prop("tts-available", false) : false
  readonly property bool installing: win ? win.prop("tts-installing", false) : false
  readonly property int phase: win ? win.prop("tts-install-phase", 0) : 0
  readonly property string installLabel: win ? win.prop("tts-install-label", "") : ""
  readonly property bool teraInstalling: win ? win.prop("tera-installing", false) : false
  readonly property int teraPhase: win ? win.prop("tera-install-phase", 0) : 0
  readonly property string teraLabel: win ? win.prop("tera-install-label", "") : ""
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Read aloud", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Read selected text (Shift+Alt+1) or a screen region (Shift+Alt+2) aloud. Pause or resume with Shift+Alt+3.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Engine:", []) : ""
      C.SettingsComboBox {
        model: root.win ? root.win.prop("tts-engine-names", []) : []
        currentIndex: root.engine
        onSelected: function(index) {
          if (root.win && root.service) root.service.client.call(root.win.winId, "tts-engine-changed", [index])
        }
      }
    }
    C.WrapBlock {
      visible: root.engine === 0
      spacing: Style.space(6)
      C.SettingsRow {
        visible: root.available
        label: root.service ? root.service.tr("Voice:", []) : ""
        C.SettingsComboBox {
          model: root.win ? root.win.prop("tts-voice-names", []) : []
          currentIndex: root.win ? root.win.prop("tts-voice-index", 0) : 0
          onSelected: function(index) {
            if (root.win && root.service) root.service.client.call(root.win.winId, "tts-voice-changed", [index])
          }
        }
      }
      C.WrapBlock {
        visible: !root.available
        spacing: Style.space(6)
        C.SettingsCaption {
          text: root.service ? root.service.tr("Neural voices aren't installed yet — download them (Irina + Ruslan, ~130 MB).", []) : ""
          font.pixelSize: Style.font.body
        }
        C.SettingsButton {
          text: root.service ? root.service.tr(root.installing ? "Installing…" : "Install voices", []) : ""
          enabled: !root.installing
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "tts-install-clicked", [])
        }
      }
      C.StatusLine {
        visible: root.phase !== 0
        text: {
          if (!root.service) return ""
          if (root.phase === 1) return root.service.tr("Preparing…", [])
          if (root.phase === 2) return root.service.tr("Downloading: {}…", [root.installLabel])
          if (root.phase === 3) return root.service.tr("Verifying: {}…", [root.installLabel])
          if (root.phase === 4) return root.service.tr("Unpacking: {}…", [root.installLabel])
          if (root.phase === 5) return root.service.tr("{} — already installed", [root.installLabel])
          if (root.phase === 6) return root.service.tr("Voices installed", [])
          if (root.phase === 7) return root.service.tr("Installed partially; failed: {}", [root.installLabel])
          if (root.phase === 9) return root.service.tr("{} — download failed (retry later)", [root.installLabel])
          return root.service.tr("Couldn't install the voices — check the connection and retry.", [])
        }
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        visible: root.phase === 7 || root.phase === 9 || root.phase === 8 || root.phase < 0 || root.phase > 9
        // Existing msgid supplies the action missing from the phase's cause text.
        text: root.service ? root.service.tr("Install voices", []) : ""
      }
    }
    C.WrapBlock {
      visible: root.engine === 1
      spacing: Style.space(6)
      C.SettingsCaption {
        text: root.service ? root.service.tr("Experimental neural voice TeraTTS v2 (RU and EN). Downloads about 370 MB on demand. Until it is installed, reading uses Piper; speaker detection always uses Piper.", []) : ""
        font.pixelSize: Style.font.body
      }
      C.StatusLine {
        text: root.win ? root.win.prop("tera-model-status", "") : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsRow {
        visible: root.available
        label: root.service ? root.service.tr("Voice:", []) : ""
        C.SettingsComboBox {
          model: root.win ? root.win.prop("tts-voice-names", []) : []
          currentIndex: root.win ? root.win.prop("tts-voice-index", 0) : 0
          onSelected: function(index) {
            if (root.win && root.service) root.service.client.call(root.win.winId, "tts-voice-changed", [index])
          }
        }
      }
      C.SettingsButtonsRow {
        visible: !root.available || root.teraInstalling
        C.SettingsButton {
          text: root.service ? root.service.tr(root.teraInstalling ? "Installing…" : "Install model", []) : ""
          enabled: !root.teraInstalling && !root.available
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "tera-install-clicked", [])
        }
        C.SettingsButton {
          visible: root.teraInstalling
          text: root.service ? root.service.tr("Cancel", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "tera-install-cancel-clicked", [])
        }
      }
      C.StatusLine {
        visible: root.teraPhase !== 0
        text: {
          if (!root.service) return ""
          if (root.teraPhase === 1) return root.service.tr("Preparing…", [])
          if (root.teraPhase === 2) return root.service.tr("Downloading: {}…", [root.teraLabel])
          if (root.teraPhase === 3) return root.service.tr("Verifying: {}…", [root.teraLabel])
          if (root.teraPhase === 4) return root.service.tr("Publishing…", [])
          if (root.teraPhase === 6) return root.service.tr("Model installed", [])
          if (root.teraPhase === 8) return root.service.tr("Install cancelled", [])
          return root.service.tr("Couldn't install the model — check the connection and retry.", [])
        }
        font.pixelSize: Style.font.body
      }
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Speed:", []) : ""
      C.SettingsComboBox {
        // These are literal multiplier labels in Slint, not catalog msgids.
        model: root.service ? [root.service.tr("0.75×", []), root.service.tr("1.0×", []), root.service.tr("1.3×", []), root.service.tr("1.5×", []), root.service.tr("2.0×", [])] : []
        currentIndex: root.win ? root.win.prop("tts-rate-index", 1) : 1
        onSelected: function(index) {
          if (root.win && root.service) root.service.client.call(root.win.winId, "tts-rate-changed", [index])
        }
      }
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Test voice", []) : ""
      enabled: root.available
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "tts-test-clicked", [])
    }
    C.StatusLine {
      text: root.win ? root.win.prop("tts-test-status", "") : ""
      tone: "danger"
      font.pixelSize: Style.font.body
    }
  }
}
