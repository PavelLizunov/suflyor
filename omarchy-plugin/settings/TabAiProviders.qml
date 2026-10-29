pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import "controls" as C

// SettingsWindow tab 11, Linux branches. The frame, not this tab, owns scrolling.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property int provider: p("ai-provider-index", 0)
  readonly property bool llamaInstalled: p("llama-installed", false)
  readonly property bool managed: p("managed-local-server", false)
  readonly property bool customActive: p("ai-local-custom-active", false)
  readonly property int modelProfile: p("ai-local-model-profile-index", 1)
  readonly property bool legacyPresent: p("legacy-model-present", false)
  readonly property bool fallbackPresent: p("fallback-model-present", false)
  readonly property bool qualityPresent: p("quality-model-present", false)
  readonly property bool installing: p("local-ai-installing", false)
  readonly property bool qualityDownloading: p("quality-downloading", false)
  readonly property bool modelSwitching: p("model-switching", false)
  readonly property bool visionDownloading: p("vision12b-downloading", false)
  readonly property bool engineUpdating: p("engine-updating", false)
  readonly property bool builtinBusy: installing || qualityDownloading || modelSwitching || visionDownloading || engineUpdating || p("model-speed-running", false)
  readonly property bool strataBusy: p("strata-busy", false)
  readonly property bool idle: !builtinBusy && !strataBusy
  readonly property bool strataInstalled: p("strata-installed", false)
  readonly property bool strataActive: p("strata-active", false)
  readonly property bool strataAdopted: p("strata-adopted", false)
  readonly property bool strataRunning: p("strata-running", false)
  readonly property var foundModels: p("found-models", [])
  readonly property bool hasOnPc: (llamaInstalled && (legacyPresent || fallbackPresent || qualityPresent))
    || customActive || strataInstalled || foundModels.length > 0
  readonly property bool hasDownload: !llamaInstalled || installing || (!strataInstalled && !p("strata-external", false))
    || !legacyPresent || !fallbackPresent || !qualityPresent
  readonly property bool codexBusy: p("codex-auth-busy", false) || p("codex-models-busy", false)
  readonly property string installStatus: p("local-ai-status", "")
  readonly property string qualityStatus: p("quality-status", "")
  readonly property string visionStatus: p("vision12b-status", "")
  readonly property string engineStatus: p("engine-update-status", "")
  readonly property string strataStatus: p("strata-status", "")
  property string strataFolder: ""
  property var initializedWin: null
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function tr(msgid, args) { return root.service ? root.service.tr(msgid, args || []) : "" }
  function set(name, value) {
    if (root.win && root.service) root.service.client.set(root.win.winId, name, value)
  }
  function call(name, args) {
    if (root.win && root.service) root.service.client.call(root.win.winId, name, args || [])
  }
  function choose(name, value, callback, argument) {
    root.set(name, value)
    root.call(callback, [argument === undefined ? value : argument])
  }
  function relayStatus(status) {
    if (root.win && root.initializedWin === root.win) root.set("local-choice-status", status)
  }
  function refreshWhenIdle() {
    if (root.win && root.initializedWin === root.win && root.idle) root.call("now-refresh-requested")
  }
  // Slint's local derived properties and changed handlers are not wire properties.
  onInstallStatusChanged: Qt.callLater(root.relayStatus, root.installStatus)
  onQualityStatusChanged: Qt.callLater(root.relayStatus, root.qualityStatus)
  onVisionStatusChanged: Qt.callLater(root.relayStatus, root.visionStatus)
  onEngineStatusChanged: Qt.callLater(root.relayStatus, root.engineStatus)
  onStrataStatusChanged: Qt.callLater(root.relayStatus, root.strataStatus)
  onBuiltinBusyChanged: root.refreshWhenIdle()
  onStrataBusyChanged: root.refreshWhenIdle()
  // Do not replay initial snapshot values as edits when the Loader assigns win.
  onWinChanged: {
    root.initializedWin = null
    Qt.callLater(function() { root.initializedWin = root.win })
  }

  // A saved single-line field, with only its adjacent action kept compact.
  component SavedField: C.SettingsFieldRow {
    id: field
    required property string propertyName
    required property string callbackName
    property string placeholder: ""
    property string buttonText: root.tr("Save")
    property bool secret: false
    property bool clearAfterSave: false
    C.SettingsLineEdit {
      Layout.fillWidth: true
      Layout.minimumWidth: 0
      text: root.p(field.propertyName, "")
      placeholderText: field.placeholder
      password: field.secret
      onTextEdited: root.set(field.propertyName, text)
    }
    C.SettingsButton {
      compact: true
      iconName: "check"
      text: field.buttonText
      onClicked: {
        root.call(field.callbackName, [root.p(field.propertyName, "")])
        if (field.clearAfterSave) root.set(field.propertyName, "")
      }
    }
  }

  component Reasoning: C.WrapBlock {
    id: reasoning
    required property string propertyName
    required property string callbackName
    C.SettingsCaption { text: root.tr("Reasoning effort:"); font.pixelSize: Style.font.bodySmall }
    C.SettingsComboBox {
      model: [root.tr("Model default"), root.tr("Off — answer at once"), root.tr("Minimal"), root.tr("Low"), root.tr("Medium"), root.tr("High")]
      currentIndex: root.p(reasoning.propertyName, 0)
      onSelected: function(index) { root.choose(reasoning.propertyName, index, reasoning.callbackName) }
    }
    C.SettingsCaption {
      text: root.tr("Sent with every request. «Off — answer at once» makes thinking models such as Gemini answer without reasoning first; a server that does not know the setting may reject requests — choose «Model default» then.")
    }
  }

  component EngineInstall: C.LocalModelRow {
    service: root.service
    title: root.tr("Built-in engine + Gemma 4 12B")
    detail: root.installing ? root.installStatus : root.tr("llama.cpp · about 7 GB with speech components")
    action: root.tr(root.installing ? "Installing…" : "Install")
    actionEnabled: root.idle
    busy: root.installing
    progressVisible: true
    progress: root.p("local-ai-progress", 0)
    onActionClicked: root.call("install-local-ai-clicked")
    onCancelClicked: root.call("cancel-local-ai-clicked")
  }

  // Identical model rows appear in the provider-independent downloads and the local chooser.
  component BuiltinModels: C.WrapBlock {
    id: models
    property int group: -1 // -1 all, 0 installed, 1 available to download
    Repeater {
      model: 3
      delegate: C.LocalModelRow {
        id: modelRow
        required property int index
        readonly property bool present: index === 0 ? root.legacyPresent : index === 1 ? root.fallbackPresent : root.qualityPresent
        visible: root.llamaInstalled && (models.group === -1 || (models.group === 0 ? present : !present))
        service: root.service
        title: root.tr(index === 0 ? "Fast (4B)" : index === 1 ? "Balanced (12B QAT)" : "Maximum (26B-A4B)")
        detail: busy ? root.qualityStatus : root.tr(index === 0 ? "Gemma 4 E4B · 4.6 GB" : index === 1 ? "Gemma 4 12B · 6.3 GB" : "Gemma 4 26B-A4B · 9.8 GB")
        chip: index === 1 ? root.tr("recommended") : index === 2 && !root.p("quality-selection-allowed", false) ? root.tr("heavy for this PC") : ""
        chipTone: index === 1 ? 1 : 2
        active: root.provider === 1 && root.managed && !root.customActive && root.modelProfile === index && present
        action: root.tr(active ? "In use" : present ? "Use" : "Download model")
        actionEnabled: !active && root.idle
        busy: root.qualityDownloading && root.p("local-download-profile", -1) === index
        progressVisible: true
        progress: root.p("quality-progress", 0)
        onActionClicked: {
          if (!present) root.call("download-model-clicked", [index])
          else if (models.group !== -1 && root.managed) root.call("model-profile-changed", [index])
          else root.call("builtin-model-use", [index])
        }
        onCancelClicked: root.call("cancel-local-ai-clicked")
      }
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.tr("Model storage")
    C.SettingsCaption {
      text: root.tr("Choose the Suflyor model folder on any mounted disk. AI, speech models and voices use it after restarting Suflyor. Existing files are not moved.")
    }
    C.SettingsLineEdit {
      text: root.p("models-directory", "")
      placeholderText: root.tr("Absolute model folder path; empty uses the default locations")
      onTextEdited: root.set("models-directory", text)
    }
    C.SettingsButtonsRow {
      C.SettingsButton {
        text: root.tr("Save")
        enabled: root.idle
        onClicked: root.call("models-directory-save", [root.p("models-directory", "")])
      }
      C.SettingsButton {
        text: root.tr("Choose folder…")
        enabled: root.idle
        onClicked: root.call("models-directory-browse")
      }
    }
    C.StatusLine { text: root.tr(root.p("models-directory-status", "")) }
    C.SettingsCaption {
      text: root.tr("For existing Strata weights, select the parent of Strata-data here. The Strata folder below is for an installed engine, not weights alone.")
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption { text: root.tr("Where answers come from"); font.pixelSize: Style.font.bodySmall }
    C.WrapBlock {
      C.SectionTitle { text: root.tr("Reuse a connection from OpenCode or Pi") }
      C.SettingsCaption { text: root.tr("Imports compatible API profiles from global client settings. Does not run an agent or reuse subscription OAuth. Applying copies the selected API credential into Suflyor's server settings.") }
      C.SettingsButton { text: root.tr("Find connections"); enabled: !root.p("external-connections-busy", false); onClicked: root.call("external-connections-find") }
      C.WrapBlock {
        visible: root.p("external-connections", []).length > 0
        C.SettingsComboBox { model: root.p("external-connections", []); currentIndex: root.p("external-connection-index", 0); onSelected: function(index) { root.set("external-connection-index", index) } }
        C.SettingsButton { text: root.tr("Use connection"); enabled: root.idle && !root.p("external-connections-busy", false); onClicked: root.call("external-connection-use", [root.p("external-connection-index", 0)]) }
      }
      C.StatusLine { text: root.p("external-connections-status", "") }
    }
    C.SettingsComboBox {
      model: [root.tr("Your server or bridge (OpenAI-compatible)"), root.tr("Model on this PC"), root.tr("OpenAI API (Responses)"), root.tr("Anthropic API (Messages)"), root.tr("ChatGPT via Codex (experimental)")]
      currentIndex: root.provider
      onSelected: function(index) { root.choose("ai-provider-index", index, "ai-provider-changed") }
    }
    C.SettingsSeparator {}

    C.WrapBlock {
      visible: root.provider === 0
      C.SectionTitle { text: root.tr("Cloud bridge") }
      C.SettingsCaption { text: root.tr("Bearer token:") + " " + root.p("ai-bearer-status", ""); font.pixelSize: Style.font.bodySmall }
      SavedField {
        propertyName: "ai-bearer-input"
        callbackName: "ai-bearer-save"
        placeholder: root.tr("paste new bearer token then Save")
        secret: true
        clearAfterSave: true
      }
      C.SettingsCaption { text: root.tr("API base URL (OpenAI-compatible):"); font.pixelSize: Style.font.bodySmall }
      SavedField {
        propertyName: "ai-base-url-input"
        callbackName: "ai-base-url-save"
        placeholder: root.tr("http://your.bridge.host:PORT/v1")
      }
      C.SettingsCaption { text: root.tr("Model ID:"); font.pixelSize: Style.font.bodySmall }
      C.SettingsFieldRow {
        C.SettingsComboBox {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          model: root.p("ai-models", [])
          currentIndex: root.p("ai-model-index", 0)
          onSelected: function(index) { root.choose("ai-model-index", index, "ai-model-selected", model[index]) }
        }
        C.SettingsButton {
          compact: true
          iconName: "refresh"
          text: root.tr("refresh")
          onClicked: root.call("ai-models-refresh")
        }
      }
      C.SettingsCaption {
        visible: root.p("ai-models", []).length === 0
        text: root.tr("No models found — check the bridge URL/token, then press refresh.")
      }
      SavedField { propertyName: "ai-model-input"; callbackName: "ai-model-selected"; placeholder: root.tr("Model ID (manual entry is supported)"); buttonText: root.tr("Save model") }
      C.SettingsCaption { text: root.tr("Model discovery is optional. Enter the exact model ID if the server does not list models.") }
      Reasoning { propertyName: "ai-reasoning-index"; callbackName: "ai-reasoning-changed" }
      C.SettingsButton { iconName: "diag"; text: root.tr("Test AI bridge connection"); onClicked: root.call("ai-bridge-test-clicked") }
      C.StatusLine { text: root.p("ai-bridge-test-result", ""); tone: "accent" }
      C.DarkCheck {
        text: root.tr("Prompt-caching (experimental — needs bridge support)")
        checked: root.p("ai-prompt-cache", false)
        onToggled: function(checked) { root.choose("ai-prompt-cache", checked, "ai-prompt-cache-changed") }
      }
      C.SettingsCaption {
        text: root.tr("Marks the system prompt as cacheable for faster repeat asks. If your bridge rejects it (AI stops answering), turn this back off.")
      }
    }

    C.WrapBlock {
      visible: root.provider === 2
      C.SectionTitle { text: root.tr("OpenAI Responses API") }
      C.SettingsCaption { text: root.tr("API key:") + " " + root.p("openai-key-status", ""); font.pixelSize: Style.font.bodySmall }
      SavedField {
        propertyName: "openai-key-input"
        callbackName: "openai-key-save"
        placeholder: root.tr("paste OpenAI API key")
        secret: true
        clearAfterSave: true
      }
      C.SettingsCaption { text: root.tr("Stored in a user-only local credentials file and excluded from configuration exports.") }
      SavedField {
        propertyName: "openai-base-url-input"
        callbackName: "openai-base-url-save"
        placeholder: root.tr("https://api.openai.com/v1")
        buttonText: root.tr("Save URL")
      }
      SavedField {
        propertyName: "openai-model-input"
        callbackName: "openai-model-save"
        placeholder: root.tr("OpenAI model ID")
        buttonText: root.tr("Save model")
      }
      Reasoning { propertyName: "openai-reasoning-index"; callbackName: "openai-reasoning-changed" }
      C.SettingsButton { iconName: "diag"; text: root.tr("Test OpenAI connection"); onClicked: root.call("ai-bridge-test-clicked") }
      C.StatusLine { text: root.p("ai-bridge-test-result", ""); tone: "accent" }
    }

    C.WrapBlock {
      visible: root.provider === 3
      C.SectionTitle { text: root.tr("Anthropic Messages API") }
      C.SettingsCaption { text: root.tr("API key:") + " " + root.p("anthropic-key-status", ""); font.pixelSize: Style.font.bodySmall }
      SavedField {
        propertyName: "anthropic-key-input"
        callbackName: "anthropic-key-save"
        placeholder: root.tr("paste Anthropic API key")
        secret: true
        clearAfterSave: true
      }
      C.SettingsCaption { text: root.tr("Stored in a user-only local credentials file and excluded from configuration exports.") }
      SavedField {
        propertyName: "anthropic-base-url-input"
        callbackName: "anthropic-base-url-save"
        placeholder: root.tr("https://api.anthropic.com/v1")
        buttonText: root.tr("Save URL")
      }
      SavedField {
        propertyName: "anthropic-model-input"
        callbackName: "anthropic-model-save"
        placeholder: root.tr("Anthropic model ID")
        buttonText: root.tr("Save model")
      }
      C.SettingsButton { iconName: "diag"; text: root.tr("Test Anthropic connection"); onClicked: root.call("ai-bridge-test-clicked") }
      C.StatusLine { text: root.p("ai-bridge-test-result", ""); tone: "accent" }
      C.DarkCheck {
        text: root.tr("Prompt caching")
        checked: root.p("ai-prompt-cache", false)
        onToggled: function(checked) { root.choose("ai-prompt-cache", checked, "ai-prompt-cache-changed") }
      }
    }

    C.WrapBlock {
      visible: root.provider === 4
      C.SectionTitle { text: root.tr("Codex safe models (ChatGPT subscription)") }
      C.SettingsCaption { text: root.tr("Uses the official local Codex app-server. Codex owns the ChatGPT login and stores tokens; suflyor never reads them.") }
      C.StatusLine { text: root.p("codex-auth-status", "") }
      C.SettingsButtonsRow {
        C.SettingsButton {
          text: root.tr(root.p("codex-auth-busy", false) ? "Connecting..." : "Connect ChatGPT")
          enabled: !root.codexBusy
          onClicked: root.call("codex-connect-clicked")
        }
        C.SettingsButton { text: root.tr("Disconnect"); onClicked: root.call("codex-disconnect-clicked") }
      }
      C.SettingsCaption { text: root.tr("Account model:"); font.pixelSize: Style.font.bodySmall }
      C.SettingsFieldRow {
        C.SettingsComboBox {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          model: root.p("codex-model-labels", [])
          currentIndex: root.p("codex-model-index", 0)
          enabled: !root.codexBusy
          onSelected: function(index) { root.choose("codex-model-index", index, "codex-model-selected") }
        }
        C.SettingsButton {
          compact: true
          text: root.tr(root.p("codex-models-busy", false) ? "loading..." : "refresh")
          enabled: !root.codexBusy
          onClicked: root.call("codex-models-refresh")
        }
      }
      C.SettingsCaption { text: root.tr("Reasoning effort:"); font.pixelSize: Style.font.bodySmall }
      C.SettingsComboBox {
        model: root.p("codex-reasoning-labels", [])
        currentIndex: root.p("codex-reasoning-index", 0)
        enabled: !root.codexBusy && model.length > 0
        onSelected: function(index) { root.choose("codex-reasoning-index", index, "codex-reasoning-selected") }
      }
      C.SettingsCaption { visible: root.p("codex-models-busy", false); text: root.tr("Loading account models...") }
      C.SettingsCaption {
        visible: !root.p("codex-models-busy", false) && root.p("codex-model-labels", []).length === 0
        text: root.tr("No account models are available. Sign in or refresh.")
      }
      C.StatusLine { text: root.p("codex-rate-status", ""); tone: "muted" }
      C.WrapBlock {
        visible: root.p("codex-login-url", "") !== "" && root.p("codex-user-code", "") !== ""
        C.SettingsCaption {
          text: root.tr("Code:") + " " + root.p("codex-user-code", "")
          color: Commons.Color.accent
          font.pixelSize: Style.font.body
          font.bold: true
        }
        C.SettingsButtonsRow {
          C.SettingsButton {
            text: root.tr("Open sign-in page")
            onClicked: root.call("codex-open-signin-clicked", [root.p("codex-login-url", "")])
          }
          C.SettingsButton {
            text: root.tr("Copy code")
            Accessible.name: root.tr("Copy one-time code")
            enabled: root.p("codex-user-code", "") !== ""
            onClicked: root.call("codex-copy-code-clicked", [root.p("codex-user-code", "")])
          }
        }
      }
      C.StatusLine { text: root.p("codex-copy-status", "") }
      C.StatusLine {
        text: root.tr("Experimental safe mode: the model is pinned; model-command file and network access are denied, and unexpected tools are stopped.")
        tone: "warning"
      }
      C.SettingsCaption { text: root.tr("OpenCode or Qwen belong under Custom OpenAI-compatible bridge only when they expose a compatible inference endpoint.") }
    }

    C.SettingsCard {
      visible: root.provider !== 1
      title: root.tr("Models on this PC")
      C.SettingsCaption {
        text: root.tr("Download models ahead of time. Your current answer provider stays selected until you choose to use a local model.")
      }
      EngineInstall { visible: !root.llamaInstalled || root.installing }
      BuiltinModels {}
      C.StatusLine { visible: root.installStatus !== "" && !root.installing; text: root.installStatus }
      C.StatusLine { text: root.qualityStatus }
      C.LocalModelRow {
        visible: root.llamaInstalled && root.qualityPresent
        service: root.service
        title: root.tr("Install Vision for 26B (~1.1 GiB)")
        detail: root.visionStatus
        action: root.tr(root.visionDownloading ? "Installing Vision…" : root.p("quality-vision-present", false) ? "Installed" : "Download model")
        actionEnabled: !root.p("quality-vision-present", false) && root.idle
        busy: root.visionDownloading
        progressVisible: true
        progress: root.p("quality-progress", 0)
        onActionClicked: root.call("download-vision12b-clicked")
        onCancelClicked: root.call("cancel-local-ai-clicked")
      }
      C.SettingsButton {
        text: root.tr("Use for answers")
        enabled: root.llamaInstalled && (root.legacyPresent || root.fallbackPresent || root.qualityPresent) && root.idle
        onClicked: root.call("builtin-model-use", [root.fallbackPresent ? 1 : root.legacyPresent ? 0 : 2])
      }
    }

    C.WrapBlock {
      visible: root.provider === 1
      C.SettingsCard {
        title: root.tr("Now answering")
        C.SettingsCaption { text: root.p("now-title", ""); color: Commons.Color.foreground; font.pixelSize: Style.font.body; font.bold: true }
        C.SettingsCaption { text: root.p("now-detail", "") }
        Rectangle {
          id: nowPill
          readonly property int nowState: root.strataBusy || (root.builtinBusy && !root.p("model-speed-running", false)) ? 2 : root.p("now-state", 4)
          readonly property color tone: nowState === 1 ? Commons.Color.accent : nowState === 2 || nowState === 3 ? Commons.Color.urgent : Commons.Color.muted
          width: parent.width
          height: pillText.implicitHeight + Style.space(8)
          radius: Style.cornerRadius
          color: Qt.alpha(tone, 0.12)
          C.SettingsCaption {
            id: pillText
            x: Style.space(4)
            y: Style.space(4)
            width: parent.width - Style.space(8)
            text: root.tr(nowPill.nowState === 1 ? "Working" : nowPill.nowState === 2 ? "Starting/Loading" : nowPill.nowState === 3 ? "Not answering" : nowPill.nowState === 4 ? "Not set up" : "Checking…")
            color: nowPill.tone
          }
        }
        C.SettingsButton { text: root.tr("Check"); enabled: root.idle; onClicked: root.call("ai-local-test-clicked") }
        C.StatusLine { text: root.p("ai-local-test-result", "") }
        C.StatusLine { text: root.p("now-notice", ""); tone: "warning" }
      }

      C.SettingsCard {
        title: root.tr("Model speed")
        C.SettingsCaption { text: root.tr("Prefill reads the input; decode generates the answer. A synthetic sample uses the selected model and server timings. No server is started or reconfigured. Up to 128 output tokens; allow up to 3 minutes.") }
        C.SettingsComboBox {
          model: [root.tr("Short input"), root.tr("Medium input"), root.tr("Long input")]
          currentIndex: root.p("model-speed-size", 1)
          enabled: !root.p("model-speed-running", false)
          onSelected: function(index) { root.set("model-speed-size", index) }
        }
        C.SettingsButtonsRow {
          C.SettingsButton { text: root.tr("Measure model speed"); enabled: root.idle; onClicked: root.call("measure-model-speed") }
          C.SettingsButton { visible: root.p("model-speed-running", false); text: root.tr("Cancel"); onClicked: root.call("cancel-model-speed") }
        }
        C.StatusLine { text: root.p("model-speed-status", "") }
        C.SettingsCaption { text: root.p("model-speed-result", ""); color: Commons.Color.foreground }
        C.SettingsCaption { text: root.tr("Last measurement, not a guarantee: input length, cache, GPU load and engine settings change the speed. Missing server timings are shown as unavailable. First-token and total time may include waiting and model loading.") }
      }

      C.SettingsCard {
        title: root.tr("Choose a model")
        C.SettingsCaption { text: root.p("local-hardware-line", "") }
        Repeater {
          model: 2
          delegate: C.WrapBlock {
            id: modelGroup
            required property int index
            C.SectionTitle {
              visible: modelGroup.index === 0 ? root.hasOnPc : root.hasDownload
              text: root.tr(modelGroup.index === 0 ? "On this PC" : "Available to download").toUpperCase()
            }
            EngineInstall { visible: modelGroup.index === 1 && (!root.llamaInstalled || root.installing) }
            BuiltinModels { group: modelGroup.index }
            C.LocalModelRow {
              visible: modelGroup.index === 0 && root.customActive
              service: root.service
              title: root.tr("Your GGUF file")
              detail: root.p("ai-local-custom-model-name", "")
              active: root.managed
              action: root.tr("Choose GGUF…")
              actionEnabled: root.idle
              onActionClicked: root.call("choose-custom-gguf-clicked")
            }
            C.LocalModelRow {
              visible: modelGroup.index === 0 ? root.strataInstalled : !root.strataInstalled && !root.p("strata-external", false)
              service: root.service
              title: root.tr("Qwen3.8-Flash-Next IQ2_XS")
              detail: busy ? root.strataStatus : root.tr(root.strataInstalled ? "Strata engine · installed" : "Strata engine · ~72 GB download")
              chip: !root.p("strata-hardware-fits", true) ? root.tr("needs 48 GB RAM") : ""
              chipTone: 2
              active: root.strataActive && !root.strataAdopted && !root.p("strata-external", false) && root.strataInstalled
              action: root.tr(root.strataBusy ? "Working…" : active && root.strataRunning ? "In use" : active ? "Start" : root.strataInstalled ? "Use" : "Download model")
              actionEnabled: !(active && root.strataRunning) && root.idle
              busy: root.strataBusy && root.p("local-strata-job", -1) === 0
              onActionClicked: {
                if (root.strataInstalled) root.call("strata-choose", [true])
                else root.call("strata-install-clicked")
              }
              onCancelClicked: root.call("cancel-local-ai-clicked")
            }
            C.WrapBlock {
              visible: modelGroup.index === 0
              Repeater {
                model: root.foundModels
                delegate: C.LocalModelRow {
                  id: found
                  required property var modelData
                  required property int index
                  service: root.service
                  title: modelData.title
                  detail: modelData.kind === 1 && busy ? root.strataStatus : modelData.detail
                  active: modelData.kind === 1 ? root.strataActive && root.strataAdopted : modelData.active
                  action: root.tr(modelData.kind === 1 ? (root.strataBusy ? "Working…" : active && root.strataRunning ? "In use" : active ? "Start" : "Use") : (active ? "In use" : "Connect"))
                  actionEnabled: !(modelData.kind === 1 && active && root.strataRunning) && !(modelData.kind === 0 && active) && root.idle
                  busy: modelData.kind === 1 && root.strataBusy && root.p("local-strata-job", -1) === 1
                  onActionClicked: root.call("found-model-use", [found.index])
                  onCancelClicked: root.call("cancel-local-ai-clicked")
                }
              }
            }
          }
        }
        C.SettingsCaption { text: root.tr("Existing Strata Linux installation") }
        C.SettingsLineEdit {
          text: root.strataFolder
          placeholderText: root.tr("Folder containing .venv, serve/server.py and strata-<model>.json")
          onTextEdited: root.strataFolder = text
        }
        C.SettingsButtonsRow {
          C.SettingsButton {
            text: root.tr("Use this Strata folder")
            enabled: root.idle && root.strataFolder.trim() !== ""
            onClicked: root.call("strata-folder-selected", [root.strataFolder])
          }
          C.SettingsButton { text: root.tr("Strata folder…"); enabled: root.idle; onClicked: root.call("strata-find-folder-clicked") }
          C.SettingsButton { visible: root.llamaInstalled; text: root.tr(".gguf file…"); enabled: root.idle; onClicked: root.call("choose-custom-gguf-clicked") }
        }
        C.SettingsButtonsRow {
          C.SettingsButton {
            text: root.tr("Connect by address…")
            enabled: root.idle
            onClicked: {
              root.set("local-connect-auto", false)
              root.set("local-connect-open", !root.p("local-connect-open", false))
            }
          }
          C.SettingsButton {
            visible: !root.p("found-models-searching", false)
            text: root.tr("Search again")
            enabled: root.idle
            onClicked: root.call("found-models-search")
          }
        }
        C.WrapBlock {
          visible: root.p("strata-update-supported", false)
          C.SettingsCaption { text: root.tr("Installed Strata engine: {}", [root.p("strata-engine-version", "")]) }
          C.SettingsCaption { text: root.tr("Strata release and engine versions differ. Checks use the latest official feed; updates replace only the engine. Model weights are kept. A running server restarts.") }
          C.SettingsButtonsRow {
            C.SettingsButton { text: root.tr("Check engine updates"); enabled: root.idle; onClicked: root.call("strata-engine-check") }
            C.SettingsButton { text: root.tr("Update Strata engine"); enabled: root.idle; onClicked: root.call("strata-engine-update") }
            C.SettingsButton { visible: root.p("strata-update-job", 0) !== 0; text: root.tr("Cancel"); onClicked: root.call("cancel-local-ai-clicked") }
          }
          C.StatusLine { text: root.p("strata-update-status", "") }
        }
        C.SettingsCaption { visible: root.p("found-models-searching", false); text: root.tr("Looking for models on this PC…") }
        C.WrapBlock {
          visible: root.p("local-connect-open", false)
          C.SettingsLineEdit { text: root.p("ai-local-model-input", ""); placeholderText: root.tr("Model ID (manual entry is supported)"); onTextEdited: root.set("ai-local-model-input", text) }
          C.SettingsCaption { text: root.tr("Model discovery is optional. Enter the exact model ID if the server does not list models.") }
          C.SettingsLineEdit {
            text: root.p("ai-local-base-url-input", "")
            placeholderText: root.tr("local URL — http://127.0.0.1:11434/v1 (Ollama)")
            onTextEdited: root.set("ai-local-base-url-input", text)
          }
          C.SettingsLineEdit {
            text: root.p("ai-local-bearer-input", "")
            placeholderText: root.tr("local server bearer token (optional)")
            password: true
            onTextEdited: root.set("ai-local-bearer-input", text)
          }
          C.SettingsButton {
            text: root.tr("Clear saved token")
            onClicked: { root.call("ai-local-bearer-save", [""]); root.set("ai-local-bearer-input", "") }
          }
          C.SettingsComboBox {
            model: root.p("ai-local-models", [])
            currentIndex: root.p("ai-local-model-index", 0)
            onSelected: function(index) { root.choose("ai-local-model-index", index, "ai-local-model-selected", model[index]) }
          }
          C.SettingsButton {
            text: root.tr("Connect")
            enabled: root.idle
            onClicked: root.call("ai-local-connect")
          }
        }
        C.StatusLine { text: root.p("local-choice-status", ""); tone: "accent" }
        C.SettingsButton { text: root.tr("Model licence: Qwen. Open the model page"); onClicked: root.call("strata-license-clicked") }
      }

      C.SettingsCard {
        title: root.tr("What the model can do")
        C.DarkCheck {
          text: root.tr("Local model reasoning/thinking (slower, off = faster)")
          checked: root.p("ai-local-thinking", false)
          onToggled: function(checked) { root.choose("ai-local-thinking", checked, "ai-local-thinking-changed") }
        }
        C.DarkCheck {
          text: root.tr("Searches the knowledge base and personal memory")
          checked: root.p("ai-agent-tools", false)
          onToggled: function(checked) { root.choose("ai-agent-tools", checked, "ai-agent-tools-changed") }
        }
        C.DarkCheck {
          x: Style.space(8)
          width: parent.width - x
          text: root.tr("Also search the web (DuckDuckGo)")
          checked: root.p("ai-web-search", false)
          enabled: root.p("ai-agent-tools", false)
          onToggled: function(checked) { root.choose("ai-web-search", checked, "ai-web-search-changed") }
        }
        C.SettingsCaption { text: root.tr("Fast answers can search the web and read public pages when enabled: at most two lookups with a 15-second lookup budget. The model decides when to look up; web queries go to DuckDuckGo. A server without tool support answers without web lookup.") }
        C.DarkCheck {
          visible: root.p("ai-local-vision-available", true)
          text: root.tr("Local model supports images (vision)")
          checked: root.p("ai-local-vision", false)
          onToggled: function(checked) { root.choose("ai-local-vision", checked, "ai-local-vision-changed") }
        }
        C.WrapBlock {
          visible: root.llamaInstalled && root.managed && !root.p("ai-local-vision-available", true)
          C.StatusLine {
            text: root.tr(root.modelProfile !== 2
              ? "Local images need the installed 12B model and its projector. For the cloud or another server, use the Screenshots (F8) tab."
              : root.p("quality-vision-supported", false)
              ? "Vision for 26B requires its matching ~1.1 GiB projector."
              : "Update the local engine before installing Vision for 26B.")
            tone: "warning"
          }
          C.SettingsButton {
            visible: root.modelProfile === 2 && root.p("quality-vision-supported", false)
            text: root.tr(root.visionDownloading ? "Installing Vision…" : "Install Vision for 26B (~1.1 GiB)")
            enabled: !root.builtinBusy
            onClicked: root.call("download-vision12b-clicked")
          }
        }
        C.LocalModelRow {
          visible: root.visionDownloading
          service: root.service
          title: root.tr("Install Vision for 26B (~1.1 GiB)")
          detail: root.visionStatus
          action: root.tr("Installing Vision…")
          actionEnabled: false
          busy: true
          progressVisible: true
          progress: root.p("quality-progress", 0)
          onCancelClicked: root.call("cancel-local-ai-clicked")
        }
      }

      C.SettingsCard {
        visible: root.llamaInstalled && root.managed
        C.SettingsButton {
          text: root.tr(root.p("local-more-open", false) ? "Hide fine tuning" : "Show fine tuning")
          onClicked: root.set("local-more-open", !root.p("local-more-open", false))
        }
        C.WrapBlock {
          visible: root.p("local-more-open", false)
          C.SettingsSeparator {}
          C.SectionTitle { text: root.tr("Context window") }
          C.SettingsSlider {
            minimum: 0.99
            maximum: root.p("ai-local-context-max-k", 32) >= 96 ? 6.01
              : root.p("ai-local-context-max-k", 32) >= 64 ? 5.01
              : root.p("ai-local-context-max-k", 32) >= 32 ? 4.01
              : root.p("ai-local-context-max-k", 32) >= 16 ? 3.01 : 2.01
            value: root.p("ai-local-context-preview-index", 0) + 1
            enabled: !root.builtinBusy
            onChanged: function(value) { root.call("ai-local-context-preview-changed", [value - 1]) }
          }
          C.SettingsCaption {
            text: root.p("ai-local-context-preview-index", 0) === 0
              ? root.tr("Selected: Auto ({}K)", [root.p("ai-local-context-auto-k", 16)])
              : root.tr("Selected: {}K", [root.p("ai-local-context-preview-index", 0) === 1 ? 8
                : root.p("ai-local-context-preview-index", 0) === 2 ? 16
                : root.p("ai-local-context-preview-index", 0) === 3 ? 32
                : root.p("ai-local-context-preview-index", 0) === 4 ? 64 : 96])
          }
          C.SettingsCaption { text: root.p("ai-local-context-vram-hint", "") }
          C.SettingsButton {
            text: root.tr(root.modelSwitching ? "Applying…" : "Apply context")
            enabled: root.p("ai-local-context-preview-index", 0) !== root.p("ai-local-context-index", 0) && !root.builtinBusy
            onClicked: root.call("ai-local-context-changed", [root.p("ai-local-context-preview-index", 0)])
          }
          C.SettingsSeparator {}
          C.SettingsCaption {
            text: root.p("engine-build", "") !== ""
              ? root.tr("Engine llama.cpp {}. The app checks it weekly and verifies a new build before replacing the current one.", [root.p("engine-build", "")])
              : root.tr("Engine llama.cpp. The app checks it weekly and verifies a new build before replacing the current one.")
            font.pixelSize: Style.font.bodySmall
          }
          C.SettingsButtonsRow {
            C.SettingsButton {
              text: root.tr(root.engineUpdating ? "Updating…" : "Update engine")
              enabled: !root.builtinBusy
              onClicked: root.call("update-engine-clicked")
            }
            C.SettingsButton {
              text: root.tr(root.installing ? "Installing…" : "Install / complete local AI")
              enabled: root.idle
              onClicked: root.call("install-local-ai-clicked")
            }
          }
          C.StatusLine {
            text: root.p("local-ai-gpu", "")
            tone: root.p("local-ai-on-gpu", false) ? "good" : "warning"
          }
        }
      }
    }
  }
}
