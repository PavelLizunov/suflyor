pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import "controls" as C

// SettingsWindow tab 12, Linux: GigaAM uses CPU; Whisper is a configured server.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property int provider: win ? win.prop("stt-provider-index", 0) : 0
  readonly property bool installing: win ? win.prop("stt-gigaam-installing", false) : false
  readonly property bool failed: win ? win.prop("stt-gigaam-install-failed", false) : false
  readonly property bool installed: win ? win.prop("stt-gigaam-installed", false) : false
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Speech-to-text engine. Local options run on this PC (no cloud, no Groq key needed).", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsComboBox {
      model: root.service ? [root.service.tr("Cloud (Groq Whisper)", []), root.service.tr("Local GigaAM-v3 (Russian)", []), root.service.tr("Local Whisper (whisper.cpp)", [])] : []
      currentIndex: root.provider
      onSelected: function(index) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "stt-provider-index", index)
        root.service.client.call(root.win.winId, "stt-provider-changed", [index])
      }
    }
    C.SettingsCaption {
      visible: root.provider >= 0 && root.provider <= 2
      text: root.service ? root.service.tr(root.provider === 0
        ? "Cloud: audio is sent to Groq's API for recognition. Requires a Groq API key. Supports many languages."
        : root.provider === 1 ? "Local: runs entirely on this PC, no cloud. Russian only. Requires the GigaAM model folder set below."
        : "Local: runs on your whisper-server. Multilingual. Configure the server URL and model below.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Recognition language — Auto detects per phrase (Whisper/Groq). GigaAM is Russian-only.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsComboBox {
      model: root.service ? [root.service.tr("Auto-detect", []), root.service.tr("Russian", []), root.service.tr("English", [])] : []
      currentIndex: root.win ? root.win.prop("stt-language-index", 0) : 0
      onSelected: function(index) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "stt-language-index", index)
        root.service.client.call(root.win.winId, "stt-language-changed", [index])
      }
    }
    C.WrapBlock {
      visible: root.provider === 0
      spacing: Style.space(6)
      C.SettingsCaption {
        text: root.service ? root.service.tr("Cloud recognition model", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsComboBox {
        model: root.service ? [root.service.tr("Fast — recommended (whisper-large-v3-turbo)", []), root.service.tr("More accurate — slower (whisper-large-v3)", [])] : []
        currentIndex: root.win ? root.win.prop("stt-cloud-model-index", 0) : 0
        onSelected: function(index) {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "stt-cloud-model-index", index)
          root.service.client.call(root.win.winId, "stt-cloud-model-changed", [index])
        }
      }
    }
    C.WrapBlock {
      visible: root.provider === 1
      spacing: Style.space(6)
      C.SettingsFieldRow {
        C.SettingsLineEdit {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          placeholderText: root.service ? root.service.tr("GigaAM model dir — folder with model.int8.onnx + vocab.txt", []) : ""
          text: root.win ? root.win.prop("stt-gigaam-dir-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "stt-gigaam-dir-input", text)
        }
        C.SettingsButton {
          compact: true
          iconName: "check"
          text: root.service ? root.service.tr("Save", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "stt-gigaam-dir-save", [root.win.prop("stt-gigaam-dir-input", "")])
        }
      }
      C.StatusLine {
        text: {
          if (!root.service) return ""
          if (root.installing) return root.service.tr("Downloading and verifying…", [])
          if (root.failed) return root.service.tr("Download or activation failed. Retry when ready.", [])
            + " (" + root.service.tr(root.installed ? "Check files" : "Download / Resume", []) + ")"
          if (root.installed) return root.service.tr("Installed", [])
          if (root.win && root.win.prop("stt-gigaam-install-cancelled", false))
            return root.service.tr("Download cancelled. Resume keeps verified files.", [])
          return root.service.tr("Not installed", [])
        }
        tone: root.failed ? "danger" : root.installed ? "good" : "neutral"
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        visible: root.installing
        text: root.service ? root.service.tr("{} / {} MB downloaded", [root.win ? root.win.prop("stt-gigaam-install-done", "0") : "0", root.win ? root.win.prop("stt-gigaam-install-total", "214") : "214"]) : ""
      }
      C.SuflyorDownloadProgress {
        visible: root.installing
        progress: root.win ? root.win.prop("stt-gigaam-install-progress", 0) : 0
      }
      C.SettingsButtonsRow {
        C.SettingsButton {
          text: root.service ? root.service.tr(root.installed ? "Check files" : "Download / Resume", []) : ""
          enabled: !root.installing && !(root.installed && root.win && root.win.prop("stt-gigaam-custom", false))
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "stt-gigaam-install", [])
        }
        C.SettingsButton {
          visible: root.installing
          text: root.service ? root.service.tr("Cancel", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "stt-gigaam-install-cancel", [])
        }
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("GigaAM uses the native ONNX Runtime CPU backend on Linux.", []) : ""
      }
    }
    C.WrapBlock {
      visible: root.provider === 2
      spacing: Style.space(6)
      C.SettingsFieldRow {
        C.SettingsLineEdit {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          placeholderText: root.service ? root.service.tr("whisper-server URL — http://127.0.0.1:8081/v1", []) : ""
          text: root.win ? root.win.prop("stt-whisper-url-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "stt-whisper-url-input", text)
        }
        C.SettingsButton {
          compact: true
          iconName: "check"
          text: root.service ? root.service.tr("Save", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "stt-whisper-url-save", [root.win.prop("stt-whisper-url-input", "")])
        }
      }
      C.SettingsFieldRow {
        C.SettingsLineEdit {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          password: true
          placeholderText: root.service ? root.service.tr("whisper-server bearer token (optional)", []) : ""
          text: root.win ? root.win.prop("stt-whisper-bearer-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "stt-whisper-bearer-input", text)
        }
        C.SettingsButton {
          compact: true
          iconName: "check"
          text: root.service ? root.service.tr("Save", []) : ""
          onClicked: {
            if (!root.win || !root.service) return
            root.service.client.call(root.win.winId, "stt-whisper-bearer-save", [root.win.prop("stt-whisper-bearer-input", "")])
            root.service.client.set(root.win.winId, "stt-whisper-bearer-input", "")
          }
        }
      }
      C.SettingsFieldRow {
        C.SettingsLineEdit {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          placeholderText: root.service ? root.service.tr("Server model id (advanced) — e.g. whisper-large-v3-turbo", []) : ""
          text: root.win ? root.win.prop("stt-whisper-model-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "stt-whisper-model-input", text)
        }
        C.SettingsButton {
          compact: true
          iconName: "check"
          text: root.service ? root.service.tr("Save", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "stt-whisper-model-save", [root.win.prop("stt-whisper-model-input", "")])
        }
      }
    }
    C.WrapBlock {
      visible: root.provider === 0
      spacing: Style.space(6)
      C.SettingsSeparator {}
      C.StatusLine {
        text: root.win ? root.win.prop("groq-api-key-status", root.service ? root.service.tr("(loading...)", []) : "") : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsFieldRow {
        C.SettingsLineEdit {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          password: true
          placeholderText: root.service ? root.service.tr("paste new Groq key then Save", []) : ""
          text: root.win ? root.win.prop("groq-api-key-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "groq-api-key-input", text)
        }
        C.SettingsButton {
          compact: true
          iconName: "check"
          text: root.service ? root.service.tr("Save", []) : ""
          onClicked: {
            if (!root.win || !root.service) return
            root.service.client.call(root.win.winId, "groq-api-key-save", [root.win.prop("groq-api-key-input", "")])
            root.service.client.set(root.win.winId, "groq-api-key-input", "")
          }
        }
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Get Groq API Key (console.groq.com)", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "open-groq-keys-clicked", [])
      }
    }
    C.SettingsSeparator {}
    C.SettingsButton {
      iconName: "diag"
      text: root.service ? root.service.tr("Test STT connection", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "stt-test-clicked", [])
    }
    C.StatusLine {
      text: root.win ? root.win.prop("stt-test-result", "") : ""
      tone: "accent"
      font.pixelSize: Style.font.body
    }
  }
}
