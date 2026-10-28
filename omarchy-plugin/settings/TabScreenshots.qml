pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import "controls" as C

// SettingsWindow tab 17, Linux. builtin-busy is private Slint state, derived here.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property int provider: win ? win.prop("vision-provider-index", 3) : 3
  readonly property bool sameAvailable: win ? win.prop("vision-same-available", false) : false
  readonly property bool installable: win ? win.prop("strata-vision-installable", false) : false
  readonly property bool downloaded: win ? win.prop("strata-vision-downloaded", false) : false
  readonly property bool visionEnabled: win ? win.prop("strata-vision-enabled", false) : false
  readonly property bool strataActive: win ? win.prop("strata-active", false) : false
  readonly property int aiProvider: win ? win.prop("ai-provider-index", 0) : 0
  readonly property int visionJob: win ? win.prop("strata-vision-job", 0) : 0
  readonly property bool strataBusy: win ? win.prop("strata-busy", false) : false
  readonly property bool builtinBusy: win ? (win.prop("local-ai-installing", false)
    || win.prop("quality-downloading", false) || win.prop("model-switching", false)
    || win.prop("vision12b-downloading", false) || win.prop("engine-updating", false)) : false
  readonly property bool authBusy: win ? win.prop("codex-auth-busy", false) : false
  readonly property bool modelsBusy: win ? win.prop("codex-models-busy", false) : false
  readonly property string loginUrl: win ? win.prop("codex-login-url", "") : ""
  readonly property string userCode: win ? win.prop("codex-user-code", "") : ""
  readonly property var visionModels: win ? win.prop("codex-vision-model-labels", []) : []
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Which model looks at F8 screenshots. It is separate from the answer model: a local model can keep answering while images go to a model that understands them.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsCard {
      title: root.service ? root.service.tr("Add Vision to Strata", []) : ""
      C.SettingsCaption {
        text: root.service ? root.service.tr("Optional 907 MB encoder for Qwen3.8-Flash-Next. CUDA builds prefer the GPU when supported, with CPU fallback. No second language model. Downloading does not change F8.", []) : ""
      }
      C.StatusLine {
        text: root.service ? root.service.tr(!root.installable
          ? "First install or select a compatible Strata folder in AI providers. The image helper must be present."
          : root.visionEnabled && root.provider === 1 && root.aiProvider === 1 && root.strataActive ? "Enabled for F8"
          : root.downloaded ? "Downloaded. First use Strata for text, then enable F8."
          : "Not downloaded", []) : ""
        tone: "muted"
      }
      C.StatusLine {
        text: root.win ? root.win.prop("strata-vision-status", "") : ""
      }
      C.SuflyorDownloadProgress {
        visible: root.visionJob === 1
        progress: root.win ? root.win.prop("strata-vision-progress", 0) : 0
      }
      C.SettingsButtonsRow {
        C.SettingsButton {
          text: root.service ? root.service.tr("Download / Resume", []) : ""
          enabled: root.installable && !root.downloaded && root.visionJob === 0 && !root.strataBusy && !root.builtinBusy
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "strata-vision-download", [])
        }
        C.SettingsButton {
          visible: root.visionJob !== 0
          text: root.service ? root.service.tr("Cancel", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "cancel-local-ai-clicked", [])
        }
        C.SettingsButton {
          text: root.service ? root.service.tr("Enable for F8", []) : ""
          enabled: root.installable && root.downloaded && (!root.visionEnabled || root.provider !== 1)
            && root.strataActive && root.aiProvider === 1 && root.visionJob === 0 && !root.strataBusy && !root.builtinBusy
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "strata-vision-enable", [])
        }
      }
    }
    C.SettingsComboBox {
      model: root.service ? [root.service.tr("Off", []),
        root.service.tr(root.sameAvailable ? "Same as the answer model" : "Same as the answer model (unavailable)", []),
        root.service.tr("Your server or bridge (OpenAI-compatible)", []), root.service.tr("Model on this PC", []),
        root.service.tr("OpenAI API (Responses)", []), root.service.tr("Anthropic API (Messages)", []),
        root.service.tr("ChatGPT via Codex (experimental)", [])] : []
      currentIndex: root.provider
      onSelected: function(index) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "vision-provider-index", index)
        root.service.client.call(root.win.winId, "vision-provider-changed", [index])
      }
    }
    C.StatusLine {
      visible: !root.sameAvailable && root.provider === 1
      text: root.service ? root.service.tr("The 'Same as the answer model' option is unavailable because the selected answer model does not declare image input. Choose another screenshot provider or model.", []) : ""
      tone: "warning"
    }
    C.WrapBlock {
      visible: root.provider === 2
      C.StatusLine {
        text: root.service ? root.service.tr("Screenshots go to this server. Blank fields reuse the answers bridge.", []) : ""
        tone: "warning"
      }
      C.SettingsRow {
        label: root.service ? root.service.tr("Server address", []) : ""
        C.SettingsLineEdit {
          placeholderText: root.service ? root.service.tr("blank = same as the answers bridge", []) : ""
          text: root.win ? root.win.prop("vision-base-url-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "vision-base-url-input", text)
        }
      }
      C.SettingsRow {
        label: root.service ? root.service.tr("API key", []) : ""
        C.SettingsLineEdit {
          password: true
          placeholderText: root.service ? root.service.tr("type a new key to replace it", []) : ""
          text: root.win ? root.win.prop("vision-bearer-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "vision-bearer-input", text)
        }
      }
      C.SettingsRow {
        label: root.service ? root.service.tr("Model name", []) : ""
        C.SettingsLineEdit {
          placeholderText: root.service ? root.service.tr("blank = claude-sonnet-4-6", []) : ""
          text: root.win ? root.win.prop("vision-model-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "vision-model-input", text)
        }
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Save", []) : ""
        onClicked: {
          if (!root.win || !root.service) return
          root.service.client.call(root.win.winId, "vision-base-url-save", [root.win.prop("vision-base-url-input", "")])
          root.service.client.call(root.win.winId, "vision-model-save", [root.win.prop("vision-model-input", "")])
          var key = root.win.prop("vision-bearer-input", "")
          if (key !== "") {
            root.service.client.call(root.win.winId, "vision-bearer-save", [key])
            root.service.client.set(root.win.winId, "vision-bearer-input", "")
          }
        }
      }
    }
    C.WrapBlock {
      visible: root.provider === 3
      C.SettingsCaption {
        text: root.service ? root.service.tr("A second server on this PC with a model that understands images (e.g. Qwen2-VL). Blank fields reuse the answers server.", []) : ""
      }
      C.SettingsRow {
        label: root.service ? root.service.tr("Server address", []) : ""
        C.SettingsLineEdit {
          // Literal URL in Slint; pass through the same visible-text route.
          placeholderText: root.service ? root.service.tr("http://127.0.0.1:8082/v1", []) : ""
          text: root.win ? root.win.prop("vision-local-base-url-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "vision-local-base-url-input", text)
        }
      }
      C.SettingsRow {
        label: root.service ? root.service.tr("Model name", []) : ""
        C.SettingsLineEdit {
          placeholderText: root.service ? root.service.tr("e.g. qwen2-vl", []) : ""
          text: root.win ? root.win.prop("vision-local-model-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "vision-local-model-input", text)
        }
      }
      C.SettingsRow {
        label: root.service ? root.service.tr("API key", []) : ""
        C.SettingsLineEdit {
          password: true
          placeholderText: root.service ? root.service.tr("optional", []) : ""
          text: root.win ? root.win.prop("vision-local-bearer-input", "") : ""
          onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "vision-local-bearer-input", text)
        }
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Save", []) : ""
        onClicked: {
          if (!root.win || !root.service) return
          root.service.client.call(root.win.winId, "vision-local-base-url-save", [root.win.prop("vision-local-base-url-input", "")])
          root.service.client.call(root.win.winId, "vision-local-model-save", [root.win.prop("vision-local-model-input", "")])
          var key = root.win.prop("vision-local-bearer-input", "")
          if (key !== "") {
            root.service.client.call(root.win.winId, "vision-local-bearer-save", [key])
            root.service.client.set(root.win.winId, "vision-local-bearer-input", "")
          }
        }
      }
    }
    C.SettingsCaption {
      visible: root.provider === 4
      text: root.service ? root.service.tr("Screenshots use the OpenAI API connection and model set in AI providers. The answer and screenshot models stay independent.", []) : ""
    }
    C.SettingsCaption {
      visible: root.provider === 5
      text: root.service ? root.service.tr("Screenshots use the Anthropic API connection and model set in AI providers. The answer and screenshot models stay independent.", []) : ""
    }
    C.WrapBlock {
      visible: root.provider === 6
      C.SettingsCaption {
        text: root.service ? root.service.tr("Choose a separate Codex model for screenshots. Only account models that advertise image input are shown.", []) : ""
      }
      C.StatusLine {
        text: root.win ? root.win.prop("codex-auth-status", "") : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsButtonsRow {
        C.SettingsButton {
          text: root.service ? root.service.tr(root.authBusy ? "Connecting..." : "Connect ChatGPT", []) : ""
          enabled: !root.authBusy && !root.modelsBusy
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "codex-connect-clicked", [])
        }
        C.SettingsButton {
          text: root.service ? root.service.tr("Disconnect", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "codex-disconnect-clicked", [])
        }
      }
      C.WrapBlock {
        visible: root.loginUrl !== "" && root.userCode !== ""
        C.StatusLine {
          text: root.service ? root.service.tr("Code:", []) + " " + root.userCode : ""
          tone: "accent"
          font.pixelSize: Style.font.body
          font.bold: true
        }
        C.SettingsButtonsRow {
          C.SettingsButton {
            text: root.service ? root.service.tr("Open sign-in page", []) : ""
            onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "codex-open-signin-clicked", [root.loginUrl])
          }
          C.SettingsButton {
            text: root.service ? root.service.tr("Copy code", []) : ""
            Accessible.name: root.service ? root.service.tr("Copy one-time code", []) : ""
            enabled: root.userCode !== ""
            onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "codex-copy-code-clicked", [root.userCode])
          }
        }
      }
      C.StatusLine {
        text: root.win ? root.win.prop("codex-copy-status", "") : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsFieldRow {
        C.SettingsComboBox {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          model: root.visionModels
          currentIndex: root.win ? root.win.prop("codex-vision-model-index", -1) : -1
          enabled: !root.authBusy && !root.modelsBusy && root.visionModels.length > 0
          onSelected: function(index) {
            if (!root.win || !root.service) return
            root.service.client.set(root.win.winId, "codex-vision-model-index", index)
            root.service.client.call(root.win.winId, "codex-vision-model-selected", [index])
          }
        }
        C.SettingsButton {
          compact: true
          text: root.service ? root.service.tr(root.modelsBusy ? "loading..." : "refresh", []) : ""
          enabled: !root.authBusy && !root.modelsBusy
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "codex-models-refresh", [])
        }
      }
      C.SettingsCaption {
        visible: !root.modelsBusy && root.visionModels.length === 0
        text: root.service ? root.service.tr("No image-capable Codex account models are available. Sign in or refresh.", []) : ""
      }
    }
    C.WrapBlock {
      visible: root.provider >= 1
      C.SettingsButton {
        text: root.service ? root.service.tr("Test screenshot model", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "vision-test-clicked", [])
      }
      C.StatusLine {
        text: root.win ? root.win.prop("vision-test-result", "") : ""
        tone: "accent"
        font.pixelSize: Style.font.body
      }
    }
  }
  C.SettingsCard {
    visible: root.provider >= 1
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("F8 options", []) : ""
    C.DarkCheck {
      text: root.service ? root.service.tr("Practice: answer and a short review", []) : ""
      checked: root.win ? root.win.prop("vision-test-practice", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "vision-test-practice", checked)
        root.service.client.call(root.win.winId, "vision-test-practice-changed", [checked])
      }
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Phonetics in F8 translate mode (IPA, e.g. schedule [ˈʃedjuːl])", []) : ""
      checked: root.win ? root.win.prop("vision-phonetics", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "vision-phonetics", checked)
        root.service.client.call(root.win.winId, "vision-phonetics-changed", [checked])
      }
    }
  }
}
